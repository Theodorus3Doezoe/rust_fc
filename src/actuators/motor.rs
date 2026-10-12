use super::channel::MotorChannel;

pub struct MotorController<C: MotorChannel> {
    channel: C,
    pub cw: bool,
}
impl<C: MotorChannel> MotorController<C> {
    pub fn new(channel: C, cw: bool) -> Self {
        Self { channel, cw }
    }

    pub fn apply(&mut self, throttle: f32) {
        let _ = self.channel.set_throttle(throttle);
    }
    pub fn latest_telemetry(&mut self) -> Option<C::Telemetry> {
        self.channel.latest_telemetry()
    }
    pub fn command(&mut self, cmd: u8) {
        let _ = self.channel.esc_command(cmd);
    }
}
