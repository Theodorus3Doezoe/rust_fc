use crate::actuators::dshot::frame::decode_dshot_telemetry;
use crate::actuators::dshot::state::link_at;
use crate::sync::Receiver;

#[embassy_executor::task]
pub async fn esc_task(mut rx: Receiver<'static, (usize, u64)>) {
    loop {
        let (index, raw) = rx.receive().await;
        if let Some(t) = decode_dshot_telemetry(raw) {
            // Haal de link op basis van de motor-index
            if let Some(link) = link_at(index) {
                link.publish(t.raw, t.erpm, t.erpm_period_us);
            }
        }
    }
}
