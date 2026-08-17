use core::time::Duration as CoreDuration;
use embassy_time::{Duration, Instant, Ticker};

use crate::{
    config::{ATTITUDE_FREQ_HZ, RATE_FREQ_HZ},
    filters::vqf,
    rate::IMU_CHANNEL,
    types::ImuBurst,
};
struct TempSetpoint {
    roll: f32,
    pitch: f32,
    yaw: f32,
}

#[embassy_executor::task]
pub async fn attitude_task() {
    let dt = CoreDuration::from_micros(1_000_000 / RATE_FREQ_HZ as u64);
    let mut vqf = vqf::VqfFilter::new(dt);

    let mut error: f32 = 0.0;
    let setpoint = TempSetpoint {
        roll: 0.0,
        pitch: 0.0,
        yaw: 0.0,
    };

    let mut counter: u16 = 0;

    let mut total_duration_nanos: u64 = 0;

    loop {
        let batch = IMU_CHANNEL.receive().await;
        let start = Instant::now();

        // for sample in batch {
        //     vqf.update(sample);
        // }
        //
        let orientation = vqf.update(batch);

        // let orientation = vqf.orientation();

        total_duration_nanos += start.elapsed().as_nanos();

        let (roll, pitch, yaw) = orientation.euler_angles();

        counter = (counter + 1) % 16000;

        if counter == 0 {
            let avg_nanos = total_duration_nanos / 16000;
            let avg_micros = avg_nanos / 1000;
            let avg_micros_fraction = (avg_nanos % 1000) / 100;

            defmt::info!(
                "ATTITUDE: Average read time: {}.{} µs (Budget: 1000 µs) ",
                avg_micros,
                avg_micros_fraction,
            );

            defmt::info!(
                "Roll: {}°, Pitch: {}°, Yaw: {}°",
                roll.to_degrees(),
                pitch.to_degrees(),
                yaw.to_degrees()
            );

            total_duration_nanos = 0;
        }
    }
}
