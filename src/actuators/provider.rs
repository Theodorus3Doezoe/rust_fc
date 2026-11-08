use crate::actuators::dshot::{driver::EscDriver, dshot_channel::DshotChannel, state::claim_link};
use embedded_hal::pwm::SetDutyCycle;

pub trait ActuatorProvider {
    type ServoPin: SetDutyCycle;
    type Esc: EscDriver;

    fn take_servo(&mut self) -> Option<Self::ServoPin>;
    fn take_esc_driver(&mut self) -> Option<Self::Esc>;

    // dshot specifik now
    fn take_motor(&mut self) -> Option<DshotChannel<Self::Esc>> {
        let link = claim_link()?;
        let driver = self.take_esc_driver()?;
        Some(DshotChannel::new(driver, link))
    }
}
