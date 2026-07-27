use embassy_rp::bind_interrupts;
use embassy_rp::dma;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, DMA_CH1, DMA_CH2, DMA_CH3, SPI0, SPI1, USB};
use embassy_rp::spi::Error as SpiError;
use embassy_rp::spi::{Async, Config as SpiConfig, Spi};
use embassy_rp::usb::{Driver, InterruptHandler as UsbInterruptHandler};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};

use static_cell::StaticCell;

const SPI_INIT_CLOCK: u32 = 1_000_000;
const IMU_SPI_CLOCK: u32 = 10_000_000;

pub type ImuSpiDevice = ExclusiveDevice<
    Spi<'static, SPI0, Async>, // BUS
    Output<'static>,           // CS-pin
    NoDelay,                   // delay-type (geen delay)
>;

pub type Spi1Bus = Mutex<CriticalSectionRawMutex, Spi<'static, SPI1, Async>>;
pub static SPI1_BUS: StaticCell<Spi1Bus> = StaticCell::new();

bind_interrupts!(struct Irqs {
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>, dma::InterruptHandler<DMA_CH1>,
                 dma::InterruptHandler<DMA_CH2>, dma::InterruptHandler<DMA_CH3>;
});

bind_interrupts!(struct UsbIrqs {
    USBCTRL_IRQ => UsbInterruptHandler<USB>;
});

pub struct BoardHardware {
    pub imu_device: ImuSpiDevice,
    pub spi1_bus: &'static Spi1Bus,
    pub baro_cs: Output<'static>,
    pub usb_driver: Driver<'static, USB>,
}

pub fn init() -> BoardHardware {
    let p = embassy_rp::init(Default::default());

    let usb_driver = Driver::new(p.USB, UsbIrqs);

    // imu spi
    let mut imu_config = SpiConfig::default();
    imu_config.frequency = SPI_INIT_CLOCK;

    let imu_spi = Spi::new(
        p.SPI0, p.PIN_18, p.PIN_19, p.PIN_16, p.DMA_CH0, p.DMA_CH1, Irqs, imu_config,
    );

    let gyro_cs = Output::new(p.PIN_20, Level::High);
    let imu_device: ImuSpiDevice =
        ExclusiveDevice::new_no_delay(imu_spi, gyro_cs).expect("Failed to create IMU SPI device");

    let mut spi1_config = SpiConfig::default();
    spi1_config.frequency = SPI_INIT_CLOCK;
    let spi1 = Spi::new(
        p.SPI1,
        p.PIN_10,
        p.PIN_11,
        p.PIN_12,
        p.DMA_CH2,
        p.DMA_CH3,
        Irqs,
        spi1_config,
    );
    let spi1_bus = SPI1_BUS.init(Mutex::new(spi1));
    let baro_cs = Output::new(p.PIN_21, Level::High);

    BoardHardware {
        imu_device,
        spi1_bus,
        baro_cs,
        usb_driver,
    }
}
