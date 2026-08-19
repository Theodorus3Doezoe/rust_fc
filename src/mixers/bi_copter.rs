use crate::types::Rates;

pub struct BiCopMixer {
    idle_throttle: f32,
    servo_left_offset: f32,
    servo_right_offset: f32,
}

impl BiCopMixer {
    pub const fn new(idle_throttle: f32, servo_left_offset: f32, servo_right_offset: f32) -> Self {
        Self {
            idle_throttle,
            servo_left_offset,
            servo_right_offset,
        }
    }

    pub fn update(pid_outputs: Rates) {}
}
