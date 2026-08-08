use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::Timer;

pub static LOG_CHANNEL: Channel<CriticalSectionRawMutex, LogMessage, 20> = Channel::new();

pub enum LogMessage {
    ImuData {
        accel_x: f32,
        accel_y: f32,
        accel_z: f32,
        gyro_x: f32,
        gyro_y: f32,
        gyro_z: f32,
    },
    VqfOrientation {
        w: f32,
        x: f32,
        y: f32,
        z: f32,
    },
}

#[embassy_executor::task]
pub async fn logging_task() {
    loop {
        match LOG_CHANNEL.receive().await {
            LogMessage::ImuData {
                accel_x,
                accel_y,
                accel_z,
                gyro_x,
                gyro_y,
                gyro_z,
            } => {
                defmt::info!(
                    "IMU: a({},{},{}) g({},{},{})",
                    accel_x,
                    accel_y,
                    accel_z,
                    gyro_x,
                    gyro_y,
                    gyro_z
                );
            }
            LogMessage::VqfOrientation { w, x, y, z } => {
                defmt::info!("VQF: w={}°, x={}°, y={}°, z={}", w, x, y, z);
            }
        }
        Timer::after_micros(500).await;
    }
}
