use crate::actuators::provider::ActuatorProvider;
use embassy_usb_driver::Driver;
use embedded_hal_async::spi::SpiDevice;

pub mod rp2350;

pub trait Board: ActuatorProvider {
    type ImuSpi: SpiDevice;
    type UsbDriver: Driver<'static>;

    fn init() -> Self;
    fn take_imu_spi(&mut self) -> Self::ImuSpi;
    fn take_usb_driver(&mut self) -> Self::UsbDriver;
}
