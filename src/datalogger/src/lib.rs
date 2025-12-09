#![cfg_attr(not(test), no_std)]

mod datalogger;
mod services;

use datalogger::commands::*;
use datalogger::payloads::*;
use datalogger::settings::*;

use rriv_board::{
    RRIVBoard, EEPROM_DATALOGGER_SETTINGS_SIZE, EEPROM_SENSOR_SETTINGS_SIZE,
    EEPROM_TOTAL_SENSOR_SLOTS,
};
extern crate alloc;
use crate::datalogger::bytes;
use crate::datalogger::helper;
use crate::datalogger::modes::DataLoggerMode;
use crate::datalogger::modes::DataLoggerSerialTxMode;
use crate::{protocol::responses, services::*, telemetry::telemeters::lorawan::RakWireless3172};
use alloc::boxed::Box;
use alloc::format;
use rtt_target::rprintln;

mod drivers;
use drivers::{resources::gpio::*, types::*, *};

mod protocol;
mod registry;
mod telemetry;
use registry::*;

use serde_json::{json, Value};

pub struct DataLogger {
    settings: DataloggerSettings,
    sensor_drivers: [Option<Box<dyn SensorDriver>>; rriv_board::EEPROM_TOTAL_SENSOR_SLOTS], // TODO: this could be called 'sensor_configs'. modules, (composable modules)

    last_interactive_log_time: i64,

    assigned_gpios: GpioRequest,

    mode: DataLoggerMode,
    serial_tx_mode: DataLoggerSerialTxMode,
    interactive_logging: bool,

    // naive calibration value book keeping
    // not memory efficient
    calibration_point_values: [Option<Box<[CalibrationPair]>>; EEPROM_TOTAL_SENSOR_SLOTS],

    telemeter: telemetry::telemeters::lorawan::RakWireless3172,

    // measurement cycle
    completed_bursts: u8,
    readings_completed_in_current_burst: u8,
}

const SENSOR_DRIVER_INIT_VALUE: core::option::Option<Box<dyn drivers::types::SensorDriver>> = None;
const CALIBRATION_INIT_VALUE: core::option::Option<Box<[types::CalibrationPair]>> = None;

impl DataLogger {
    pub fn new() -> Self {
        DataLogger {
            settings: DataloggerSettings::new(),
            sensor_drivers: [SENSOR_DRIVER_INIT_VALUE; rriv_board::EEPROM_TOTAL_SENSOR_SLOTS],
            last_interactive_log_time: 0,
            assigned_gpios: GpioRequest::none(),
            mode: DataLoggerMode::Interactive,
            serial_tx_mode: DataLoggerSerialTxMode::Normal,
            calibration_point_values: [CALIBRATION_INIT_VALUE; EEPROM_TOTAL_SENSOR_SLOTS],
            telemeter: RakWireless3172::new(),
            completed_bursts: 0,
            readings_completed_in_current_burst: 0,
            interactive_logging: false,
        }
    }

    fn retrieve_settings(&self, board: &mut impl RRIVBoard) -> DataloggerSettings {
        let mut bytes: [u8; EEPROM_DATALOGGER_SETTINGS_SIZE] =
            [b'\0'; EEPROM_DATALOGGER_SETTINGS_SIZE];
        board.retrieve_datalogger_settings(&mut bytes);
        rprintln!("retrieved {:?}", bytes);
        let settings: DataloggerSettings = DataloggerSettings::new_from_bytes(bytes); // convert the bytes pack into a DataloggerSettings

        let settings = settings.configure_defaults();

        settings
    }

    fn store_settings(&mut self, board: &mut impl RRIVBoard) {
        let bytes = self.settings.get_bytes();
        board.store_datalogger_settings(&bytes);
        rprintln!("stored {:?}", bytes);
    }

    fn get_driver_slot_by_id(&self, id: &str) -> Option<usize> {
        let drivers = &self.sensor_drivers;
        for i in 0..self.sensor_drivers.len() {
            // create json and output it
            if let Some(driver) = &drivers[i] {
                let mut id_bytes = driver.get_id();
                let id_str = util::str_from_utf8(&mut id_bytes).unwrap_or_default();
                if id == id_str {
                    return Some(i);
                }
            }
        }
        return None;
    }

    fn get_driver_index_by_id_value(&self, id: Value) -> Option<usize> {
        let id = match id {
            serde_json::Value::String(ref payload_id) => payload_id,
            _ => {
                return None;
            }
        };

        return self.get_driver_slot_by_id(id);
    }

