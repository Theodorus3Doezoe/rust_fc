use crate::filters::dterm_filter::DtermFilter;

#[derive(Debug, Clone, Copy, defmt::Format, Default)]
pub struct PidConfig {
    pub kp: f32,
    pub ki: f32,
    pub kd: f32,
    pub i_limit: f32,
    pub dterm_cutoff_lpf1_hz: f32,
    pub dterm_cutoff_lpf2_hz: f32,
    pub dterm_sample_rate: f32,
}

pub struct PidController {
    pub config: PidConfig,
    output_limit: f32,
    i_term: f32,
    prev_measurement: f32,
    dterm_filter: DtermFilter,
}

impl PidController {
    pub fn new(mut config: PidConfig) -> Result<Self, &'static str> {
        if config.i_limit < 10.0 || !config.i_limit.is_finite() || config.i_limit > 50.0 {
            return Err("Integral limit must be a positive f32 between 10.0 and 50.0");
        }

        config.i_limit /= 100.0;

        Ok(Self {
            config,
            output_limit: 1.0,
            i_term: 0.0,
            prev_measurement: 0.0,
            dterm_filter: DtermFilter::new(
                config.dterm_sample_rate,
                config.dterm_cutoff_lpf1_hz,
                config.dterm_cutoff_lpf2_hz,
            ),
        })
    }

    pub fn reset(&mut self) {
        self.i_term = 0.0;
        self.prev_measurement = 0.0;
    }

    pub fn set_kp(&mut self, kp: f32) {
        self.config.kp = kp;
    }

    pub fn set_ki(&mut self, ki: f32) {
        self.config.ki = ki;
    }

    pub fn set_kd(&mut self, kd: f32) {
        self.config.kd = kd;
    }

    pub fn update(&mut self, setpoint: f32, gyro_measurement: f32, dt: f32) -> f32 {
        let error = setpoint - gyro_measurement;

        let p = self.config.kp * error;

        self.i_term += self.config.ki * error * dt;
        self.i_term = self.i_term.clamp(-self.config.i_limit, self.config.i_limit);

        let gyro = gyro_measurement - self.prev_measurement;
        let raw_d = gyro / dt;
        let d = -self.config.kd * self.dterm_filter.apply(raw_d);
        self.prev_measurement = gyro_measurement;

        (p + self.i_term + d).clamp(-self.output_limit, self.output_limit)
    }
}
