use crate::{
    receiver::receiver::Receiver,
    state::{SYSTEM, State, SystemErrorFlags},
    sync::{AtomicF32, AtomicRates},
};
use embassy_time::{Duration, Instant, with_timeout};

#[embassy_executor::task]
pub async fn receiver_task(
    mut rx: crate::config::receiver::Concrete,
    throttle_atomic: &'static AtomicF32,
    pilot_input: &'static AtomicRates,
) {
    let timeout_duration = Duration::from_millis(200);
    let arm_timer = Duration::from_millis(2000);
    let mut arm_start: Option<Instant> = None;

    loop {
        match with_timeout(timeout_duration, rx.receive()).await {
            Ok(Ok(data)) => {
                throttle_atomic.set(data.throttle);
                pilot_input.set(data.rates);

                if data.disarm {
                    SYSTEM.set_state(State::Disarmed);
                    arm_start = None;
                    defmt::info!("System disarmed");
                } else if data.arm && data.throttle < 0.05 {
                    match arm_start {
                        Some(start) => {
                            if start.elapsed() >= arm_timer {
                                SYSTEM.set_state(State::Armed);
                                arm_start = None;
                                defmt::info!("System Armed!");
                            }
                        }
                        None => {
                            arm_start = Some(Instant::now());
                        }
                    }
                } else {
                    arm_start = None;
                }
            }
            Ok(Err(e)) => {
                defmt::warn!("RX Failed: {:?}", defmt::Debug2Format(&e));
                SYSTEM.add_system_error(SystemErrorFlags::RX_LOST);
                SYSTEM.set_state(State::Disarmed);
            }
            Err(e) => {
                defmt::warn!("RX Timeout error: {:?}", e);
                SYSTEM.add_system_error(SystemErrorFlags::RX_LOST);
                SYSTEM.set_state(State::Disarmed);
            }
        }
    }
}
