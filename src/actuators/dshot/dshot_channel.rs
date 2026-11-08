use core::{
    convert::Infallible,
    sync::atomic::{AtomicU32, Ordering},
};

use crate::actuators::{
    channel::MotorChannel,
    dshot::{
        driver::EscDriver,
        frame::{self, create_dshot_frame},
        state::EscLink,
    },
};

use super::frame::DshotTelemetry;

pub struct DshotChannel<D: EscDriver> {
    driver: D,
    link: &'static EscLink,
    drops: AtomicU32,
}

impl<D: EscDriver> DshotChannel<D> {
    pub const fn new(driver: D, link: &'static EscLink) -> Self {
        Self {
            driver,
            link,
            drops: AtomicU32::new(0),
        }
    }
}

impl<D: EscDriver> MotorChannel for DshotChannel<D> {
    type Telemetry = DshotTelemetry;
    type Error = Infallible;

    fn set_throttle(&mut self, throttle: f32) -> Result<(), Self::Error> {
        let value = frame::throttle_to_dshot_value(throttle);
        let frame = create_dshot_frame(value, true);
        if !self.driver.try_send(frame) {
            self.drops.fetch_add(1, Ordering::Relaxed);
        }
        Ok(())
    }

    fn esc_command(&mut self, cmd: u8) -> Result<(), Self::Error> {
        let frame = create_dshot_frame(cmd as u16, true);
        if !self.driver.try_send(frame) {
            self.drops.fetch_add(1, Ordering::Relaxed);
        }
        Ok(())
    }

    fn latest_telemetry(&mut self) -> Option<DshotTelemetry> {
        self.link.latest()
    }

    fn try_recv_raw(&mut self) -> Option<u64> {
        self.driver.try_recv()
    }
}
