use crate::{
    boards::Board,
    config::{BoardType, ImuSpi, ImuType},
};

pub struct System {
    pub board: BoardType,
    pub imu: ImuType<ImuSpi>,
}

pub async fn create_system() -> System {
    let mut board = BoardType::init();
    let imu_spi_device = board.take_imu_spi();

    let imu = ImuType::new(imu_spi_device)
        .await
        .expect("Error: IMU cannot be initialised");

    System { board, imu }
}
