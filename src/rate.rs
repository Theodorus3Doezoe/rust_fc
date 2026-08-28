use embassy_time::{Duration, Instant, Ticker};

use crate::{
    config::{ActiveBoard, Frame, GYRO_FILTER_CUTOFF_HZ, Imu, RATE_FREQ_HZ, frame, imu},
    controllers::{pid_controller::PidConfig, rate_pid::RatePID},
    filters::gyro::GyroFilter,
    frames::v_copter::FrameOutput,
    state::{FailsafeAction, SYSTEM, State, SystemErrorFlags},
    sync::{AtomicF32, AtomicRates, ImuProducer},
    types::{ImuBurst, Rates},
};

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

    let mut counter = 0.0;
    let mut total_duration_nanos: u64 = 0;

    let times_a_sec = 0.5;
    let print_after_ticks = RATE_FREQ_HZ as f32 / times_a_sec;

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

        let mut telemetry: Option<(Rates, Rates, FrameOutput)> = None;

        match SYSTEM.get_state() {
            State::Armed => {
                let sp = rate_setpoints.get();

                let pid = rate_pids.update(sp, filtered_gyro, RATE_DT);

                let out = frame.apply(throttle.get(), pid);

                telemetry = Some((sp, pid, out));
            }
            State::Disarmed | State::Init => {
                rate_pids.reset();
                frame.stop_all();
            }
            State::Failsafe => {
                rate_pids.reset();
                match SYSTEM.get_failsafe() {
                    FailsafeAction::None => frame.stop_all(),
                    FailsafeAction::Land => {
                        frame.stop_all();
                        // land procedure
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
                    defmt::info!(
                        "[ACT] Mix: [SL: {}, SR: {}], [ML_raw: {}, MR_Raw: {}, [ML: {}, MR: {}] -> Servos: [L: {}µs, R: {}µs] -> Motors: [L: {}, R:{}]",
                        out.mixer.servo_left,
                        out.mixer.servo_right,
                        out.actuators.ml_raw_to_dshot,
                        out.actuators.mr_raw_to_dshot,
                        out.mixer.motor_left,
                        out.mixer.motor_right,
                        out.actuators.servo_left_us,
                        out.actuators.servo_right_us,
                        out.actuators.motor_left_throttle,
                        out.actuators.motor_right_throttle,
                    );
                }
                None => {
                    defmt::info!(
                        "[RATE {}.{}µs] [STATE: {:?}] ArmBlocks: {:?}",
                        avg_us,
                        avg_frac,
                        SYSTEM.get_state(),
                        defmt::Debug2Format(&SYSTEM.get_arm_errors())
                    );
                }
            }
        }
    }
}
