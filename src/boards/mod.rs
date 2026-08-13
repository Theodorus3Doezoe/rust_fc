use embedded_hal_async::spi::SpiDevice;

pub mod rp2350dev;

pub trait Board {
    fn init() -> Self;
    type ImuSpi: SpiDevice;
    fn take_imu_spi(&mut self) -> Self::ImuSpi;
}
