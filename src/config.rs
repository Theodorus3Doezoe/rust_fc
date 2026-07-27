use crate::boards::rp2350_dev;
use crate::drivers::sensor_rig::ImuSpiDevice;
use crate::drivers::sensors::mpu6500::Mpu6500;

pub use rp2350_dev::init as init_board;
pub type ActiveImu = Mpu6500<ImuSpiDevice>;

pub const IMU_RUN_FREQ: u32 = 20_000_000;

pub type ActiveUsbDriver = embassy_rp::usb::Driver<'static, embassy_rp::peripherals::USB>;
