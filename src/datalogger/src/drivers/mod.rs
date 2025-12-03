

// codes and sensor names mapped to a sensor implementation

pub mod types;

pub mod mcp9808;
pub mod generic_analog;
pub mod ring_temperature;
pub mod timed_switch_2;
#[cfg(not(feature = "N2O"))]
pub mod ds18b20;
#[cfg(not(feature = "N2O"))]
pub mod k30_co2;
#[cfg(not(feature = "N2O"))]
pub mod atlas_ec;
#[cfg(not(feature = "N2O"))]
pub mod aht20;
#[cfg(feature = "N2O")]
pub mod basic_evo;

pub mod resources;