    pub fn setup(&mut self, board: &mut impl RRIVBoard) {
        // enable power to the eeprom and bring i2bufferc online

        // rprintln!("retrieving settings");
        self.settings = self.retrieve_settings(board);
        // rprintln!("retrieved settings {:?}", self.settings);
        self.mode = DataLoggerMode::from_u8(self.settings.mode);

        // setup each service
        command_service::setup(board);
        usart_service::setup(board);
        modbus_service::setup(board);

        // read all the sensors from EEPROM
        let registry = get_registry();
        let mut sensor_config_bytes = bytes::empty_sensors_settings();
        board.retrieve_sensor_settings(&mut sensor_config_bytes);
        for i in 0..rriv_board::EEPROM_TOTAL_SENSOR_SLOTS {
            let base = i * rriv_board::EEPROM_SENSOR_SETTINGS_SIZE;
            let general_settings_slice =
                &sensor_config_bytes[base..(base + SENSOR_SETTINGS_PARTITION_SIZE)];
            let special_settings_slice: &[u8] = &sensor_config_bytes[(base
                + SENSOR_SETTINGS_PARTITION_SIZE)
                ..(i + 1) * rriv_board::EEPROM_SENSOR_SETTINGS_SIZE];

            let mut general_settings_partition = bytes::empty_sensor_settings_partition();
            general_settings_partition.clone_from_slice(general_settings_slice); // explicitly define the size of the array
            let settings: SensorDriverGeneralConfiguration =
                SensorDriverGeneralConfiguration::new_from_bytes(&general_settings_partition);

            let sensor_type_id = usize::from(settings.sensor_type_id);
            if sensor_type_id > registry.len() {
                continue;
            }
            let create_function = registry[usize::from(settings.sensor_type_id)];
            if let Some(functions) = create_function {
                let mut special_settings_partition = bytes::empty_sensor_settings_partition();
                special_settings_partition.clone_from_slice(special_settings_slice); // explicitly define the size of the arra
                let mut driver = functions.1(settings, &special_settings_partition);

                // check for dedicated resources
                match self
                    .assigned_gpios
                    .update_or_conflict(driver.get_requested_gpios())
                {
                    Ok(_) => {}
                    Err(message) => {
                        // if we have a conflict, what should happen?
                        // definitely don't load the driver, but also this should never happen
                        // should we send something on serial? this is during startup.
                        responses::send_command_response_error(board, message, "");
                        return;
                    }
                };

                driver.setup(board.get_sensor_driver_services());
                self.sensor_drivers[i] = Some(driver);
            }
        }
        rprintln!("done loading sensors");

        let requested_gpios = self.telemeter.get_requested_gpios();
        if self.settings.toggles.enable_telemetry() {
            match self.assigned_gpios.update_or_conflict(requested_gpios) {
                Ok(_) => {}
                Err(_) => {
                    // need a way to tell the telemeter so it can respond at a better time, or some similar strategy
                    responses::send_command_response_error(board, "usart pin conflict", "");
                }
            }
        }

        match self.mode {
            DataLoggerMode::Field => {
                // if we are launching into field mode, write column headers to a new file
                self.write_column_headers_to_storage(board);
            },
            _ => {
                // otherwise we are not logging to storage by default, so don't write any file yet
            }
        }
        rprintln!("done with setup");

        protocol::status::send_ready_status(board);
    }

    pub fn run_loop_iteration(&mut self, board: &mut impl RRIVBoard) {
        //
        // Process incoming commands
        //
        let mut command = command_service::get_pending_command(board); //todo: refactor to use Result<T,E>
        while let Some(get_command_result) = command {
            match get_command_result {
                Ok(command_payload) => {
                    self.execute_command(board, command_payload);
                }
                Err(error) => {
                    responses::send_command_response_error(
                        board,
                        "Error processing command",
                        format!("{:?}\n", error).as_str(),
                    );
                }
            }

            command = command_service::get_pending_command(board);
        }

        //
        //  Process any telemetry setup or QOS
        //
        if self.settings.toggles.enable_telemetry() {
            self.telemeter.run_loop_iteration(board);
        }

        self.update_actuators(board);

        //
        // Do the measurement cycle
        //

        match self.mode {
            DataLoggerMode::Interactive => {
                // process CLI
                // process telemetry
                // process actuators

                if board.timestamp()
                    >= self.last_interactive_log_time
                        + self.settings.interactive_logging_interval as i64
                {
                    // process a single measurement
                    // is this called a 'single measurement cycle' ?

                    self.measure_sensor_values(board); // measureSensorValues(false);
                    self.write_last_measurement_to_serial(board); //outputLastMeasurement();
                                                                  // Serial2.print(F("CMD >> "));
                                                                  // writeRawMeasurementToLogFile();
                                                                  // fileSystemWriteCache->flushCache();
                    if self.interactive_logging {
                        self.write_raw_measurement_to_storage(board);
                    }

                    if self.settings.toggles.enable_telemetry() {
                        self.process_telemetry(board); // telemeterize the measured values
                    }


                    self.last_interactive_log_time = board.timestamp();
                }
            }
            DataLoggerMode::Field => {
                // run measurement cycle
                // maybe we have sleep_interval (minutes) and interactive_logged_interval (seconds)

                self.run_measurement_cycle(board);
                if self.measurement_cycle_completed() {
                    rprintln!("Measurement cycle completed");
                    //     // go to sleep until the next in interval (in minutes)
                    let mut slept = 0u64;
                    while slept < (self.settings.sleep_interval as u64) * 1000u64 * 60u64 {
                        // TODO: allow using interactive_logging_interval here for testing
                        board.delay_ms(2000);
                        board.sleep(); // TODO: this just feeds the watchdog for now
                        slept = slept + 2000;
                    }

                    //     // start the next measurement cycle
                    self.initialize_measurement_cycle();
                }
            }
            DataLoggerMode::HibernateUntil => {
                // this block is processed when we start up, don't get an interactive mode interrupt, and are in HibernateUntil mode
                // or when we have just entered this mode
                // TODO: hibernate until the requested wake time
            }
        }
    }

