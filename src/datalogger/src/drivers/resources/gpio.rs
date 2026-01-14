


macro_rules! check_gpio {
    ($self:ident, $gpio:ident, $request:ident) => {
        if $self.$gpio && $request.$gpio {
            return Err(concat!(stringify!($gpio), " already requested"))
        } 
        // else if $request.$gpio {
        //     $self.$gpio = true;
        // }
    }
}

macro_rules! bind_gpio {
    ($self:ident, $gpio:ident, $request:ident) => {
        if $request.$gpio {
            $self.$gpio = true;
        }
    }
}


macro_rules! release_gpio {
    ($self:ident, $gpio:ident, $request:ident) => {
        if $self.$gpio && $request.$gpio {
            $self.$gpio = false;
        }
    }
}


macro_rules! use_pin {
    ($pin:expr, $pin_value:ident, $self:ident, $gpio:ident) => {
        if $pin_value == $pin {
            $self.$gpio = true;        }
    };
}

pub struct GpioRequest {
    gpio1 : bool,
    gpio2 : bool,
    gpio3 : bool,
    gpio4 : bool,
    gpio5 : bool,
    gpio6 : bool,
    gpio7 : bool,
    gpio8 : bool,
    usart : bool,
    usart_count : u8,
    rs485: bool,
}


impl GpioRequest {
    pub fn none() -> GpioRequest {
        GpioRequest { 
            gpio1: false, 
            gpio2: false, 
            gpio3: false, 
            gpio4: false, 
            gpio5: false, 
            gpio6: false, 
            gpio7: false, 
            gpio8: false, 
            usart: false,
            usart_count : 0,
            rs485: false        
        }
    }

    #[allow(unused)]
    pub fn differs(&mut self, request: GpioRequest) -> bool {
        if self.gpio1 != request.gpio1 { return true }
        if self.gpio2 != request.gpio2 { return true }
        if self.gpio3 != request.gpio3 { return true }
        if self.gpio4 != request.gpio4 { return true }
        if self.gpio5 != request.gpio5 { return true }
        if self.gpio6 != request.gpio6 { return true }
        if self.gpio7 != request.gpio7 { return true }
        if self.gpio8 != request.gpio8 { return true }
        if self.usart != request.usart { return true }
        if self.rs485 != request.rs485 { return true }
        return false;
    }

    pub fn update_or_conflict(&mut self, request: GpioRequest) -> Result<(), &'static str>{
        if request.gpio3 || request.gpio4 {
            return Err("pin3 and pin4 are not supported");
        }


        //TODO this has a problem, we can't assign the true value in the check, because it remains if we have a failure
        check_gpio!(self, gpio1, request);
        check_gpio!(self, gpio2, request);
        check_gpio!(self, gpio3, request);
        check_gpio!(self, gpio4, request);

        if self.rs485 {
           if request.gpio5 || request.gpio6 {
                return Err("rs485 in use, conflicts with gpio5 and gpio6");
            } 
        }

        check_gpio!(self, gpio5, request);
        check_gpio!(self, gpio6, request);

        if self.usart {
            if request.gpio7 || request.gpio8 {
                return Err("usart in use, conflicts with gpio7 and gpio8");
            }
        }

        check_gpio!(self, gpio7, request);
        check_gpio!(self, gpio8, request);

        bind_gpio!(self, gpio1, request);
        bind_gpio!(self, gpio2, request);
        bind_gpio!(self, gpio3, request);
        bind_gpio!(self, gpio4, request);
        bind_gpio!(self, gpio5, request);
        bind_gpio!(self, gpio6, request);
        bind_gpio!(self, gpio7, request);
        bind_gpio!(self, gpio8, request);

        // check_gpio!(self, usart, request); // allow multiple users are usart, we would need to have a users count
        if request.usart == true {
            if self.gpio7 || self.gpio8 {
                return Err("gpio7 and gpio8 must both be free to use usart");
            }
            self.usart_count = self.usart_count + 1;
            self.usart = true;
            self.gpio7 = true;
            self.gpio8 = true;
        }

        if request.rs485 == true {
            if self.gpio5 || self.gpio6 {
                return Err("gpio5 and gpio6 must both be free to use usart");
            }
            self.rs485 = true;
            self.gpio5 = true;
            self.gpio6 = true;
        }
        Ok(())
    }

    pub fn gpio1(&self) -> bool {
        return self.gpio1
    }

    pub fn gpio2(&self) -> bool {
        return self.gpio2
    }

    pub fn gpio3(&self) -> bool {
        return self.gpio3
    }

    pub fn gpio4(&self) -> bool {
        return self.gpio4
    }

    pub fn gpio5(&self) -> bool {
        return self.gpio5
    }

    pub fn gpio6(&self) -> bool {
        return self.gpio6
    }

    pub fn gpio7(&self) -> bool {
        return self.gpio7
    }

    pub fn gpio8(&self) -> bool {
        return self.gpio8
    }

    pub fn usart(&self) -> bool {
        return self.usart
    }

    #[allow(unused)]
    pub fn usart_count(&self) -> u8 {
        return self.usart_count
    }


    // TODO: this idiom should be handing the driver some kind of handle it can use to access the pin
    // TODO: since it is ONLY a way to register usage, it means another driver can be mis-implemented
    pub fn use_pin(&mut self, pin: u8){
        use_pin!(1, pin, self, gpio1); 
        use_pin!(2, pin, self, gpio2);
        use_pin!(3, pin, self, gpio3); // these pins are mapped wronly on the hardware
        use_pin!(4, pin, self, gpio4);
        use_pin!(5, pin, self, gpio5);
        use_pin!(6, pin, self, gpio6);
        use_pin!(7, pin, self, gpio7);
        use_pin!(8, pin, self, gpio8);       
    }

    pub fn use_usart(&mut self){
        self.usart = true;
    }

    pub fn use_rs485(&mut self){
        self.rs485 = true;
    }

    pub fn rs485(&self) -> bool{
        return self.rs485;
    }

    pub fn release(&mut self, request: GpioRequest) {
        release_gpio!(self, gpio1, request);
        release_gpio!(self, gpio2, request);
        release_gpio!(self, gpio3, request);
        release_gpio!(self, gpio4, request);
        release_gpio!(self, gpio5, request);
        release_gpio!(self, gpio6, request);
        release_gpio!(self, gpio7, request);
        release_gpio!(self, gpio8, request);
        if request.usart {
            self.usart_count = self.usart_count - 1;
            if self.usart_count == 0 {
                self.usart = false;
                self.gpio7 = false;
                self.gpio8 = false;
            }
        }

        if request.rs485 {
            self.rs485 = false;
            self.gpio5 = false;
            self.gpio6 = false;
        }
    }
}


