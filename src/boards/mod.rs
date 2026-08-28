use crate::actuators::DshotChannel::MotorChannel;
use embassy_usb_driver::Driver;
use embedded_hal::pwm::SetDutyCycle;
use embedded_hal_async::spi::SpiDevice;

pub mod rp2350dev;

pub trait ActuatorProvider {
    type ServoPin: SetDutyCycle;
    type MotorPin: MotorChannel;

    fn take_servo(&mut self) -> Option<Self::ServoPin>;
    fn take_motor(&mut self) -> Option<Self::MotorPin>;
}

pub trait Board: ActuatorProvider {
    type ImuSpi: SpiDevice;
    type UsbDriver: Driver<'static>;

    fn init() -> Self;
    fn take_imu_spi(&mut self) -> Self::ImuSpi;
    fn take_usb_driver(&mut self) -> Self::UsbDriver;
}
