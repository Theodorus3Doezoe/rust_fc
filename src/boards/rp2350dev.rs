use super::{Board, PwmChannels};

use embassy_rp::bind_interrupts;
use embassy_rp::dma;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, DMA_CH1, SPI0};
use embassy_rp::pwm::{Config as PwmConf, Pwm, PwmOutput};
use embassy_rp::spi::{Async, Config as SpiConfig, Spi};
use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};
use fixed::traits::ToFixed;

bind_interrupts!(struct Irqs {
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>,
                 dma::InterruptHandler<DMA_CH1>;
});

// Type aliases
type PwmPinConcrete = PwmOutput<'static>;
type ImuConcrete = ExclusiveDevice<Spi<'static, SPI0, Async>, Output<'static>, NoDelay>;

pub struct Rp2350Dev {
    imu_spi: Option<ImuConcrete>,
    pwm_channels: Option<PwmChannels<PwmPinConcrete>>,
}

impl Board for Rp2350Dev {
    type ImuSpi = ImuConcrete;
    type PwmPin = PwmPinConcrete;

    fn init() -> Self {
        let p = embassy_rp::init(Default::default());

        let mut spi_config = SpiConfig::default();
        spi_config.frequency = 1_000_000;

        let imu_spi = Spi::new(
            p.SPI0, p.PIN_18, p.PIN_19, p.PIN_16, p.DMA_CH0, p.DMA_CH1, Irqs, spi_config,
        );

        let imu_cs = Output::new(p.PIN_20, Level::High);

        let imu_spi_device = ExclusiveDevice::new_no_delay(imu_spi, imu_cs).unwrap();

        let mut pwm_conf = PwmConf::default();
        pwm_conf.divider = 15.to_fixed();
        pwm_conf.top = 39_999;
        pwm_conf.compare_a = 15_000;
        pwm_conf.compare_b = 15_000;

        let pwm_slice1 = Pwm::new_output_ab(p.PWM_SLICE1, p.PIN_2, p.PIN_3, pwm_conf.clone());
        let pwm_slice2 = Pwm::new_output_ab(p.PWM_SLICE2, p.PIN_4, p.PIN_5, pwm_conf);

        let (Some(pwm_1), Some(pwm_2)) = pwm_slice1.split() else {
            panic!("Cant split PWM slice 1");
        };
        let (Some(pwm_3), Some(pwm_4)) = pwm_slice2.split() else {
            panic!("Cant split PWM slice 2");
        };

        Self {
            imu_spi: Some(imu_spi_device),
            pwm_channels: Some(PwmChannels {
                pwm1: pwm_1,
                pwm2: pwm_2,
                pwm3: pwm_3,
                pwm4: pwm_4,
            }),
        }
    }

    fn take_imu_spi(&mut self) -> Self::ImuSpi {
        self.imu_spi.take().unwrap()
    }

    fn take_pwm_channels(&mut self) -> PwmChannels<Self::PwmPin> {
        self.pwm_channels.take().unwrap()
    }
}
