use crate::mixers::{desaturate_motors, desaturate_servos};
use crate::{config::MOTOR_IDLE, mixers::Mixer, types::Rates};

pub struct BiCopMixer {}

pub struct BicopterOutput {
    pub servo_left: f32, // -1.0 .. 1.0 rads
    pub servo_right: f32,
    pub motor_left: f32, // 0.0 .. 1.0
    pub motor_right: f32,
}

impl Mixer for BiCopMixer {
    type Output = BicopterOutput;
    fn mix(&self, throttle: f32, pid: Rates) -> Self::Output {
        let servo_left = pid.pitch + pid.yaw;
        let servo_right = pid.pitch - pid.yaw;
        let [servo_left, servo_right] = desaturate_servos([servo_left, servo_right]);

        let motor_left = throttle + pid.roll;
        let motor_right = throttle - pid.roll;
        let [motor_left, motor_right] = desaturate_motors([motor_left, motor_right]);

        BicopterOutput {
            servo_left,
            servo_right,
            motor_left,
            motor_right,
        }
    }
}

impl BiCopMixer {
    pub const fn init(servo_left_offset: f32, servo_right_offset: f32) -> Self {
        Self {}
    }

    // calibrate function?

    pub fn mix(&self, throttle: f32, pid: Rates) -> BicopterOutput {
        //servos
        let mut servo_left = pid.pitch + pid.yaw;
        let mut servo_right = pid.pitch - pid.yaw;

        // servo desaturation
        let peak = servo_left.abs().max(servo_right.abs());
        if peak > 1.0 {
            servo_left /= peak;
            servo_right /= peak;
        }

        servo_left = servo_left.clamp(-1.0, 1.0);
        servo_right = servo_right.clamp(-1.0, 1.0);

        // motors
        let mut motor_left = throttle + pid.roll;
        let mut motor_right = throttle - pid.roll;

        // motor desaturation
        let motor_max = motor_left.max(motor_right);
        if motor_max > 1.0 {
            motor_left -= motor_max - 1.0;
            motor_right -= motor_max - 1.0;
        }

        let m_min = motor_left.min(motor_right);
        if m_min < MOTOR_IDLE {
            motor_left += MOTOR_IDLE - m_min;
            motor_right += MOTOR_IDLE - m_min;
        }

        motor_left = motor_left.clamp(MOTOR_IDLE, 1.0);
        motor_right = motor_right.clamp(MOTOR_IDLE, 1.0);

        BicopterOutput {
            servo_left,
            servo_right,
            motor_left,
            motor_right,
        }
    }
}
