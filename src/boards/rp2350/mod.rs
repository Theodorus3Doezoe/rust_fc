pub mod excecutor;
pub mod imu_spi;
pub mod pins;
pub mod pio;
pub mod servo_pwm;
pub mod uart;
pub mod usb;

use super::{ActuatorProvider, Board};
use embassy_rp::dma::InterruptHandler as DmaInterruptHandler;
use embassy_rp::gpio::Output;
use embassy_rp::peripherals::{DMA_CH0, DMA_CH1, DMA_CH2, DMA_CH3, SPI0, UART0};
use embassy_rp::peripherals::{PIO0, USB};
use embassy_rp::pio::{Common, InterruptHandler as PioHandler, LoadedProgram, Pio};
use embassy_rp::pwm::PwmOutput;
use embassy_rp::spi::{Async, Spi};
use embassy_rp::uart::{
    Async as UartAsync, Config as UartConfig, Error, InterruptHandler as UartInterruptHandler,
    Uart, UartRx, UartTx,
};
use embassy_rp::usb::{Driver as RpUsbDriver, InterruptHandler as UsbInterruptHandler};
use embassy_rp::{Peri, bind_interrupts};
use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};

use crate::boards::rp2350::excecutor::create_rate_excecutor;
use crate::boards::rp2350::pio::dshot_pio::{MotorSm, PioEscDriver, take_esc_driver};

use heapless::Deque;

bind_interrupts!(pub struct Irqs {
    DMA_IRQ_0 => DmaInterruptHandler<DMA_CH0>,
                 DmaInterruptHandler<DMA_CH1>,
                 DmaInterruptHandler<DMA_CH2>,
                 DmaInterruptHandler<DMA_CH3>;
    PIO0_IRQ_0 => PioHandler<PIO0>;
    USBCTRL_IRQ => UsbInterruptHandler<USB>;
    UART0_IRQ => UartInterruptHandler<UART0>;
});
//Type aliases
pub type PwmPinConcrete = PwmOutput<'static>;
pub type ImuConcrete = ExclusiveDevice<Spi<'static, SPI0, Async>, Output<'static>, NoDelay>;
pub type StaticPeri<T> = Peri<'static, T>;

pub struct Rp2350Dev {
    imu_spi: Option<imu_spi::ImuConcrete>,
    usb_driver: Option<RpUsbDriver<'static, USB>>,
    pub available_slices: Deque<pins::ServoSlice, 2>,
    pub pending_servo: Option<PwmPinConcrete>,
    pub available_motors: Deque<pins::MotorPins, 4>,
    pub available_sm: Deque<MotorSm, 4>,
    pub pio_common: Common<'static, PIO0>,
    pub motor_program: Option<LoadedProgram<'static, PIO0>>,
    crsf_uart: Option<Uart<'static, UartAsync>>,
}

impl Board for Rp2350Dev {
    type ImuSpi = ImuConcrete;
    type UsbDriver = RpUsbDriver<'static, USB>;
    type CrsfTx = UartTx<'static, UartAsync>;
    type CrsfRx = UartRx<'static, UartAsync>;

    fn init() -> Self {
        let p = embassy_rp::init(Default::default());

        let imu_spi_device = imu_spi::create_imu_spi(
            p.SPI0, p.PIN_18, p.PIN_19, p.PIN_16, p.DMA_CH0, p.DMA_CH1, p.PIN_20, Irqs,
        );

        let mut available_slices = Deque::new();

        let _ = available_slices.push_back(pins::ServoSlice::Slice1 {
            slice: p.PWM_SLICE1,
            pin_a: p.PIN_2,
            pin_b: p.PIN_3,
        });

        let _ = available_slices.push_back(pins::ServoSlice::Slice2 {
            slice: p.PWM_SLICE2,
            pin_a: p.PIN_4,
            pin_b: p.PIN_5,
        });

        // push pins for motors into vector
        // Later I could do something to make unused pins available
        let mut available_motors = Deque::new();

        let _ = available_motors.push_back(pins::MotorPins::Pin6(p.PIN_6));
        let _ = available_motors.push_back(pins::MotorPins::Pin7(p.PIN_7));
        let _ = available_motors.push_back(pins::MotorPins::Pin8(p.PIN_8));
        let _ = available_motors.push_back(pins::MotorPins::Pin9(p.PIN_9));

        let Pio {
            common,
            sm0,
            sm1,
            sm2,
            sm3,
            ..
        } = Pio::new(p.PIO0, Irqs);

        // push created pio state machines into pio
        let mut available_sm = Deque::new();
        available_sm.push_back(MotorSm::Sm0(sm0));
        available_sm.push_back(MotorSm::Sm1(sm1));
        available_sm.push_back(MotorSm::Sm2(sm2));
        available_sm.push_back(MotorSm::Sm3(sm3));

        let usb_driver = usb::create_usb(p.USB, Irqs);

        // uart crsf
        let mut uart_config = UartConfig::default();
        uart_config.baudrate = 420_000;
        let crsf_uart = Uart::new(
            p.UART0,
            p.PIN_0,
            p.PIN_1,
            Irqs,
            p.DMA_CH2,
            p.DMA_CH3,
            uart_config,
        );

        Self {
            imu_spi: Some(imu_spi_device),
            available_slices,
            pending_servo: None,
            usb_driver: Some(usb_driver),
            available_motors,
            available_sm,
            pio_common: common,
            motor_program: None,
            crsf_uart: Some(crsf_uart),
        }
    }

    fn take_imu_spi(&mut self) -> Self::ImuSpi {
        self.imu_spi.take().unwrap()
    }

    fn take_usb_driver(&mut self) -> Self::UsbDriver {
        self.usb_driver.take().unwrap()
    }

    fn take_crsf_uart(&mut self) -> (Self::CrsfTx, Self::CrsfRx) {
        self.crsf_uart.take().unwrap().split()
    }

    fn start_rate_executor() -> embassy_executor::SendSpawner {
        create_rate_excecutor()
    }
}

impl ActuatorProvider for Rp2350Dev {
    type ServoPin = PwmPinConcrete;
    type Esc = PioEscDriver;

    fn take_servo(&mut self) -> Option<Self::ServoPin> {
        servo_pwm::take_servo(self)
    }

    fn take_esc_driver(&mut self) -> Option<Self::Esc> {
        take_esc_driver(self)
    }
}
