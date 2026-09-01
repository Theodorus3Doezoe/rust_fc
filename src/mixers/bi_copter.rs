use crate::types::Rates;

pub struct BiCopMixer {
    idle_throttle: f32,
    servo_left_offset: f32,
    servo_right_offset: f32,
}

pub struct BicopterOutput {
    pub servo_left: f32, // -1.0 .. 1.0 rads
    pub servo_right: f32,
    pub motor_left: f32, // 0.0 .. 1.0
    pub motor_right: f32,
}

impl BiCopMixer {
    pub const fn init(idle_throttle: f32, servo_left_offset: f32, servo_right_offset: f32) -> Self {
        Self {
            idle_throttle: idle_throttle / 100.0,
            servo_left_offset,
            servo_right_offset,
        }
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
        if m_min < self.idle_throttle {
            motor_left += self.idle_throttle - m_min;
            motor_right += self.idle_throttle - m_min;
        }

        motor_left = motor_left.clamp(self.idle_throttle, 1.0);
        motor_right = motor_right.clamp(self.idle_throttle, 1.0);

        BicopterOutput {
            servo_left,
            servo_right,
            motor_left,
            motor_right,
        }
    }

    pub fn get_idle_throttle(&self) -> f32 {
        self.idle_throttle
    }
}