    fn initialize_measurement_cycle(&mut self) {
        self.completed_bursts = 0;
        self.readings_completed_in_current_burst = 0;
    }

    fn measurement_cycle_completed(&self) -> bool {
        self.completed_bursts >= self.settings.bursts_per_measurement_cycle
    }

    fn run_measurement_cycle(&mut self, board: &mut impl rriv_board::RRIVBoard) {
        // calculate the total number of readings per burst, max of readings requests on each sensor.
        // allow burst length to be set per sensors OR overwridden

        let readings_per_burst: u8 = 10; // TODO get it from settings (need to be added there)

        // get next raw reading
        rprintln!("measuring sensor values in cycle");
        self.measure_sensor_values(board);
        self.readings_completed_in_current_burst = self.readings_completed_in_current_burst + 1;
        rprintln!(
            "completed reading {}",
            self.readings_completed_in_current_burst
        );

        // output raw data to serial?
        let output_raw_data_to_serial = true;
        if output_raw_data_to_serial {
            self.write_last_measurement_to_serial(board)
        }

        // write raw data to storage
        self.write_raw_measurement_to_storage(board);
        // store values into the summarizer

        // check on progress
        if self.readings_completed_in_current_burst >= readings_per_burst {
            rprintln!("completed burst {}", self.completed_bursts);
            self.completed_bursts = self.completed_bursts + 1;
        }
        rprintln!("run_measurement_cycle done");
    }

    fn process_telemetry(&mut self, board: &mut impl rriv_board::RRIVBoard) {

        self.telemeter.process_events(board);

        if !self.telemeter.ready_to_transmit(board) && false { // support both telemeters
            return;
        }

        // TODO: this book-keeping to get the sensor values is not correct / robust / fully functional
        let mut values: [f32; 12] = [f32::MAX; 12];
        // let mut bits: [u8; 12] = [0_u8; 12];
        let mut j = 0; // index of value into the values array
        for i in 0..self.sensor_drivers.len() {
            if let Some(ref mut driver) = self.sensor_drivers[i] {
                // TODO: iterate values
                for k in 0..driver.get_measured_parameter_count() {
                    match driver.get_measured_parameter_value(k) {
                        Ok(value) => {
                            values[j] = value as f32;
                            j = j + 1;
                        }
                        Err(_) => rprintln!("error reading value"),
                    }
                }

                // TODO: returning bits instead of full f64 is a way to use less space in the payload
                //  Bits is a number of bits that should be used when encoding.
                // match driver.get_measured_parameters_bits(0){
                //     Ok(bits) => bits[i] = bits,
                //     Err(_) => todo!(),
                // }
            }
        }

        // let timestamp_hour_offset = 0; // TODO get the timestamp offset from the beginning of the utc hour
        // let bits = [13_u8; 12];

  
        let payload = telemetry::codecs::naive_codec::encode(board.epoch_timestamp(), &values);

        // stateful deltas codec
        // let payload = telemetry::codecs::first_differences_codec::encode(timestamp_hour_offset, values, bits);let p

        if self.telemeter.ready_to_transmit(board) {
            self.telemeter.transmit(board, &payload);
        } 

        let mut values_i16 = [i16::MAX; telemetry::telemeters::rs485::MAX_DATA_VALUES];
        for i in 0..telemetry::telemeters::rs485::MAX_DATA_VALUES {
            values_i16[i] = (values[i] * 10.0) as i16;
        }

        telemetry::telemeters::rs485::send_input_registers_response(board, values_i16);

    }

    fn measure_sensor_values(&mut self, board: &mut impl rriv_board::RRIVBoard) {
        for i in 0..self.sensor_drivers.len() {
            if let Some(ref mut driver) = self.sensor_drivers[i] {
                driver.take_measurement(board.get_sensor_driver_services());
            }
        }
    }

    fn update_actuators(&mut self, board: &mut impl rriv_board::RRIVBoard) {
        for i in 0..self.sensor_drivers.len() {
            if let Some(ref mut driver) = self.sensor_drivers[i] {
                driver.update_actuators(board.get_sensor_driver_services());
            }
        }
    }

