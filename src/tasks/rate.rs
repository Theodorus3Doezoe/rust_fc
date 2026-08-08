use crate::filters::vqf::VqfFilter;
use crate::tasks::imu_task::IMU_SIGNAL;
use crate::tasks::logger::{LOG_CHANNEL, LogMessage};
use core::time::Duration as CoreDuration;
use embassy_time::{Duration, Ticker};

const RATE_FREQ_HZ: u64 = 1000;

struct TempSetpoint {
    roll: f32,
    pitch: f32,
    yaw: f32,
}

#[embassy_executor::task]
pub async fn attitude_task() {
    let mut ticker = Ticker::every(Duration::from_hz(RATE_FREQ_HZ));
    let dt = CoreDuration::from_micros(1_000_000 / RATE_FREQ_HZ);
    let mut vqf = VqfFilter::new(dt);

    let mut error: f32 = 0.0;
    let setpoint = TempSetpoint {
        roll: 0.0,
        pitch: 0.0,
        yaw: 0.0,
    };

    let mut vqf_log_counter: u32 = 0;

    loop {
        ticker.next().await;
        if let Some(imu_data) = IMU_SIGNAL.try_take() {
            let orientation = vqf.update(&imu_data);

            vqf_log_counter += 1;
            if vqf_log_counter.is_multiple_of(100) {
                let q = orientation.quaternion();

                let _ = LOG_CHANNEL.try_send(LogMessage::VqfOrientation {
                    w: q.w,
                    x: q.i,
                    y: q.j,
                    z: q.k,
                });
            }
        }
    }
}
