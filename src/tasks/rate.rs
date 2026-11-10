use embassy_time::{Duration, Instant, Ticker};

use crate::filters::rpm_filter::{self, RpmFilter};
use crate::sync::AtomicPid;
use crate::{
    config::{Frame, GYRO_FILTER_CUTOFF_HZ, Imu, MOTOR_POLE_PAIRS, RATE_FREQ_HZ, frame, imu},
    controllers::{pid_controller::PidConfig, rate_pid::RatePID},
    filters::gyro::GyroFilter,
    frames::FrameOutput,
    state::{FailsafeAction, SYSTEM, State, SystemErrorFlags},
    sync::{AtomicF32, AtomicRates, ImuProducer},
    types::Rates,
};

use crate::sync::{PID_VALS, PITCH_AXIS, ROLL_AXIS, YAW_AXIS};
#[embassy_executor::task]
pub async fn rate_task(
    mut imu: imu::Calibrated,
    mut frame: frame::Concrete,
    mut producer: ImuProducer,
    rate_setpoints: &'static AtomicRates,
    throttle: &'static AtomicF32,
) {
    let mut ticker = Ticker::every(Duration::from_hz(RATE_FREQ_HZ as u64));
    let mut gyro_filter = GyroFilter::new(RATE_FREQ_HZ as f32, GYRO_FILTER_CUTOFF_HZ);
    const RATE_DT: f32 = 1.0 / RATE_FREQ_HZ as f32;

    let pid_conf = AtomicPid::pid_configs();
    let mut rate_pids =
        RatePID::new(pid_conf[0], pid_conf[1], pid_conf[2]).expect("Pid config invalid");

    const MOTORS: usize = <frame::Concrete as Frame>::MOTOR_COUNT;
    const SLOTS: usize = MOTORS * rpm_filter::HARMONICS;
    let mut rpm_filter = RpmFilter::<SLOTS>::new(RATE_FREQ_HZ as f32);

    let mut counter = 0.0;
    let mut total_duration_nanos: u64 = 0;

    let times_a_sec = 0.5;
    let print_after_ticks = RATE_FREQ_HZ as f32 / times_a_sec;
    let mut prev_state: State = SYSTEM.get_state();

    loop {
        ticker.next().await;

        let start = Instant::now();

        let burst = match imu.read_burst().await {
            Ok(b) => {
                SYSTEM.clear_system_error(SystemErrorFlags::IMU_FAILURE);
                b
            }
            Err(e) => {
                defmt::warn!("IMU error: {:?}", defmt::Debug2Format(&e));
                SYSTEM.add_system_error(SystemErrorFlags::IMU_FAILURE);
                SYSTEM.set_state(State::Failsafe);
                return;
            }
        };

        let _ = producer.enqueue(burst);

        let filtered_gyro = gyro_filter.apply(burst.gyro);
        let esc_telemetry = frame.read_telemetry();
        let hz: [Option<f32>; MOTORS] = core::array::from_fn(|i| {
            esc_telemetry[i].map(|t| t.erpm as f32 / 60.0 / MOTOR_POLE_PAIRS)
        });

        rpm_filter.update(&hz);

        let filtered_gyro = rpm_filter.apply(filtered_gyro);

        let mut telemetry: Option<(Rates, Rates, FrameOutput)> = None;
        let current_state = SYSTEM.get_state();
        let state_changed = current_state != prev_state;

        prev_state = current_state;

        match current_state {
            State::Armed => {
                if state_changed {
                    let gains = [
                        PID_VALS[ROLL_AXIS].get(),
                        PID_VALS[PITCH_AXIS].get(),
                        PID_VALS[YAW_AXIS].get(),
                    ];
                    rate_pids.set_all(gains);
                }

                let sp = rate_setpoints.get();

                let pid = rate_pids.update(sp, filtered_gyro, RATE_DT);

                let throttle_now = throttle.get();
                let out = if throttle_now <= 0.05 {
                    frame.stop_all()
                } else {
                    frame.apply(throttle_now, pid)
                };
                telemetry = Some((sp, pid, out));
            }
            State::Disarmed | State::Init => {
                rate_pids.reset();
                frame.stop_all();
            }
            State::Failsafe => {
                defmt::warn!("Failsafe triggerd");
                rate_pids.reset();
                match SYSTEM.get_failsafe() {
                    FailsafeAction::None => {
                        let _ = frame.stop_all();
                    }
                    FailsafeAction::Land => {
                        let _ = frame.stop_all();
                    }
                }
            }
        }

        total_duration_nanos += start.elapsed().as_nanos();

        counter += 1.0;
        if counter >= print_after_ticks {
            counter = 0.0;
            let avg_nanos = total_duration_nanos / print_after_ticks as u64;
            let avg_us = avg_nanos / 1000;
            let avg_frac = (avg_nanos % 1000) / 100;
            total_duration_nanos = 0;

            match telemetry {
                Some((sp, pid, out)) => {
                    defmt::info!(
                        "[RATE {}.{}µs] [ARMED] Burst: {:?} | Set: {:?} | PID: {:?}",
                        avg_us,
                        avg_frac,
                        burst,
                        sp,
                        pid
                    );
                    for i in 0..out.motor_count {
                        defmt::info!("[ACT] M{}: {}", i, out.motors[i as usize]);
                    }
                    for i in 0..out.servo_count {
                        defmt::info!("[ACT] S{}: {}µs", i, out.servo_us[i as usize]);
                    }
                    defmt::info!("[NOTCH] {:?}", hz);
                }
                None => {
                    defmt::info!(
                        "[RATE {}.{}µs] [STATE: {:?}] ArmBlocks: {:?}",
                        avg_us,
                        avg_frac,
                        SYSTEM.get_state(),
                        defmt::Debug2Format(&SYSTEM.get_arm_errors())
                    );
                    defmt::info!(
                        "[ESC] ML {:?} | MR {:?}",
                        esc_telemetry[0],
                        esc_telemetry[1]
                    );

                    defmt::info!("[NOTCH] {:?}", hz);
                }
            }
        }
    }
}
