use embedded_hal::pwm::SetDutyCycle;

use crate::types::Rates;

pub struct ServoController<P> {
    pwm_channel: P,
    offset: f32,
    inverse: bool,
    period_us: u32,
}

impl<P> ServoController<P>
where
    P: SetDutyCycle,
{
    pub fn new(pwm_channel: P, offset: f32, inverse: bool, servo_freq_hz: u32) -> Self {
        Self {
            pwm_channel,
            offset,
            inverse,
            period_us: 1_000_000 / servo_freq_hz,
        }
    }

    pub fn set_duty(&mut self, mut mixer_input: f32) {
        if self.inverse {
            mixer_input = -mixer_input;
        }

        let mut pulse_us = 1500.0 + (mixer_input + 500.0) + self.offset;
        pulse_us = pulse_us.clamp(1000.0, 2000.0);

        let _ = self
            .pwm_channel
            .set_duty_cycle_fraction(pulse_us as u16, self.period_us as u16);
    }
}