    fn write_column_headers_to_serial(&mut self, board: &mut impl rriv_board::RRIVBoard) {
        board.usb_serial_send("timestamp,");

        let mut first = true;
        for i in 0..self.sensor_drivers.len() {
            if let Some(ref mut driver) = self.sensor_drivers[i] {
                if first {
                    first = false;
                } else {
                    board.usb_serial_send(",");
                }

                let prefix = helper::get_prefix(&mut driver.get_id());

                for j in 0..driver.get_measured_parameter_count() {
                    let mut identifier = driver.get_measured_parameter_identifier(j);
                    let identifier_str = util::str_from_utf8(&mut identifier).unwrap_or_default();
                    board.usb_serial_send(&prefix);
                    board.usb_serial_send(identifier_str);
                    if j != driver.get_measured_parameter_count() - 1 {
                        board.usb_serial_send(",");
                    }
                }
            }
        }
        board.usb_serial_send("\n");
    }

    fn write_column_headers_to_storage(&mut self, board: &mut impl rriv_board::RRIVBoard) {
        board.write_log_file("type,site,logger,deployment,deployed_at,uid,time.s,battery.V,");

        let mut first = true;
        for i in 0..self.sensor_drivers.len() {
            if let Some(ref mut driver) = self.sensor_drivers[i] {
                if first {
                    first = false;
                } else {
                    board.write_log_file(",");
                }

                let prefix = helper::get_prefix(&mut driver.get_id());

                for j in 0..driver.get_measured_parameter_count() {
                    let mut identifier = driver.get_measured_parameter_identifier(j);
                    let identifier_str = util::str_from_utf8(&mut identifier).unwrap_or_default();
                    board.write_log_file(&prefix);
                    board.write_log_file(identifier_str);
                    if j != driver.get_measured_parameter_count() - 1 {
                        board.write_log_file(",");
                    }
                }
            }
        }
        board.write_log_file("\n");
    }

    // TODO: this function and the next one can be DRY by passing a closure
    fn write_measured_parameters_to_serial(&mut self, board: &mut impl rriv_board::RRIVBoard) {
        let epoch = board.epoch_timestamp();
        let millis = board.get_millis() % 1000;
        let output = format!("{}.{},", epoch, millis);
        board.usb_serial_send(&output);

        let mut first = true;
        for i in 0..self.sensor_drivers.len() {
            if let Some(ref mut driver) = self.sensor_drivers[i] {
                if first {
                    first = false;
                } else {
                    board.usb_serial_send(",");
                }

                for j in 0..driver.get_measured_parameter_count() {
                    match driver.get_measured_parameter_value(j) {
                        Ok(value) => {
                            let output = format!("{:.4}", value);
                            rprintln!("{}", value);
                            board.usb_serial_send(&output);
                        }
                        Err(_) => {
                            rprintln!("{}", "Error");
                            board.usb_serial_send("Error");
                        }
                    }

                    if j != driver.get_measured_parameter_count() - 1 {
                        board.usb_serial_send(",");
                    }
                }
            }
        }
        board.usb_serial_send("\n");
    }

    fn write_raw_measurement_to_storage(&mut self, board: &mut impl rriv_board::RRIVBoard) {
        let epoch = board.epoch_timestamp();
        let millis = board.get_millis() % 1000;
        // "type,site,logger,deployment,deployed_at,uid,time.s,battery.V"

        // TODO: find a better way to print this uid, or generate and use a UUID that doesn't come from the MCU's uid
        let uid = board.get_uid();
        let output = format!(
            "raw,{},{},{},-,{:X?}{:X?}{:X?}{:X?}{:X?}{:X?}{:X?}{:X?}{:X?}{:X?}{:X?}{:X?},{}.{},{},",
            util::str_from_utf8(&mut self.settings.site_name).unwrap_or_default(),
            util::str_from_utf8(&mut self.settings.logger_name).unwrap_or_default(),
            util::str_from_utf8(&mut self.settings.deployment_identifier).unwrap_or_default(),
            uid[0],
            uid[1],
            uid[2],
            uid[3],
            uid[4],
            uid[5],
            uid[6],
            uid[7],
            uid[8],
            uid[9],
            uid[10],
            uid[11],
            epoch,
            millis,
            board.get_battery_level()
        );
        board.write_log_file(&output);

        let mut first = true;
        for i in 0..self.sensor_drivers.len() {
            if let Some(ref mut driver) = self.sensor_drivers[i] {
                if first {
                    first = false;
                } else {
                    board.write_log_file(",");
                }

                for j in 0..driver.get_measured_parameter_count() {
                    match driver.get_measured_parameter_value(j) {
                        Ok(value) => {
                            let output = format!("{:.4}", value);
                            rprintln!("{}", value);
                            board.write_log_file(&output);
                        }
                        Err(_) => {
                            rprintln!("{}", "Error");
                            board.write_log_file("Error");
                        }
                    }

                    if j != driver.get_measured_parameter_count() - 1 {
                        board.write_log_file(",");
                    }
                }
            }
        }
        board.write_log_file("\n");
    }

