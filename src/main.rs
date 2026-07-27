#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_probe as _;

use embassy_executor::{InterruptExecutor, Spawner};
use embassy_rp::interrupt;
use embassy_rp::interrupt::{InterruptExt, Priority};
use embassy_time::{Duration, Ticker};

mod boards;
mod calibration;
mod config;
mod drivers;
mod filters;
mod tasks;
mod types;

use boards::rp2350_dev;
use drivers::sensor_rig::SensorRig;
use drivers::sensors::mpu6500::{ImuData, Mpu6500};
use tasks::imu_task::{IMU_SIGNAL, imu_task};
use tasks::logger_task::logger_task;

static EXECUTOR_HIGH: InterruptExecutor = InterruptExecutor::new();

#[interrupt]
unsafe fn SWI_IRQ_1() {
    EXECUTOR_HIGH.on_interrupt();
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let board = rp2350_dev::init();

    interrupt::SWI_IRQ_1.set_priority(Priority::P3);

    let high_spawner = EXECUTOR_HIGH.start(interrupt::SWI_IRQ_1);

    let rig = SensorRig::new(board.imu_spi, board.gyro_cs);
    let mpu = rig.mpu();

    high_spawner.spawn(imu_task(mpu).unwrap());

    _spawner.spawn(logger_task(board.usb_driver).unwrap());

    let mut ticker = Ticker::every(Duration::from_hz(400));

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

            defmt::info!(
                "accel_x:{} accel_y:{} accel_z:{} gyro_x:{} gyro_y:{} gyro_z:{}",
                imu.accel.x_g,
                imu.accel.y_g,
                imu.accel.z_g,
                imu.gyro.x_dps,
                imu.gyro.y_dps,
                imu.gyro.z_dps
            );
        }

        ticker.next().await;
    }
}
