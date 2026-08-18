pub struct AngleController {
    kp: f32,
    max_rate: f32,
}

impl AngleController {
    pub fn new(kp: f32, max_rate: f32) -> Self {
        Self { kp, max_rate }
    }
    pub fn update(&self, target_angle: f32, current_angle: f32) -> f32 {
        let error = target_angle - current_angle;
        self.kp * error.clamp(-self.max_rate, self.max_rate)
    }

    pub fn set_kp(&mut self, kp: f32) {
        self.kp = kp;
    }
}
