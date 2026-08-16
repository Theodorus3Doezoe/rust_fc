use embassy_time::{Duration, Instant, Ticker};

use crate::{
    config::{Imu, imu},
    filters::gyro::GyroFilter,
};

#[embassy_executor::task]
async fn rate_task(mut imu: imu::Calibrated) {
    let mut ticker = Ticker::every(Duration::from_hz(8000));
    let mut counter: u16 = 0;
    let mut total_duration_nanos: u64 = 0;

    let mut gyro_filter = GyroFilter::new(8000.0, 80.0);

    loop {
        ticker.next().await;

        let start = Instant::now();

        let mut burst = imu.read_burst().await.unwrap();
        burst.gyro = gyro_filter.apply(burst.gyro);

        total_duration_nanos += start.elapsed().as_nanos();

        counter = (counter + 1) % 16000;

        if counter == 0 {
            let avg_nanos = total_duration_nanos / 16000;
            let avg_micros = avg_nanos / 1000;
            let avg_micros_fraction = (avg_nanos % 1000) / 100;

            defmt::info!(
                "Average read time: {}.{} µs (Budget: 125 µs) | Latest burst: {}",
                avg_micros,
                avg_micros_fraction,
                burst
            );

            total_duration_nanos = 0;
        }
    }
}
