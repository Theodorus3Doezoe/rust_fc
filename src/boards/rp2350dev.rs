use super::{ActuatorProvider, Board, PwmChannels};

use embassy_rp::dma::InterruptHandler as DmaInterruptHandler;
use embassy_rp::gpio::{AnyPin, Level, Output};
use embassy_rp::peripherals::{DMA_CH0, DMA_CH1, PIN_5, PWM_SLICE4, SPI0};
use embassy_rp::peripherals::{PIN_2, PIN_3, PIN_4, PWM_SLICE1, PWM_SLICE2, USB};
use embassy_rp::pwm::{ChannelAPin, ChannelBPin, Config as PwmConf, Pwm, PwmOutput, Slice};
use embassy_rp::spi::{Async, Config as SpiConfig, Spi};
use embassy_rp::usb::{Driver as RpUsbDriver, InterruptHandler as UsbInterruptHandler};
use embassy_rp::{Peri, bind_interrupts};
use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};
use fixed::traits::ToFixed;
use heapless::Vec;

bind_interrupts!(struct Irqs {
    DMA_IRQ_0 => DmaInterruptHandler<DMA_CH0>,
                 DmaInterruptHandler<DMA_CH1>;
    USBCTRL_IRQ => UsbInterruptHandler<USB>;
});

// Type aliases
type PwmPinConcrete = PwmOutput<'static>;
type ImuConcrete = ExclusiveDevice<Spi<'static, SPI0, Async>, Output<'static>, NoDelay>;
type StaticPeri<T> = Peri<'static, T>;

enum ServoSlice {
    Slice1 {
        slice: StaticPeri<PWM_SLICE1>,
        pin_a: StaticPeri<PIN_2>,
        pin_b: StaticPeri<PIN_3>,
    },
    Slice2 {
        slice: StaticPeri<PWM_SLICE2>,
        pin_a: StaticPeri<PIN_4>,
        pin_b: StaticPeri<PIN_5>,
    },
}

pub struct Rp2350Dev {
    imu_spi: Option<ImuConcrete>,
    // pwm_channels: Option<PwmChannels<PwmPinConcrete>>,
    usb_driver: Option<RpUsbDriver<'static, USB>>,
    available_slices: Vec<ServoSlice, 2>,
    pending_servo: Option<PwmPinConcrete>,
}

// preventing having to writ the same for every enum slice
macro_rules! init_any_slice {
    ($val:expr, $conf:expr, $( $variant:ident ),*) => {
        match $val {
            $(
                ServoSlice::$variant { slice, pin_a, pin_b } => {
                    Self::init_slice(slice, pin_a, pin_b, $conf)
                }
            )*
        }
    };
}

impl Rp2350Dev {
    pub fn init_slice<S, A, B>(
        slice: StaticPeri<S>,
        pin_a: StaticPeri<A>,
        pin_b: StaticPeri<B>,
        conf: PwmConf,
    ) -> (PwmPinConcrete, PwmPinConcrete)
    where
        S: Slice,
        A: ChannelAPin<S>,
        B: ChannelBPin<S>,
    {
        let pwm_slice = Pwm::new_output_ab(slice, pin_a, pin_b, conf);

        let (Some(pin_a), Some(pin_b)) = pwm_slice.split() else {
            panic!();
        };

        (pin_a, pin_b)
    }
}

impl ActuatorProvider for Rp2350Dev {
    type ServoPin = PwmPinConcrete;
    type MotorPin = f32; // temp placoholder 

    fn take_servo(&mut self) -> Option<Self::ServoPin> {
        // let frame decide config parameters, or servo or something?
        let mut pwm_conf = PwmConf::default();
        pwm_conf.divider = 15.to_fixed();
        pwm_conf.top = 39_999;
        pwm_conf.compare_a = 15_000;
        pwm_conf.compare_b = 15_000;

        // check pending_servo
        if let Some(servo) = self.pending_servo.take() {
            defmt::info!("Took pending servo");
            return Some(servo);
        }

        // take available_slices, slice them up and give a and put b in pending
        let next_slice = self.available_slices.pop()?;

        let (servo_a, servo_b) = init_any_slice!(next_slice, pwm_conf, Slice1, Slice2);

        self.pending_servo = Some(servo_b);
        Some(servo_a)
    }

    fn take_motor(&mut self) -> Option<Self::MotorPin> {
        todo!()
    }
}

impl Board for Rp2350Dev {
    type ImuSpi = ImuConcrete;
    // type ServoPin = PwmPinConcrete;
    type UsbDriver = RpUsbDriver<'static, USB>;

    fn init() -> Self {
        let p = embassy_rp::init(Default::default());

        let mut spi_config = SpiConfig::default();
        spi_config.frequency = 1_000_000;

        let imu_spi = Spi::new(
            p.SPI0, p.PIN_18, p.PIN_19, p.PIN_16, p.DMA_CH0, p.DMA_CH1, Irqs, spi_config,
        );

        let imu_cs = Output::new(p.PIN_20, Level::High);

        let imu_spi_device = ExclusiveDevice::new_no_delay(imu_spi, imu_cs).unwrap();

        let mut available_slices = Vec::new();

        let _ = available_slices.push(ServoSlice::Slice1 {
            slice: p.PWM_SLICE1,
            pin_a: p.PIN_2,
            pin_b: p.PIN_3,
        });

        let _ = available_slices.push(ServoSlice::Slice2 {
            slice: p.PWM_SLICE2,
            pin_a: p.PIN_4,
            pin_b: p.PIN_5,
        });

        // let mut pwm_conf = PwmConf::default();
        // pwm_conf.divider = 15.to_fixed();
        // pwm_conf.top = 39_999;
        // pwm_conf.compare_a = 15_000;
        // pwm_conf.compare_b = 15_000;
        //
        // let pwm_slice1 = Pwm::new_output_ab(p.PWM_SLICE1, p.PIN_2, p.PIN_3, pwm_conf.clone());
        // let pwm_slice2 = Pwm::new_output_ab(p.PWM_SLICE2, p.PIN_4, p.PIN_5, pwm_conf);
        //
        // let (Some(pwm_1), Some(pwm_2)) = pwm_slice1.split() else {
        //     panic!("Cant split PWM slice 1");
        // };
        // let (Some(pwm_3), Some(pwm_4)) = pwm_slice2.split() else {
        //     panic!("Cant split PWM slice 2");
        // };

        let usb_driver = RpUsbDriver::new(p.USB, Irqs);

        Self {
            imu_spi: Some(imu_spi_device),
            available_slices,
            pending_servo: None,
            usb_driver: Some(usb_driver),
        }
    }

    fn take_imu_spi(&mut self) -> Self::ImuSpi {
        self.imu_spi.take().unwrap()
    }

    // fn take_pwm_channels(&mut self) -> PwmChannels<Self::PwmPin> {
    //     self.pwm_channels.take().unwrap()
    // }

    fn take_usb_driver(&mut self) -> Self::UsbDriver {
        self.usb_driver.take().unwrap()
    }
}
