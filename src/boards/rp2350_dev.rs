use embassy_executor::{InterruptExecutor, SendSpawner};
use embassy_rp::bind_interrupts;
use embassy_rp::dma;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::interrupt;
use embassy_rp::interrupt::{InterruptExt, Priority};
use embassy_rp::peripherals::{DMA_CH0, DMA_CH1, DMA_CH2, DMA_CH3, SPI0, USB};
use embassy_rp::spi::{Async, Config as SpiConfig, Spi};
use embassy_rp::usb::{Driver, InterruptHandler as UsbInterruptHandler};

use crate::imu::AdjustableSpiSpeed;
use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};

bind_interrupts!(struct Irqs {
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>,
                 dma::InterruptHandler<DMA_CH1>,
                 dma::InterruptHandler<DMA_CH2>,
                 dma::InterruptHandler<DMA_CH3>;
    USBCTRL_IRQ => UsbInterruptHandler<USB>;
});

static EXECUTOR_HIGH: InterruptExecutor = InterruptExecutor::new();

#[interrupt]
unsafe fn SWI_IRQ_1() {
    EXECUTOR_HIGH.on_interrupt();
}

pub struct Rp2350Hardware {
    pub spi0: Spi<'static, SPI0, Async>,
    pub imu_cs: Output<'static>,
    pub usb_driver: Driver<'static, USB>,
    pub high_spawner: SendSpawner,
}

impl AdjustableSpiSpeed for ExclusiveDevice<Spi<'static, SPI0, Async>, Output<'static>, NoDelay> {
    fn set_frequency(&mut self, freq_hz: u32) {
        self.bus_mut().set_frequency(freq_hz);
    }
}

pub fn init() -> Rp2350Hardware {
    let p = embassy_rp::init(Default::default());

    let usb_driver = Driver::new(p.USB, Irqs);

    let mut spi_config = SpiConfig::default();
    spi_config.frequency = 1_000_000;

    let spi0 = Spi::new(
        p.SPI0, p.PIN_18, p.PIN_19, p.PIN_16, p.DMA_CH0, p.DMA_CH1, Irqs, spi_config,
    );

    let imu_cs = Output::new(p.PIN_20, Level::High);

    interrupt::SWI_IRQ_1.set_priority(Priority::P3);
    let high_spawner = EXECUTOR_HIGH.start(interrupt::SWI_IRQ_1);

    Rp2350Hardware {
        spi0,
        imu_cs,
        usb_driver,
        high_spawner,
    }
}
