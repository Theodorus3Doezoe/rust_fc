use crate::actuators::servo::ServoController;
use crate::config::SERVO_FREQ_HZ;
use crate::mixers::bi_copter::{self, BiCopMixer, BicopterOutput};
use crate::types::PwmChannels;
use embedded_hal::pwm::SetDutyCycle;

use super::*;

pub struct VCopterFrame<P> {
    servo_left: ServoController<P>,
    servo_right: ServoController<P>,
    mixer: BiCopMixer,
}

#[derive(Clone, Copy, defmt::Format)]
pub struct ActuatorOutput {
    pub servo_left_us: u16,
    pub servo_right_us: u16,
    // pub motor_left: f32, // of u16 als je DShot waarden hebt
    // pub motor_right: f32,
}

pub struct FrameOutput {
    pub mixer: BicopterOutput,
    pub actuators: ActuatorOutput,
}

impl<P> Frame for VCopterFrame<P>
where
    P: SetDutyCycle,
{
    type PwmPin = P;
    type Mixer = self::BiCopMixer;

    fn init(pwm: PwmChannels<P>) -> Self {
        Self {
            servo_left: ServoController::new(pwm.pwm1, 0.0, false, SERVO_FREQ_HZ),
            servo_right: ServoController::new(pwm.pwm2, 0.0, false, SERVO_FREQ_HZ),
            // temp hard coded
            mixer: bi_copter::BiCopMixer::init(5.0, 0.0, 0.0),
        }
    }

    fn apply(&mut self, throttle: f32, pid: Rates) -> FrameOutput {
        // use inner.mixer
        let mixer_out: BicopterOutput = self.mixer.mix(throttle, pid);

        let servo_left_us = self.servo_left.set_duty(mixer_out.servo_left);
        let servo_right_us = self.servo_right.set_duty(mixer_out.servo_right);

        // motors
        //
        FrameOutput {
            mixer: mixer_out,
            actuators: ActuatorOutput {
                servo_left_us,
                servo_right_us,
            },
        }
    }

    fn stop_all(&mut self) {
        let _ = self.servo_left.set_duty(0.0);
        let _ = self.servo_right.set_duty(0.0);

        // disable motors to 0%
    }
}
