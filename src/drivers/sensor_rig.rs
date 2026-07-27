use crate::drivers::sensors::mpu6500::Mpu6500;
use embassy_rp::gpio::Output;
use embassy_rp::peripherals::SPI0;
use embassy_rp::spi::{Async, Spi};

pub struct SensorRig {
    pub imu_spi: Spi<'static, SPI0, Async>,
    pub gyro_cs: Output<'static>,
}

impl SensorRig {
    pub fn new(imu_spi: Spi<'static, SPI0, Async>, gyro_cs: Output<'static>) -> Self {
        Self { imu_spi, gyro_cs }
    }

    pub fn mpu(self) -> Mpu6500 {
        Mpu6500::new(self.imu_spi, self.gyro_cs)
    }
}
