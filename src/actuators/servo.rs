use embedded_hal::pwm::SetDutyCycle;

use crate::types::Rates;

pub struct ServoController<P> {
    pwm_channel: P,
    offset: f32,
    inverse: bool,
}

impl<P> ServoController<P>
where
    P: SetDutyCycle,
{
    pub fn new(pwm_channel: P, offset: f32, inverse: bool) -> Self {
        Self {
            pwm_channel,
            offset,
            inverse,
        }
    }

    pub fn set_duty(&mut self, mixer_input: f32) {
        let _ = self.pwm_channel.set_duty_cycle(0);
    }
}
