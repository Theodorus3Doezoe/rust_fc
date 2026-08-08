use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Instant, Ticker};

use crate::calibration::calib_imu::calibrate;
use crate::config::ActiveImu;
use crate::filters::filters::GyroFilter;
use crate::imu::{Imu, ImuData};

use crate::tasks::logger::{LOG_CHANNEL, LogMessage};

pub static IMU_SIGNAL: Signal<CriticalSectionRawMutex, ImuData> = Signal::new();

#[embassy_executor::task]
pub async fn imu_task(mut imu: ActiveImu) {
    let bias = calibrate(&mut imu, 1000).await;

    let mut gyro_filter = GyroFilter::new(8000.0, 80.0);
    let mut ticker = Ticker::every(Duration::from_hz(8000));

    const TICK_BUDGET: Duration = Duration::from_micros(125);

    let mut imu_log_counter: u32 = 0;

    loop {
        ticker.next().await;
        let start = Instant::now();

        match imu.read().await {
            Ok(imu) => {
                let mut imu = bias.apply(imu);
                imu.gyro = gyro_filter.apply(imu.gyro);
                IMU_SIGNAL.signal(imu);

                imu_log_counter += 1;
                if imu_log_counter % 800 == 0 {
                    let _ = LOG_CHANNEL.try_send(LogMessage::ImuData {
                        accel_x: imu.accel.x_g,
                        accel_y: imu.accel.y_g,
                        accel_z: imu.accel.z_g,
                        gyro_x: imu.gyro.x_dps,
                        gyro_y: imu.gyro.y_dps,
                        gyro_z: imu.gyro.z_dps,
                    });
                }
            }
            Err(_) => {
                defmt::warn!("Imu reading failed in task loop");
            }
        }

        let duration = start.elapsed();
        if duration > TICK_BUDGET {
            defmt::warn!(
                "IMU OVERRUN! Took {} us (budget {} us)",
                duration.as_micros(),
                TICK_BUDGET.as_micros()
            );
        }
    }
}
