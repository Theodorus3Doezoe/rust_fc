use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Instant, Ticker};

use crate::{
    config::{GYRO_FILTER_CUTOFF_HZ, Imu, RATE_FREQ_HZ, imu},
    filters::gyro::GyroFilter,
    types::ImuBurst,
};

type ImuBatch = [ImuBurst; 8];

// pub static IMU_CHANNEL: Channel<ThreadModeRawMutex, ImuBatch, 2> = Channel::new();
pub static IMU_CHANNEL: Channel<ThreadModeRawMutex, ImuBurst, 10> = Channel::new();

#[embassy_executor::task]
pub async fn rate_task(mut imu: imu::Calibrated) {
    let mut ticker = Ticker::every(Duration::from_hz(RATE_FREQ_HZ as u64));
    let mut counter: u16 = 0;
    let mut total_duration_nanos: u64 = 0;

    let mut gyro_filter = GyroFilter::new(RATE_FREQ_HZ as f32, GYRO_FILTER_CUTOFF_HZ);

    let mut batch_array = ImuBatch::default();
    let mut dropped_frames: u32 = 0;
    let mut sample_count = 0;

    loop {
        ticker.next().await;

        let start = Instant::now();

        let mut burst = imu.read_burst().await.unwrap();
        burst.gyro = gyro_filter.apply(burst.gyro);

        if let Err(_) = IMU_CHANNEL.try_send(burst) {}

        // batch_array[sample_count] = burst;
        // sample_count += 1;
        //
        // if sample_count >= 8 {
        //     if let Err(_) = IMU_CHANNEL.try_send(batch_array) {
        //         dropped_frames = dropped_frames.saturating_add(1);
        //
        //         if dropped_frames.is_multiple_of(100) {
        //             defmt::warn!("Channel full! Dropped count: {}", dropped_frames);
        //         }
        //     }
        //     sample_count = 0;
        // }

        total_duration_nanos += start.elapsed().as_nanos();

        counter = (counter + 1) % 16000;

        if counter == 0 {
            let avg_nanos = total_duration_nanos / 16000;
            let avg_micros = avg_nanos / 1000;
            let avg_micros_fraction = (avg_nanos % 1000) / 100;

            defmt::info!(
                "RATE: Average read time: {}.{} µs (Budget: 125 µs) | Latest burst: {}",
                avg_micros,
                avg_micros_fraction,
                burst
            );

            total_duration_nanos = 0;
        }
    }
}
