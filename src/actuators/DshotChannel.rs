pub trait MotorChannel {
    type Error;
    fn set_throttle(&mut self, packet: u16) -> Result<(), Self::Error>;
    // fn arm_motor(&mut self, packet: u16) -> Result<(), Self::Error>;
    // fn set_motor_direction(&mut self, command: u16) -> Result<(), Self::Error>;
}
