pub use crate::boards::Board;
pub use crate::sensors::Imu;

pub type BoardType = crate::boards::rp2350dev::Rp2350Dev;
pub type ImuType<SPI> = crate::sensors::mpu6500::Mpu6500<SPI>;

pub type ImuSpi = <BoardType as Board>::ImuSpi;

pub type ConcreteImu = ImuType<ImuSpi>;
