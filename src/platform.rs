use crate::config::{Board, BoardType, imu};
// use crate::config::{Board, BoardType, ConcreteImu, RawImu};

pub struct Platform {
    pub board: BoardType,
    pub imu: imu::Calibrated,
}

impl Platform {
    pub async fn init() -> Self {
        let mut board = BoardType::init();
        let mut imu_spi_device = board.take_imu_spi();

        imu::Raw::init_registers(&mut imu_spi_device).await.unwrap();

        imu_spi_device.bus_mut().set_frequency(imu::RUN_FREQ_HZ);

        let raw_imu = imu::Raw::new(imu_spi_device)
            .await
            .expect("Error: IMU cannot be initialised");

        let mut imu = imu::Calibrated::new(raw_imu);
        imu.calibrate().await.expect("Imu calibration failed");

        Self { board, imu }
    }
}
