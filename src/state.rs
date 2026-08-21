use bitflags::bitflags;
use core::sync::atomic::{AtomicU8, AtomicU16, Ordering};
use defmt::info;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ArmBlockFlags: u16 {
        const THROTTLE_NOT_ZERO = 1 << 0;
        const TOO_TILTED        = 1 << 1;
        const CALIBRATING       = 1 << 2;
        const NO_RX             = 1 << 3;
    }


}
bitflags! {
    pub struct SystemErrorFlags: u16 {
        const RX_LOST = 1 << 0;
        const BAT_CRITICAL = 1 << 1;
        const IMU_FAILURE = 1 << 2;
        const SERVO_FAILURE = 1 << 3;
    }
}

#[repr(u8)]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum State {
    Init = 0,
    Disarmed = 1,
    Armed = 2,
    Failsafe = 3,
}

#[repr(u8)]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum FailsafeAction {
    None = 0,
    Land = 1,
}

pub struct SystemState {
    system_state: AtomicU8,
    arm_errors: AtomicU16,
    system_errors: AtomicU16,
    failsafe: AtomicU8,
}

impl SystemState {
    pub const fn new() -> Self {
        Self {
            system_state: AtomicU8::new(State::Init as u8),
            arm_errors: AtomicU16::new(ArmBlockFlags::CALIBRATING.bits()),
            system_errors: AtomicU16::new(0),
            failsafe: AtomicU8::new(FailsafeAction::None as u8),
        }
    }
    pub fn set_state(&self, state: State) {
        self.system_state.store(state as u8, Ordering::Relaxed);
    }

    pub fn get_state(&self) -> State {
        match self.system_state.load(Ordering::Relaxed) {
            0 => State::Init,
            1 => State::Disarmed,
            2 => State::Armed,
            3 => State::Failsafe,
            _ => State::Failsafe,
        }
    }

    pub fn set_failsafe(&self, state: FailsafeAction) {
        self.failsafe.store(state as u8, Ordering::Relaxed);
    }

    pub fn get_failsafe(&self) -> FailsafeAction {
        match self.failsafe.load(Ordering::Relaxed) {
            0 => FailsafeAction::None,
            1 => FailsafeAction::Land,
            _ => FailsafeAction::Land,
        }
    }

    pub fn add_arm_error(&self, flag: ArmBlockFlags) {
        self.arm_errors.fetch_or(flag.bits(), Ordering::Relaxed);
    }

    pub fn clear_arm_error(&self, flag: ArmBlockFlags) {
        self.arm_errors.fetch_and(!flag.bits(), Ordering::Relaxed);
    }

    pub fn get_arm_errors(&self) -> ArmBlockFlags {
        let raw = self.arm_errors.load(Ordering::Relaxed);
        ArmBlockFlags::from_bits_truncate(raw)
    }

    pub fn add_system_error(&self, flag: SystemErrorFlags) {
        self.system_errors.fetch_or(flag.bits(), Ordering::Relaxed);
    }

    pub fn clear_system_error(&self, flag: SystemErrorFlags) {
        self.system_errors
            .fetch_and(!flag.bits(), Ordering::Relaxed);
    }

    pub fn get_system_errors(&self) -> SystemErrorFlags {
        let raw = self.system_errors.load(Ordering::Relaxed);
        SystemErrorFlags::from_bits_truncate(raw)
    }

    pub fn is_armed(&self) -> bool {
        self.get_state() == State::Armed
    }

    pub fn can_arm(&self) -> bool {
        self.get_arm_errors().is_empty() && self.get_system_errors().is_empty()
    }
}
