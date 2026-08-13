use crate::config::{Board, BoardType, ConcreteImu};

pub struct System {
    pub board: BoardType,
    pub imu: ConcreteImu,
}

pub async fn create_system() -> System {
    let mut board = BoardType::init();
    let imu_spi_device = board.take_imu_spi();

    let imu = ConcreteImu::new(imu_spi_device)
        .await
        .expect("Error: IMU cannot be initialised");

    System { board, imu }
}
