pub mod calibrated_imu;
pub mod mpu6500;

use embedded_hal_async::spi::SpiDevice;

use crate::types::{ImuBurst, Rates, Vec3};

pub trait Imu<SPI: SpiDevice> {
    async fn read_accel(&mut self) -> Result<Vec3, SPI::Error>;
    async fn read_gyro(&mut self) -> Result<Rates, SPI::Error>;
    async fn read_burst(&mut self) -> Result<ImuBurst, SPI::Error>;
}
