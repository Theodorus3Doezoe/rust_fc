pub mod calibrated_imu;
pub mod mpu6500;

use embedded_hal_async::spi::SpiDevice;

use crate::types::Vector3D;

#[derive(Debug, Clone, Copy, defmt::Format)]
pub struct ImuBurst {
    pub accel: Vector3D,
    pub gyro: Vector3D,
}

pub trait Imu<SPI: SpiDevice> {
    async fn read_accel(&mut self) -> Result<Vector3D, SPI::Error>;
    async fn read_gyro(&mut self) -> Result<Vector3D, SPI::Error>;
    async fn read_burst(&mut self) -> Result<ImuBurst, SPI::Error>;
}
