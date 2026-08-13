pub use crate::boards::rp2350dev::rp2350dev as BoardType;

pub use crate::sensors::mpu6500::Mpu6500 as ImuType;
pub type ImuSpi = <BoardType as crate::boards::Board>::ImuSpi;
pub type ConcreteImu = ImuType<ImuSpi>;
