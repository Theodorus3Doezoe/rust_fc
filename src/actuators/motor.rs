use crate::actuators::DshotChannel::MotorChannel;

// could be more generic over the channel type if i also want to support motors used with pwm
pub struct MotorController<M> {
    channel: M,
    pub cw: bool,
}

impl<M> MotorController<M>
where
    M: MotorChannel,
{
    // could choose reversed with cw or ccw enum and place that in a config or something in the
    // frame So it doesn't manually have to be set
    pub fn new(channel: M, cw: bool) -> Self {
        Self { channel, cw }
    }

    pub fn apply(&mut self, packet: u16) {
        // this is purely dshot now, should make a check somewhere wether to use dshot or something
        // else
        let _ = self.channel.set_throttle(packet);
    }
}
