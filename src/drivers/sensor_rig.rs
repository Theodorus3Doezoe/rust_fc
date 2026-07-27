use embassy_rp::gpio::Output;
use embassy_rp::peripherals::SPI0;
use embassy_rp::spi::{Async, Spi};
use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};

use crate::imu::{AdjustableSpiSpeed, Imu};

pub type ImuSpiDevice = ExclusiveDevice<Spi<'static, SPI0, Async>, Output<'static>, NoDelay>;

pub struct SensorRig;

impl SensorRig {
    pub async fn create_imu<T, F>(
        raw_spi0: Spi<'static, SPI0, Async>,
        cs: Output<'static>,
        run_freq: u32,
        imu_builder: F,
    ) -> Result<T, T::Error>
    where
        T: Imu<SpiBus = ImuSpiDevice>,
        F: FnOnce(ImuSpiDevice) -> T,
    {
        let spi_device = ExclusiveDevice::new_no_delay(raw_spi0, cs).unwrap();
        let mut imu = imu_builder(spi_device);

        imu.init().await?;

        imu.spi_device_mut().set_frequency(run_freq);

        Ok(imu)
    }
}
