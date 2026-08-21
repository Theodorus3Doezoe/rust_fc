use core::sync::atomic::Ordering;

use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Instant, Ticker};

use crate::{
    config::{Frame, GYRO_FILTER_CUTOFF_HZ, Imu, RATE_FREQ_HZ, frame, imu},
    controllers::{pid_controller::PidConfig, rate_pid::RatePID},
    filters::gyro::GyroFilter,
    state::SetPointRate,
    types::{ImuBurst, Rates},
};

#[embassy_executor::task]
pub async fn rate_task(
    mut imu: imu::Calibrated,
    mut frame: frame::Concrete,
    mut producer: heapless::spsc::Producer<'static, ImuBurst>,
    setpoints: &'static SetPointRate,
) {
    let mut ticker = Ticker::every(Duration::from_hz(RATE_FREQ_HZ as u64));
    let mut gyro_filter = GyroFilter::new(RATE_FREQ_HZ as f32, GYRO_FILTER_CUTOFF_HZ);
    const RATE_DT: f32 = 1.0 / RATE_FREQ_HZ as f32;

    // rate freq hz shouldn't have to be set 3 times
    // implement helper for gains
    let pid_conf_roll = PidConfig {
        kp: 0.15,
        ki: 0.05,
        kd: 0.005,
        i_limit: 30.0,
        dterm_cutoff_lpf1_hz: 150.0,
        dterm_cutoff_lpf2_hz: 250.0,
        dterm_sample_rate: RATE_FREQ_HZ as f32,
    };

    let pid_conf_pitch = PidConfig {
        kp: 0.15,
        ki: 0.05,
        kd: 0.005,
        i_limit: 30.0,
        dterm_cutoff_lpf1_hz: 150.0,
        dterm_cutoff_lpf2_hz: 250.0,
        dterm_sample_rate: RATE_FREQ_HZ as f32,
    };

    let pid_conf_yaw = PidConfig {
        kp: 0.25,
        ki: 0.10,
        kd: 0.0,
        i_limit: 30.0,
        dterm_cutoff_lpf1_hz: 150.0,
        dterm_cutoff_lpf2_hz: 250.0,
        dterm_sample_rate: RATE_FREQ_HZ as f32,
    };

    let mut rate_pids =
        RatePID::new(pid_conf_roll, pid_conf_pitch, pid_conf_yaw).expect("Pid config invalid");

    let mut counter: u16 = 0;
    let mut total_duration_nanos: u64 = 0;

    let times_a_sec = 2;
    let print_after_ticks = RATE_FREQ_HZ / times_a_sec;

    loop {
        ticker.next().await;

        let start = Instant::now();

        let mut burst = imu.read_burst().await.unwrap();
        burst.gyro = gyro_filter.apply(burst.gyro);

        let _ = producer.enqueue(burst);

        let setpoints = setpoints.get();

        let pid_outputs = rate_pids.update(setpoints, burst.gyro, RATE_DT);

        let _ = frame.apply(0.0, pid_outputs);
        total_duration_nanos += start.elapsed().as_nanos();

        counter += 1;
        if counter >= print_after_ticks {
            counter = 0;
            let avg_nanos = total_duration_nanos / print_after_ticks as u64;
            let avg_micros = avg_nanos / 1000;
            let avg_micros_fraction = (avg_nanos % 1000) / 100;

            defmt::info!(
                "RATE: Average read time: {}.{} µs (Budget: 125 µs) | Latest burst: {} | Setpoints: {}, PID Outputs: {}",
                avg_micros,
                avg_micros_fraction,
                burst,
                setpoints,
                pid_outputs,
            );

            total_duration_nanos = 0;
        }
    }
}
