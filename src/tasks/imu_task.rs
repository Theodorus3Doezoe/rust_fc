use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Instant, Ticker};

use crate::calibration::calib_imu::calibrate;
use crate::config::ActiveImu;
use crate::filters::filters::GyroFilter;
use crate::imu::{Imu, ImuData};

pub static IMU_SIGNAL: Signal<CriticalSectionRawMutex, ImuData> = Signal::new();

#[embassy_executor::task]
pub async fn imu_task(mut imu: ActiveImu) {
    let bias = calibrate(&mut imu, 1000).await;

    let mut gyro_filter = GyroFilter::new(8000.0, 80.0);
    let mut ticker = Ticker::every(Duration::from_hz(8000));

    const TICK_BUDGET: Duration = Duration::from_micros(125);
    loop {
        let start = Instant::now();

        match imu.read().await {
            Ok(imu) => {
                let mut imu = bias.apply(imu);
                imu.gyro = gyro_filter.apply(imu.gyro);
                IMU_SIGNAL.signal(imu);
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
        ticker.next().await;
    }
}
