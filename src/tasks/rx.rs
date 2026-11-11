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

    let mut n: u32 = 0;
    // prev state bijhouden
    loop {
        match with_timeout(timeout_duration, rx.receive()).await {
            Ok(Ok(data)) => {
                throttle_atomic.set(data.throttle);
                pilot_input.set(data.rates);
                n = n.wrapping_add(1);
                if n % 100 == 0 {
                    defmt::info!("rc: {}", data);
                }
                if !data.armed {
                    SYSTEM.set_state(State::Disarmed);
                    // arm_start = None;
                    // defmt::info!("System disarmed");
                    // flag if throttle is to high and print
                } else if data.armed && data.throttle < 0.05 {
                    SYSTEM.set_state(State::Armed);
                    // match arm_start {
                    //     Some(start) => {
                    //         if start.elapsed() >= arm_timer {
                    //             SYSTEM.set_state(State::Armed);
                    //             arm_start = None;
                    //             defmt::info!("System Armed!");
                    //         }
                    //     }
                    //     None => {
                    //         arm_start = Some(Instant::now());
                    //     }
                    // }
                } else {
                    defmt::warn!("Throttle = {}", data.throttle);
                    arm_start = None;
                }
            }
            Ok(Err(e)) => {
                // defmt::warn!("RX Failed: {:?}", defmt::Debug2Format(&e));
                SYSTEM.add_system_error(SystemErrorFlags::RX_LOST);
                // SYSTEM.set_state(State::Disarmed);
            }
            Err(e) => {
                // defmt::warn!("RX Timeout error: {:?}", e);
                SYSTEM.add_system_error(SystemErrorFlags::RX_LOST);
                // SYSTEM.set_state(State::Disarmed);
            }
        }
    }
}
