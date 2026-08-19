use embedded_hal::pwm::SetDutyCycle;
use embedded_hal_async::spi::SpiDevice;

pub mod rp2350dev;

pub struct PwmChannels<P1, P2, P3, P4> {
    pub pwm1: P1,
    pub pwm2: P2,
    pub pwm3: P3,
    pub pwm4: P4,
}

pub trait Board {
    type ImuSpi: SpiDevice;
    type Pwm1: SetDutyCycle;
    type Pwm2: SetDutyCycle;
    type Pwm3: SetDutyCycle;
    type Pwm4: SetDutyCycle;

    fn init() -> Self;
    fn take_imu_spi(&mut self) -> Self::ImuSpi;
    fn take_pwm_channels(&mut self) -> PwmChannels<Self::Pwm1, Self::Pwm2, Self::Pwm3, Self::Pwm4>;
}
