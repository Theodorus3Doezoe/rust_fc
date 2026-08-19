use core::sync::atomic::Ordering;

use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Instant, Ticker};

use crate::{
    config::{GYRO_FILTER_CUTOFF_HZ, Imu, RATE_FREQ_HZ, imu},
    filters::gyro::GyroFilter,
    state::{Axes, SetPointRate},
    types::ImuBurst,
};

#[embassy_executor::task]
pub async fn rate_task(
    mut imu: imu::Calibrated,
    mut producer: heapless::spsc::Producer<'static, ImuBurst>,
    setpoints: &'static SetPointRate,
) {
    let mut ticker = Ticker::every(Duration::from_hz(RATE_FREQ_HZ as u64));
    let mut gyro_filter = GyroFilter::new(RATE_FREQ_HZ as f32, GYRO_FILTER_CUTOFF_HZ);

    let mut counter: u16 = 0;
    let mut total_duration_nanos: u64 = 0;

    let times_a_sec = 2;
    let print_after_ticks = RATE_FREQ_HZ / times_a_sec;

    loop {
        ticker.next().await;

        let start = Instant::now();

        let mut burst = imu.read_burst().await.unwrap();
        burst.gyro = gyro_filter.apply(burst.gyro);

        producer.enqueue(burst);

        let rates = setpoints.get();

        total_duration_nanos += start.elapsed().as_nanos();

        counter += 1;
        if counter >= print_after_ticks {
            counter = 0;
            let avg_nanos = total_duration_nanos / print_after_ticks as u64;
            let avg_micros = avg_nanos / 1000;
            let avg_micros_fraction = (avg_nanos % 1000) / 100;
            let rates_roll = rates.roll;
            let rates_pitch = rates.pitch;
            let rates_yaw = rates.yaw;

            defmt::info!(
                "RATE: Average read time: {}.{} µs (Budget: 125 µs) | Latest burst: {} | Attitude rates: {}, {}, {}",
                avg_micros,
                avg_micros_fraction,
                burst,
                rates_roll,
                rates_pitch,
                rates_yaw,
            );

            total_duration_nanos = 0;
        }
    }
}
