use crate::actuators::provider::ActuatorProvider;
use embassy_executor::SendSpawner;
use embassy_usb_driver::Driver;
use embedded_hal_async::spi::SpiDevice;

pub mod rp2350;

pub trait UartRead {
    type Error: defmt::Format + core::fmt::Debug;
    async fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Self::Error>;
}

pub trait UartWrite {
    type Error: defmt::Format + core::fmt::Debug;
    async fn write_all(&mut self, buf: &[u8]) -> Result<(), Self::Error>;
}

pub trait Board: ActuatorProvider {
    type ImuSpi: SpiDevice;
    type UsbDriver: Driver<'static>;
    type CrsfTx: UartWrite;
    type CrsfRx: UartRead;

    fn init() -> Self;
    fn take_imu_spi(&mut self) -> Self::ImuSpi;
    fn take_usb_driver(&mut self) -> Self::UsbDriver;
    fn take_crsf_uart(&mut self) -> (Self::CrsfTx, Self::CrsfRx);
    fn start_rate_executor() -> SendSpawner;
}

use crate::config::ExcecutorPriority;

pub trait PriorityLevel: Copy {
    const HIGH: Self;
    const MEDIUM: Self;
    const LOW: Self;
}

pub fn to_priority<P: PriorityLevel>(logical: ExcecutorPriority) -> P {
    match logical {
        ExcecutorPriority::High => P::HIGH,
        ExcecutorPriority::Medium => P::MEDIUM,
        ExcecutorPriority::Low => P::LOW,
    }
}
