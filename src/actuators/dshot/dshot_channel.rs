use core::convert::Infallible;

use crate::actuators::{
    channel::MotorChannel,
    dshot::{
        frame::{self, create_dshot_frame},
        state::EscLink,
    },
};

use super::frame::DshotTelemetry;

pub struct DshotChannel {
    link: &'static EscLink,
}

impl DshotChannel {
    pub const fn new(link: &'static EscLink) -> Self {
        Self { link }
    }
}

impl MotorChannel for DshotChannel {
    type Telemetry = DshotTelemetry;
    type Error = Infallible;

    fn set_throttle(&mut self, throttle: f32) -> Result<(), Self::Error> {
        let value = frame::throttle_to_dshot_value(throttle);
        let frame = create_dshot_frame(value, true);
        self.link.send(frame);
        Ok(())
    }

    fn esc_command(&mut self, cmd: u8) -> Result<(), Self::Error> {
        let frame = create_dshot_frame(cmd as u16, true);
        self.link.send(frame);
        Ok(())
    }

    fn latest_telemetry(&mut self) -> Option<DshotTelemetry> {
        self.link.latest()
    }
}
