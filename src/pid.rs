use crate::filters::filters::DtermFilter;

pub struct PidController {
    pub kp: f32,
    pub ki: f32,
    pub kd: f32,

    pub integral: f32,
    pub prev_error: f32,
    dterm_filter: DtermFilter,
    output_limit: f32,
    pid_output: f32,
}

impl PidController {
    pub fn new(
        kp: f32,
        ki: f32,
        kd: f32,
        sample_rate_hz: f32,
        cutoff_hz: f32,
        output_lm: f32,
    ) -> Self {
        Self {
            kp,
            ki,
            kd,
            integral: 0.0,
            prev_error: 0.0,
            dterm_filter: DtermFilter::new(sample_rate_hz, cutoff_hz),
            output_limit: 1000.0,
            pid_output: 0.0,
        }
    }

    pub fn update(&mut self, error: f32, dt: f32) -> f32 {
        let p_term: f32 = self.kp * error;

        if self.pid_output.abs() < self.output_limit || (error * self.pid_output) < 0.0 {
            self.integral += error * dt;
        }
        let i_term: f32 = self.ki * self.integral;

        let raw_d_term: f32 = ((error - self.prev_error) / dt) * self.kd;
        let d_term = self.dterm_filter.apply(raw_d_term);

        self.prev_error = error;

        let raw_output = p_term + i_term + d_term;
        self.pid_output = raw_output.clamp(-self.output_limit, self.output_limit);
        self.pid_output
    }
}
