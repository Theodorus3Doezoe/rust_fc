pub use crate::boards::Board;
pub use crate::sensors::Imu;

pub type BoardType = crate::boards::rp2350dev::Rp2350Dev;
pub type SelectedImuDriver<SPI> = crate::sensors::mpu6500::Mpu6500<SPI>;
// pub type ActiveFrame = _; // bicopter frame

pub mod imu {
    use super::*;
    use crate::sensors::calibrated_imu::CalibratedImu;

    pub const INIT_FREQ_HZ: u32 = 1_000_000;
    pub const RUN_FREQ_HZ: u32 = 20_000_000;

    pub type Spi = <BoardType as Board>::ImuSpi;
    pub type Raw = SelectedImuDriver<Spi>;
    pub type Calibrated = CalibratedImu<Raw>;
}

pub const RATE_FREQ_HZ: u16 = 8_000;
pub const GYRO_FILTER_CUTOFF_HZ: f32 = 80.0;

pub const ATTITUDE_FREQ_HZ: u16 = 1_000;