    fn write_last_measurement_to_serial(&mut self, board: &mut impl rriv_board::RRIVBoard) {
        // first output the column headers
        match self.serial_tx_mode {
            DataLoggerSerialTxMode::Interactive => self.write_column_headers_to_serial(board),
            _ => {}
        }

        // then output the last measurement values
        match self.serial_tx_mode {
            DataLoggerSerialTxMode::Watch => self.write_measured_parameters_to_serial(board),
            _ => {}
        }
    }

    pub fn set_mode(&mut self, board: &mut impl RRIVBoard, mode: Value) {
        match mode {
            Value::String(mode) => {
                let mode = mode.as_str();
                // TODO: perhaps watch should be separate from mode, so you can watch any mode
                // TODO: and rrivctl can still accept commands when in watch mode
                match mode {
                    "watch" => {
                        self.write_column_headers_to_serial(board);
                        self.serial_tx_mode = DataLoggerSerialTxMode::Watch;
                        board.set_debug(false);
                        self.telemeter.set_watch(true);
                    }
                    "watch-debug" => {
                        self.write_column_headers_to_serial(board);
                        self.serial_tx_mode = DataLoggerSerialTxMode::Watch;
                        board.set_debug(true);
                        self.telemeter.set_watch(true);
                    }
                    "quiet" => {
                        self.serial_tx_mode = DataLoggerSerialTxMode::Quiet;
                        board.set_debug(false);
                        self.telemeter.set_watch(false);
                    }
                    "field" => {
                        self.mode = DataLoggerMode::Field;
                        // switching into field mode should create a new file
                        self.write_column_headers_to_storage(board);
                    }
                    _ => {
                        self.mode = DataLoggerMode::Interactive;
                        board.set_debug(true);
                        self.telemeter.set_watch(false);
                    }
                }
            }
            _ => {}
        }
        self.settings.mode = self.mode.to_u8();
        self.store_settings(board);
    }

