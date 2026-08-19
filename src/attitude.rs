use core::{sync::atomic::Ordering, time::Duration as CoreDuration};
use embassy_time::{Duration, Instant, Ticker};
use nalgebra::Vector3;

use crate::{
    config::{ATTITUDE_FREQ_HZ, RATE_FREQ_HZ},
    controllers::p_controller::AngleController,
    filters::vqf,
    state::{Axes, SetPointRate},
    types::{ImuBurst, Vector3D},
};

struct TempSetpoint {
    roll: f32,
    pitch: f32,
    yaw: f32,
}

#[embassy_executor::task]
pub async fn attitude_task(
    mut consumer: heapless::spsc::Consumer<'static, ImuBurst>,
    setpoints: &'static SetPointRate,
) {
    const MAX_ANGLE: f32 = f32::to_radians(30.0);
    const MAX_YAW_RATE: f32 = f32::to_radians(180.0);
    const MAX_TILT_RATE: f32 = f32::to_radians(30.0);

    const DT: core::time::Duration =
        core::time::Duration::from_micros(1_000_000 / ATTITUDE_FREQ_HZ as u64);
    defmt::info!("Attitude dt: {} µs", DT.as_micros() as u32);
    let mut vqf = vqf::VqfFilter::new(DT);
    let mut ticker = Ticker::every(Duration::from_hz(ATTITUDE_FREQ_HZ as u64));

    let roll_kp: f32 = 6.0;
    let pitch_kp: f32 = 6.0;

    let roll_controller = AngleController::new(roll_kp, MAX_ANGLE);
    let pitch_controller = AngleController::new(pitch_kp, MAX_ANGLE);

    // this has to be clamped with max input and normalised to -1.0 and +1.0 from controller input
    let setpoint = TempSetpoint {
        roll: 0.0,
        pitch: 0.0,
        yaw: 0.0,
    };

    let mut counter: u16 = 0;

    let mut total_duration_nanos: u64 = 0;

    let times_a_sec = 2;
    let print_after_ticks = ATTITUDE_FREQ_HZ / times_a_sec;

    loop {
        ticker.next().await;
        // let start = Instant::now();

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

        let orientation = vqf.update(avg);

        let (roll, pitch, yaw) = orientation.euler_angles();

        let rates = Axes {
            roll: roll_controller.update(setpoint.roll, roll),
            pitch: pitch_controller.update(setpoint.pitch, pitch),
            yaw: setpoint.yaw * MAX_YAW_RATE,
        };

        setpoints.set(rates);

        // total_duration_nanos += start.elapsed().as_nanos();

        counter += 1;
        if counter >= print_after_ticks {
            counter = 0;
            // let avg_nanos = total_duration_nanos / 2000;
            // let avg_micros = avg_nanos / 1000;
            // let avg_micros_fraction = (avg_nanos % 1000) / 100;

            let rest = vqf.is_rest();
            //
            // defmt::info!(
            //     "ATTITUDE: Average read time: {}.{} µs (Budget: 1000 µs) ",
            //     avg_micros,
            //     avg_micros_fraction,
            // );

            defmt::info!(
                "Att: [R: {}°, P: {}°, Y: {}°] | RateCmd: [R: {}°/s, P: {}°/s, Y: {}°/s] | Rest: {}",
                roll.to_degrees(),
                pitch.to_degrees(),
                yaw.to_degrees(),
                rates.roll.to_degrees(),
                rates.pitch.to_degrees(),
                rates.yaw.to_degrees(),
                rest
            );

            // total_duration_nanos = 0;
        }
    }
}
