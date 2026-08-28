use core::{sync::atomic::Ordering, time::Duration as CoreDuration};
use embassy_time::{Duration, Instant, Ticker};
use nalgebra::Vector3;

use crate::{
    config::{ATTITUDE_FREQ_HZ, RATE_FREQ_HZ},
    controllers::p_controller::AngleController,
    filters::vqf,
    state::{ArmBlockFlags, FlightMode, SYSTEM},
    sync::{AtomicRates, ImuConsumer},
    types::{ImuBurst, Rates, Vec3},
};

#[embassy_executor::task]
pub async fn attitude_task(
    mut consumer: ImuConsumer,
    pilot_input: &'static AtomicRates,
    rate_setpoints: &'static AtomicRates,
) {
    const MAX_YAW_RATE: f32 = f32::to_radians(180.0);
    const MAX_TILT_RATE: f32 = f32::to_radians(180.0);

    const ATTITUDE_DT: core::time::Duration =
        core::time::Duration::from_micros(1_000_000 / ATTITUDE_FREQ_HZ as u64);
    defmt::info!("Attitude dt: {} µs", ATTITUDE_DT.as_micros() as u32);
    let mut vqf = vqf::VqfFilter::new(ATTITUDE_DT);
    let mut ticker = Ticker::every(Duration::from_hz(ATTITUDE_FREQ_HZ as u64));

    let roll_kp: f32 = 6.0;
    let pitch_kp: f32 = 6.0;

    let roll_controller = AngleController::new(roll_kp, MAX_TILT_RATE);
    let pitch_controller = AngleController::new(pitch_kp, MAX_TILT_RATE);

    let mut counter: u16 = 0;

    let mut total_duration_nanos: u64 = 0;

    let times_a_sec = 5;
    let print_after_ticks = ATTITUDE_FREQ_HZ / times_a_sec;

    loop {
        ticker.next().await;
        let start = Instant::now();

        let mut count = 0u32;
        let mut sum_gyro = Rates::default();
        let mut sum_accel = Vec3::default();

        while let Some(sample) = consumer.dequeue() {
            sum_accel.x += sample.accel.x;
            sum_accel.y += sample.accel.y;
            sum_accel.z += sample.accel.z;

            sum_gyro.roll += sample.gyro.roll;
            sum_gyro.pitch += sample.gyro.pitch;
            sum_gyro.yaw += sample.gyro.yaw;

            count += 1;
        }

        if count == 0 {
            continue;
        }

        let inv = 1.0 / count as f32;

        let avg = ImuBurst {
            accel: Vec3 {
                x: sum_accel.x * inv,
                y: sum_accel.y * inv,
                z: sum_accel.z * inv,
            },
            gyro: Rates {
                roll: sum_gyro.roll * inv,
                pitch: sum_gyro.pitch * inv,
                yaw: sum_gyro.yaw * inv,
            },
        };

        let orientation = vqf.update(avg);

        let (roll, pitch, yaw) = orientation.euler_angles();

        // remove magic numbers
        if !SYSTEM.is_armed() {
            let max_tilt_rad = 25.0_f32.to_radians();
            if roll.abs() > max_tilt_rad || pitch.abs() > max_tilt_rad {
                SYSTEM.add_arm_error(ArmBlockFlags::TOO_TILTED);
            } else {
                SYSTEM.clear_arm_error(ArmBlockFlags::TOO_TILTED);
            }
        }

        let mut set_rates = pilot_input.get();

        if SYSTEM.get_flight_mode() == FlightMode::Angle {
            set_rates = Rates {
                roll: roll_controller.update(pilot_input.roll.get(), roll),
                pitch: pitch_controller.update(pilot_input.pitch.get(), pitch),
                yaw: pilot_input.yaw.get() * MAX_YAW_RATE,
            };
        }

        rate_setpoints.set(set_rates);

        total_duration_nanos += start.elapsed().as_nanos();

        counter += 1;
        if counter >= print_after_ticks {
            counter = 0;
            let avg_nanos = total_duration_nanos / print_after_ticks as u64;
            let avg_us = avg_nanos / 1000;
            let avg_frac = (avg_nanos % 1000) / 100;
            total_duration_nanos = 0;

            let rest = vqf.is_rest();

            defmt::info!(
                "[ATT  {}.{}µs] Euler: [R: {}°, P: {}°, Y: {}°] | Cmd: [R: {}°/s, P: {}°/s, Y: {}°/s] | Rest: {}",
                avg_us,
                avg_frac,
                roll.to_degrees(),
                pitch.to_degrees(),
                yaw.to_degrees(),
                set_rates.roll.to_degrees(),
                set_rates.pitch.to_degrees(),
                set_rates.yaw.to_degrees(),
                rest
            );
        }
    }
}
