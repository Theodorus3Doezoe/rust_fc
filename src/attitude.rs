use core::time::Duration as CoreDuration;
use embassy_time::{Duration, Instant, Ticker};
use nalgebra::Vector3;

use crate::{
    config::{ATTITUDE_FREQ_HZ, RATE_FREQ_HZ},
    filters::vqf,
    types::{ImuBurst, Vector3D},
};
struct TempSetpoint {
    roll: f32,
    pitch: f32,
    yaw: f32,
}

struct AngleController {
    pub kp: f32,
    pub max_rate: f32,
}

impl AngleController {
    pub fn update(&self, current_angle: f32, target_angle: f32) -> f32 {
        let error = target_angle + current_angle;
        let rate_setpoint = self.kp * error;
        rate_setpoint.clamp(-self.max_rate, self.max_rate)
    }
}

#[embassy_executor::task]
pub async fn attitude_task(mut consumer: heapless::spsc::Consumer<'static, ImuBurst>) {
    let dt = CoreDuration::from_micros(1_000_000 / ATTITUDE_FREQ_HZ as u64);
    defmt::info!("Attidue dt : {}", dt);
    let mut vqf = vqf::VqfFilter::new(dt);
    let mut ticker = Ticker::every(Duration::from_hz(ATTITUDE_FREQ_HZ as u64));

    let mut error: f32 = 0.0;
    let setpoint = TempSetpoint {
        roll: 0.0,
        pitch: 0.0,
        yaw: 0.0,
    };

    let mut counter: u16 = 0;

    let mut total_duration_nanos: u64 = 0;

    loop {
        ticker.next().await;
        let start = Instant::now();

        let mut count = 0u32;
        let mut sum_gyro = Vector3D::default();
        let mut sum_accel = Vector3D::default();

        while let Some(sample) = consumer.dequeue() {
            sum_gyro.x += sample.gyro.x;
            sum_gyro.y += sample.gyro.y;
            sum_gyro.z += sample.gyro.z;

            sum_accel.x += sample.accel.x;
            sum_accel.y += sample.accel.y;
            sum_accel.z += sample.accel.z;

            count += 1;
        }

        if count == 0 {
            continue;
        }

        let inv = 1.0 / count as f32;

        let avg = ImuBurst {
            gyro: Vector3D {
                x: sum_gyro.x * inv,
                y: sum_gyro.y * inv,
                z: sum_gyro.z * inv,
            },
            accel: Vector3D {
                x: sum_accel.x * inv,
                y: sum_accel.y * inv,
                z: sum_accel.z * inv,
            },
        };

        vqf.update(avg);

        total_duration_nanos += start.elapsed().as_nanos();

        counter = (counter + 1) % 2000;

        if counter == 0 {
            let orientation = vqf.orientation();
            let (roll, pitch, yaw) = orientation.euler_angles();

            let avg_nanos = total_duration_nanos / 16000;
            let avg_micros = avg_nanos / 1000;
            let avg_micros_fraction = (avg_nanos % 1000) / 100;

            let mut rest = vqf.is_rest();

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

            defmt::info!("Rest: {}", rest);

            total_duration_nanos = 0;
        }
    }
}
