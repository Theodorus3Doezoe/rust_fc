use crate::controllers::pid_controller::{PidConfig, PidController};
use crate::types::Rates;

pub struct RatePID {
    roll: PidController,
    pitch: PidController,
    yaw: PidController,
}

impl RatePID {
    // clean it later to a single config
    pub fn new(roll: PidConfig, pitch: PidConfig, yaw: PidConfig) -> Result<Self, &'static str> {
        Ok(Self {
            roll: PidController::new(roll)?,
            pitch: PidController::new(pitch)?,
            yaw: PidController::new(yaw)?,
        })
    }

    pub fn update(&mut self, setpoints: Rates, gyro: Rates, dt: f32) -> Rates {
        Rates {
            roll: self.roll.update(setpoints.roll, gyro.roll, dt),
            pitch: self.pitch.update(setpoints.pitch, gyro.pitch, dt),
            yaw: self.yaw.update(setpoints.yaw, gyro.yaw, dt),
        }
    }

    pub fn reset(&mut self) {
        self.roll.reset();
        self.pitch.reset();
        self.yaw.reset();
    }

    pub fn set_ki(&mut self, ki: Rates) {
        self.roll.set_ki(ki.roll);
        self.pitch.set_ki(ki.pitch);
        self.yaw.set_ki(ki.yaw);
    }

    pub fn set_kp(&mut self, kp: Rates) {
        self.roll.set_kp(kp.roll);
        self.pitch.set_kp(kp.pitch);
        self.yaw.set_kp(kp.yaw);
    }

    pub fn set_kd(&mut self, kd: Rates) {
        self.roll.set_kd(kd.roll);
        self.pitch.set_kd(kd.pitch);
        self.yaw.set_kd(kd.yaw);
    }
}
