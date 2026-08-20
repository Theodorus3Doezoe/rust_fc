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
            idle_throttle,
            servo_left_offset,
            servo_right_offset,
        }
    }

    pub fn mix(&self, throttle: f32, pid: Rates, is_armed: bool) -> BicopterOutput {
        todo!()
    }
}
