#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_probe as _;

use embassy_executor::Spawner;
use embassy_time::{Duration, Ticker};

mod boards;
mod calibration;
mod config;
mod drivers;
mod filters;
mod imu;
mod tasks;
mod types;

use config::{ActiveImu, IMU_RUN_FREQ, init_board};
use drivers::sensor_rig::SensorRig;
use tasks::imu_task::{IMU_SIGNAL, imu_task};
use tasks::logger_task::logger_task;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let board = init_board();
    defmt::info!("iniatlizing board");
    _spawner.spawn(logger_task(board.usb_driver).unwrap());
    defmt::info!("Spawned usb task");

    let imu = SensorRig::create_imu(board.spi0, board.imu_cs, IMU_RUN_FREQ, ActiveImu::new)
        .await
        .expect("Failed to initialize IMU");

    board.high_spawner.spawn(imu_task(imu).unwrap());
    defmt::info!("Spanwed imu task");

    let mut ticker = Ticker::every(Duration::from_hz(10));

    loop {
        if let Some(imu) = IMU_SIGNAL.try_take() {
            log::info!(
                "accel_x:{} accel_y:{} accel_z:{} gyro_x:{} gyro_y:{} gyro_z:{}",
                imu.accel.x_g,
                imu.accel.y_g,
                imu.accel.z_g,
                imu.gyro.x_dps,
                imu.gyro.y_dps,
                imu.gyro.z_dps
            );

            // defmt::info!(
            //     "accel_x:{} accel_y:{} accel_z:{} gyro_x:{} gyro_y:{} gyro_z:{}",
            //     imu.accel.x_g,
            //     imu.accel.y_g,
            //     imu.accel.z_g,
            //     imu.gyro.x_dps,
            //     imu.gyro.y_dps,
            //     imu.gyro.z_dps
            // );
        }

        ticker.next().await;
    }
}
