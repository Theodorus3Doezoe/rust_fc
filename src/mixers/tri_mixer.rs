use crate::mixers::{desaturate_motors, desaturate_servos};
use crate::{mixers::Mixer, types::Rates};

pub struct TriMixer {}

pub struct TriOutput {
    pub motor_front_left: f32,
    pub motor_front_right: f32,
    pub motor_tail: f32,
    pub servo_tail: f32,
}

impl Mixer for TriMixer {
    type Output = TriOutput;

    fn mix(&self, throttle: f32, pid: Rates) -> Self::Output {
        let motor_front_left = throttle + pid.roll - pid.pitch;
        let motor_front_right = throttle - pid.roll - pid.pitch;
        let motor_tail = throttle + pid.pitch;

        let [motor_front_left, motor_front_right, motor_tail] =
            desaturate_motors([motor_front_left, motor_front_right, motor_tail]);

        let servo_tail = pid.yaw;
        let [servo_tail] = desaturate_servos([servo_tail]);

        TriOutput {
            motor_front_left,
            motor_front_right,
            motor_tail,
            servo_tail,
        }
    }
}

impl TriMixer {
    pub fn init() -> Self {
        Self {}
    }
}
