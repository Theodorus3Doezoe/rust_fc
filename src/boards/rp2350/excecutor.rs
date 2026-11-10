use super::super::PriorityLevel;
use crate::boards::to_priority;
use crate::config::{ExcecutorPriority, RATE_PRIO};
use embassy_executor::SendSpawner;
use embassy_executor::{Executor, InterruptExecutor};
use embassy_rp::interrupt;
use embassy_rp::interrupt::SWI_IRQ_0;
use embassy_rp::interrupt::{InterruptExt, Priority};

static RATE_EXECUTOR: InterruptExecutor = InterruptExecutor::new();

#[interrupt]
unsafe fn SWI_IRQ_0() {
    unsafe { RATE_EXECUTOR.on_interrupt() }
}

pub fn create_rate_excecutor() -> SendSpawner {
    let prio = to_priority::<Priority>(RATE_PRIO);
    SWI_IRQ_0.set_priority(prio);
    RATE_EXECUTOR.start(SWI_IRQ_0)
}

impl PriorityLevel for embassy_rp::interrupt::Priority {
    const HIGH: Self = Priority::P0;
    const MEDIUM: Self = Priority::P1;
    const LOW: Self = Priority::P2;
}