    pub fn execute_command(&mut self, board: &mut impl RRIVBoard, command_payload: CommandPayload) {
        // rprintln!("executing command {:?}", command_payload);
        match command_payload {
            CommandPayload::DataloggerSet(payload) => {
                match self.update_datalogger_settings(board, payload) {
                    Ok(_) => responses::send_json(board, self.datalogger_settings_payload()),
                    Err(message) => responses::send_command_response_error(board, message, ""),
                }
            }
            CommandPayload::DataloggerGet(_) => {
                responses::send_json(board, self.datalogger_settings_payload());
            }
            CommandPayload::DataloggerSetModeCommandPayload(payload) => {
                // Deprecated, replaced by DataloggerSet
                // TODO: this command is deprecated
                if let Some(mode) = payload.mode {
                    self.set_mode(board, mode);
                }

                responses::send_json(board, self.datalogger_settings_payload());
            }
            CommandPayload::SensorSet(payload, raw_values) => {
                // convert to values
                let mut payload_values = match payload.convert() {
                    Ok(values) => values,
                    Err(message) => {
                        responses::send_command_response_error(board, message, "");
                        return;
                    }
                };

                // make sure the id is unique
                if payload_values.sensor_id == None {
                    let sensor_id: [u8; 6] = [b'0'; 6]; // base default value
                    payload_values.sensor_id =
                        Some(make_unique_sensor_id(&mut self.sensor_drivers, sensor_id));
                }

                // build the driver
                let mut driver =
                    match datalogger::commands::build_driver(&payload_values, raw_values) {
                        Ok(driver) => driver,
                        Err(message) => {
                            responses::send_command_response_error(board, message, "");
                            return;
                        }
                    };

                let mut slot = None;
                if let Some(sensor_id) = &mut payload_values.sensor_id {
                    match util::str_from_utf8(sensor_id) {
                        Ok(sensor_id) => slot = self.get_driver_slot_by_id(sensor_id),
                        Err(_) => {}
                    }
                }

                if slot.is_some() {
                    if let Some(existing_driver) = &self.sensor_drivers[slot.unwrap()] {
                        // release bound resources so they can be checked and rebound or changed in next step
                        self.assigned_gpios
                            .release(existing_driver.get_requested_gpios());
                    }
                }

                // check for dedicated resources if this is a new sensor
                match self
                    .assigned_gpios
                    .update_or_conflict(driver.get_requested_gpios())
                {
                    Ok(_) => {}
                    Err(message) => {
                        responses::send_command_response_error(board, message, "");
                        return;
                    }
                };

                driver.setup(board.get_sensor_driver_services());

                if slot.is_none() {
                    slot = find_empty_slot(&mut self.sensor_drivers);
                }
                let slot = slot.unwrap();

                let mut configuration_bytes: [u8; EEPROM_SENSOR_SETTINGS_SIZE] =
                    [0; EEPROM_SENSOR_SETTINGS_SIZE];
                driver.get_configuration_bytes(&mut configuration_bytes);
                board.store_sensor_settings(slot as u8, &configuration_bytes);
                self.sensor_drivers[slot] = Some(driver);

                if let Some(driver) = &mut self.sensor_drivers[slot] {
                    responses::send_json(board, driver.get_configuration_json());
                    return;
                } else {
                    responses::send_command_response_error(board, "sensor not configured", "");
                }
            }
            CommandPayload::SensorGet(payload) => {
                if let Some(index) = self.get_driver_index_by_id_value(payload.id) {
                    if let Some(driver) = &mut self.sensor_drivers[index] {
                        responses::send_json(board, driver.get_configuration_json());
                        return;
                    }
                }

                responses::send_command_response_message(board, "didn't find the sensor");
                // TODO: send 404
            }
            CommandPayload::SensorRemove(payload) => {
                let mut values = match payload.convert() {
                    Ok(values) => values,
                    Err(message) => {
                        responses::send_command_response_error(board, message, "");
                        return;
                    }
                };

                let id = util::str_from_utf8(&mut values.id).unwrap_or_default();
                let slot = self.get_driver_slot_by_id(id);
                if slot.is_none() {
                    responses::send_command_response_error(board, "sensor not found", "");
                    return;
                }
                let slot = slot.unwrap();

                let driver = &self.sensor_drivers[slot];
                if let Some(driver) = driver {
                    self.assigned_gpios.release(driver.get_requested_gpios());
                } else {
                    responses::send_command_response_error(board, "sensor not found", "");
                    return;
                }

                let bytes = bytes::empty_sensor_settings();
                board.store_sensor_settings(slot as u8, &bytes);
                self.sensor_drivers[slot] = None;
                responses::send_command_response_message(board, "sensor removed");
            }
            CommandPayload::SensorList(_) => {
                datalogger::commands::list_sensors(board, &mut self.sensor_drivers);
            }
            CommandPayload::BoardRtcSet(payload) => {
                match payload.epoch {
                    serde_json::Value::Number(epoch) => {
                        let epoch = epoch.as_i64();
                        if let Some(epoch) = epoch {
                            board.set_epoch(epoch);
                            responses::send_command_response_message(board, "Epoch set");
                        } else {
                            responses::send_command_response_message(board, "Bad epoch in command");
                        }
                    }
                    err => {
                        responses::send_command_response_error(
                            board,
                            "Bad epoch in command",
                            format!("{:?}", err).as_str(),
                        );
                        return;
                    }
                };
            }
            CommandPayload::BoardGet(payload) => {
                datalogger::commands::get_board(board, payload);
            }
            CommandPayload::SensorCalibratePoint(payload) => {
                let args =
                    match datalogger::commands::sensor_add_calibration_point_arguments(&payload) {
                        Ok(args) => args,
                        Err(message) => {
                            responses::send_command_response_message(board, message);
                            return;
                        }
                    };

                if let Some(index) = self.get_driver_slot_by_id(args.0) {
                    if let Some(driver) = &mut self.sensor_drivers[index] {
                        // read sensor values
                        driver.take_measurement(board.get_sensor_driver_services());

                        let count = driver.get_measured_parameter_count() / 2; // TODO: get_measured_parameter_count, vs get_output_parameter_count
                        let mut values = Box::new([0_f64; 10]); // TODO: max of 10, should we make this dynamic?
                        for j in 0..count {
                            rprintln!("{:?}", j);
                            let value = match driver.get_measured_parameter_value(j * 2) {
                                Ok(value) => value,
                                Err(_) => {
                                    // board.usb_serial_send("missing parameter value\n");
                                    0_f64
                                }
                            };
                            values[j] = value;
                        }

                        // TODO: datalogger can own a calibration object that tracks the calibrations and does the fit

                        // store the point
                        // TODO: we are only storing one point for now
                        let calibration_pair = CalibrationPair {
                            point: args.1,
                            values: values,
                        };
                        if self.calibration_point_values[index].is_some() {
                            let pairs: &Option<Box<[CalibrationPair]>> =
                                &self.calibration_point_values[index];
                            if let Some(pairs) = pairs {
                                let arr = [calibration_pair, pairs[0].clone()];
                                let arr_box = Box::new(arr);
                                self.calibration_point_values[index] = Some(arr_box);
                            }
                        } else {
                            let arr = [calibration_pair];
                            let arr_box = Box::new(arr);
                            self.calibration_point_values[index] = Some(arr_box);
                        }

                        // Success, so return the current calibration point list
                        responses::calibration_point_list(
                            board,
                            &self.calibration_point_values[index],
                        ); // TODO: is responses the right place for marshaling JSON lists to serial?
                    }
                } else {
                    responses::send_command_response_message(board, "Didn't find the sensor");
                }
            }
            CommandPayload::SensorCalibrateList(payload) => {
                let payload_values = match payload.convert() {
                    Ok(payload_values) => payload_values,
                    Err(message) => {
                        responses::send_command_response_message(
                            board,
                            format!("{}", message).as_str(),
                        );
                        return;
                    }
                };

                // list the values
                if let Some(index) = self.get_driver_slot_by_id(payload_values.id) {
                    rprintln!("driver index{}", index);
                    let pairs: &Option<Box<[CalibrationPair]>> =
                        &self.calibration_point_values[index];

                    responses::calibration_point_list(board, pairs); // TODO: is responses the right place for marshaling JSON lists to serial?
                }
            }
            CommandPayload::SensorCalibrateRemove(payload) => {
                responses::send_command_response_message(
                    board,
                    "This command is not implemented yet",
                );
                return;

                // TO DO: implement removal by tag

                // let payload_values = match payload.convert() {
                //     Ok(payload_values) => payload_values,
                //     Err(message) => {
                //         board.usb_serial_send(format!("{}", message).as_str());
                //         return;
                //     }
                // };

                // if let Some(index) = self.get_driver_index_by_id(payload_values.id) {
                //     let pairs: &Option<Box<[CalibrationPair]>> =
                //         &self.calibration_point_values[index];
                //     if let Some(pairs) = pairs {
                //         for i in 0..pairs.len() {
                //             let pair: CalibrationPair = pairs[i];
                //         }
                //     }
                // }
            }
            CommandPayload::SensorCalibrateFit(payload) => {
                let payload_values = match payload.convert() {
                    Ok(payload_values) => payload_values,
                    Err(message) => {
                        responses::send_command_response_message(
                            board,
                            format!("{}", message).as_str(),
                        );
                        return;
                    }
                };

                if let Some(index) = self.get_driver_slot_by_id(payload_values.id) {
                    if let Some(driver) = &mut self.sensor_drivers[index] {
                        let pairs: &Option<Box<[CalibrationPair]>> =
                            &self.calibration_point_values[index];
                        // if let Some(pairs) = pairs {
                        //     for i in 0..pairs.len() {
                        //         let pair = &pairs[i];
                        //         rprintln!("calib pair{:?} {} {}", i, pair.point, pair.values[i]);
                        //     }
                        // }

                        if let Some(pairs) = pairs {
                            for i in 0..pairs.len() {
                                let pair = &pairs[i];
                                rprintln!("calib pair{:?} {} {}", i, pair.point, pair.values[0]);
                            }

                            driver.clear_calibration();
                            match driver.fit(pairs) {
                                Ok(_) => {
                                    let mut storage = bytes::empty_sensor_settings();
                                    driver.get_configuration_bytes(&mut storage);

                                    board.store_sensor_settings(index as u8, &storage);
                                    responses::send_command_response_message(board, "Fit OK");
                                }
                                Err(_) => {
                                    responses::send_command_response_message(
                                        board,
                                        "something went wrong",
                                    );
                                }
                            }
                        }
                    }
                }
            }
            CommandPayload::SensorCalibrateClear(payload) => {
                let payload_values = match payload.convert() {
                    Ok(payload_values) => payload_values,
                    Err(message) => {
                        board.usb_serial_send(format!("{}", message).as_str());
                        return;
                    }
                };

                if let Some(index) = self.get_driver_slot_by_id(payload_values.id) {
                    if let Some(driver) = &mut self.sensor_drivers[index] {
                        driver.clear_calibration();
                        responses::send_command_response_message(board, "Cleared payload");
                    }
                }
            }
            CommandPayload::BoardSerialSend(payload) => {
                let payload_values = match payload.convert() {
                    Ok(payload_values) => payload_values,
                    Err(error) => {
                        responses::send_command_response_error(
                            board,
                            "Problem with message",
                            error,
                        );
                        return;
                    }
                };

                let message = match core::str::from_utf8(
                    &payload_values.message[0..payload_values.message_len as usize],
                ) {
                    Ok(message) => message,
                    Err(error) => {
                        responses::send_command_response_error(
                            board,
                            "Problem sending message",
                            format!(" {} \n", error).as_str(),
                        );
                        return;
                    }
                };

                let prepared_message = format!("{}\r\n", message);
                let prepared_message = prepared_message.as_str();
                rprintln!("message {}", prepared_message);
                board.usart_send(prepared_message);
                // rprintln!("{}", "\r\n");
                // board.usart_send("\r\n");
                // rprintln!("just line feed");
                // board.usart_send("\r");

                board.delay_ms(500);

                let response = match usart_service::take_command(board) {
                    Ok(message) => message,
                    Err(_) => {
                        rprintln!("no usart response");
                        responses::send_command_response_message(
                            board,
                            "No response received on serial",
                        );
                        return;
                    }
                };

                let length = response.len();
                for b in &response[0..length] {
                    let c = *b as char;
                    rprintln!("{}", c);
                }

                match core::str::from_utf8(&response) {
                    Ok(response) => {
                        let end = response.find("\0");
                        let mut response = response;
                        match end {
                            Some(end) => response = &response[0..end],
                            None => {}
                        }
                        responses::send_command_response_message(board, response);
                    }
                    Err(error) => {
                        responses::send_command_response_error(
                            board,
                            "Problem receiving message",
                            format!(" {} \n", error).as_str(),
                        );
                    }
                };

                return;
            }
            CommandPayload::TelemeterGet => match self.telemeter.get_identity(board) {
                Ok(message) => {
                    board.usb_serial_send(format!("{}\n", message.as_str()).as_str());
                }
                Err(_) => {
                    responses::send_command_response_message(board, "Failed to get identifiers");
                }
            },
            CommandPayload::DeviceSetSerialNumber(device_set_serial_number_payload) => {
                match device_set_serial_number_payload.convert() {
                    Ok(values) => {
                        if board.set_serial_number(values.serial_number) {
                            self.device_get(board);
                        } else {
                            responses::send_command_response_error(
                                board,
                                "Serial number already set",
                                "",
                            );
                        }
                    }
                    Err(err) => {
                        responses::send_command_response_error(board, "error", err);
                    }
                }
            }
            
            #[allow(unused)] // the payload doesn't contain anything useful
            CommandPayload::DeviceGet(device_get_payload) => {
                self.device_get(board);
            }
        }
    }

