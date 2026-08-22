use crate::sync::{AtomicF32, AtomicRates};
use embassy_time::{Duration, Instant, Ticker};

#[embassy_executor::task]
pub async fn receiver_task(
    mut rx: crate::config::receiver::Concrete,
    throttle_atomic: &'static AtomicF32,
    rate_targets: &'static AtomicRates,
    system: &'static AtomicRates,
) {
    let mut arm_hold_duration = Duration::from_millis(0);
}
