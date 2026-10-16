use crate::actuators::dshot::driver::EscDriver;
use crate::actuators::dshot::frame::decode_dshot_telemetry;
use crate::actuators::dshot::state::EscLink;
use crate::config::MOTOR_COUNT;
use crate::config::esc::Driver;

#[embassy_executor::task(pool_size = MOTOR_COUNT)]
pub async fn esc_task(link: &'static EscLink, mut driver: Driver) {
    loop {
        let frame = link.wait().await;
        driver.send_frame(frame).await;
        let (a, b) = driver.read_pair().await;
        if let Some(t) = decode_dshot_telemetry(((a as u64) << 32) | b as u64) {
            link.publish(t.raw, t.erpm, t.erpm_period_us);
        }
    }
}