    pub fn update_datalogger_settings(
        &mut self,
        board: &mut impl RRIVBoard,
        set_command_payload: DataloggerSetPayload,
    ) -> Result<(), &'static str> {
        let mode = set_command_payload.mode.clone(); // TODO: clean this up
        let values = set_command_payload.values();
        if let Some(enable_telemetry) = &values.enable_telemetry {
            if !self.settings.toggles.enable_telemetry() && *enable_telemetry {
                match self
                    .assigned_gpios
                    .update_or_conflict(self.telemeter.get_requested_gpios())
                {
                    Ok(_) => {}
                    Err(message) => return Err(message),
                }
            } else if self.settings.toggles.enable_telemetry() && !*enable_telemetry {
                self.assigned_gpios
                    .release(self.telemeter.get_requested_gpios());
            }
        }

        let new_settings = self.settings.with_values(values);
        self.settings = new_settings;
        if let Some(mode) = mode {
            self.set_mode(board, mode);
        }
        self.store_settings(board);
        Ok(())
    }

    fn datalogger_settings_payload(&mut self) -> Value {
        json!({
           "site_name": util::str_from_utf8(&mut self.settings.site_name).unwrap_or_default(),
           "logger_name" : util::str_from_utf8(&mut self.settings.logger_name).unwrap_or_default(),
           "deployment_identifier" : util::str_from_utf8(&mut self.settings.deployment_identifier).unwrap_or_default(),
           "deployment_timestamp" : self.settings.deployment_timestamp,
           "interactive_logging_interval" : self.settings.interactive_logging_interval,
           "sleep_interval" : self.settings.sleep_interval,
           "start_up_delay" : self.settings.start_up_delay,
           "delay_between_bursts" : self.settings.delay_between_bursts,
           "bursts_per_measurement_cycle" : self.settings.bursts_per_measurement_cycle,
           "mode" : datalogger::modes::mode_text(&self.mode),
           "enable_telemetry" : self.settings.toggles.enable_telemetry()
        })
    }

    fn device_get(&self, board: &mut impl RRIVBoard) {
        let serial_number = board.get_serial_number();
        let uid = board.get_uid();
        // let gpio_assignments = &self.assigned_gpios;
        let mut assignments: [[u8; 6]; 9] = [[b'\0'; 6]; 9];
        for i in 0..self.sensor_drivers.len() {
            if let Some(driver) = &self.sensor_drivers[i] {
                let gpios = driver.get_requested_gpios();
                if gpios.gpio1() {
                    assignments[0] = driver.get_id()
                }
                if gpios.gpio2() {
                    assignments[1] = driver.get_id()
                }
                if gpios.gpio3() {
                    assignments[2] = driver.get_id()
                }
                if gpios.gpio4() {
                    assignments[3] = driver.get_id()
                }
                if gpios.gpio5() {
                    assignments[4] = driver.get_id()
                }
                if gpios.gpio6() {
                    assignments[5] = driver.get_id()
                }
                if gpios.gpio7() {
                    assignments[6] = driver.get_id()
                }
                if gpios.gpio8() {
                    assignments[7] = driver.get_id()
                }
                if gpios.usart() {
                    assignments[8] = driver.get_id()
                }
            }
        }
        if self.settings.toggles.enable_telemetry() {
            let id = b"lorawn";
            assignments[6].clone_from_slice(id);
            assignments[7].clone_from_slice(id);
            assignments[8].clone_from_slice(id);
        }
        responses::device_get(board, serial_number, uid, assignments);
    }
}
