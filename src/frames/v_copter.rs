use crate::actuators::servo::ServoController;
use crate::mixers::bi_copter::{self, BiCopMixer, BicopterOutput};
use crate::types::PwmChannels;
use embedded_hal::pwm::SetDutyCycle;

use super::*;

pub struct VCopterFrame<P> {
    servo_left: ServoController<P>,
    servo_right: ServoController<P>,
    mixer: BiCopMixer,
}

impl<P> Frame for VCopterFrame<P>
where
    P: SetDutyCycle,
{
    type PwmPin = P;
    type Mixer = self::BiCopMixer;

    fn init(mixer: self::BiCopMixer, pwm: PwmChannels<P>) -> Self {
        Self {
            servo_left: ServoController::new(pwm.pwm1, 0.0, false),
            servo_right: ServoController::new(pwm.pwm2, 0.0, false),
            mixer,
        }
    }

    fn apply(&mut self, throttle: f32, pid: Rates, is_armed: bool) {
        // use inner.mixer
        let output: BicopterOutput = self.mixer.mix(throttle, pid, is_armed);

        self.servo_left.set_duty(output.servo_left);
        self.servo_right.set_duty(output.servo_right);

        // motors
    }
}
