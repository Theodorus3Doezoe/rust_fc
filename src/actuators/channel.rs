pub trait MotorChannel {
    type Telemetry;
    type Error;
    fn set_throttle(&mut self, throttle: f32) -> Result<(), Self::Error>;
    fn latest_telemetry(&mut self) -> Option<Self::Telemetry> {
        None
    }
    fn esc_command(&mut self, cmd: u8) -> Result<(), Self::Error> {
        Ok(())
    }
}
