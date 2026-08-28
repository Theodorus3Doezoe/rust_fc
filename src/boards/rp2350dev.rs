use crate::actuators;
use crate::actuators::DshotChannel::MotorChannel;
use crate::state::State;

use super::{ActuatorProvider, Board, PwmChannels};

use embassy_rp::dma::InterruptHandler as DmaInterruptHandler;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{
    DMA_CH0, DMA_CH1, DMA_CH2, DMA_CH3, PIN_5, PIN_6, PIN_7, PIN_8, PIN_9, PIN_16, PWM_SLICE4, SPI0,
};
use embassy_rp::peripherals::{PIN_2, PIN_3, PIN_4, PIO0, PWM_SLICE1, PWM_SLICE2, USB};
use embassy_rp::pio::program::pio_asm;
use embassy_rp::pio::{
    Common, Config, Direction, Instance, InterruptHandler as PioHandler, Pin, Pio, PioPin,
    StateMachine,
};
use embassy_rp::pwm::{ChannelAPin, ChannelBPin, Config as PwmConf, Pwm, PwmOutput, Slice};
use embassy_rp::spi::{Async, Config as SpiConfig, Spi};
use embassy_rp::usb::{Driver as RpUsbDriver, InterruptHandler as UsbInterruptHandler};
use embassy_rp::{Peri, Peripherals, bind_interrupts};
use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};
use fixed::traits::ToFixed;
use heapless::Deque;

bind_interrupts!(struct Irqs {
    DMA_IRQ_0 => DmaInterruptHandler<DMA_CH0>,
                 DmaInterruptHandler<DMA_CH1>,
                 DmaInterruptHandler<DMA_CH2>,
                 DmaInterruptHandler<DMA_CH3>;
    PIO0_IRQ_0 => PioHandler<PIO0>;
    USBCTRL_IRQ => UsbInterruptHandler<USB>;
});

// Type aliases
type PwmPinConcrete = PwmOutput<'static>;
type ImuConcrete = ExclusiveDevice<Spi<'static, SPI0, Async>, Output<'static>, NoDelay>;
type StaticPeri<T> = Peri<'static, T>;

// could be in type.rs with a generic
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

pub enum MotorPins {
    Pin6(StaticPeri<PIN_6>),
    Pin7(StaticPeri<PIN_7>),
    Pin8(StaticPeri<PIN_8>),
    Pin9(StaticPeri<PIN_9>),
}

macro_rules! init_motor_pin {
    ($val: expr, $common: expr, $( $variant:ident ),* $(,)?) => {
        match $val {
            $(
                MotorPins::$variant(p) => {
                    $common.make_pio_pin(p)
        }
            )*
        }
    };
}

pub enum MotorSm {
    Sm0(StateMachine<'static, PIO0, 0>),
    Sm1(StateMachine<'static, PIO0, 1>),
    Sm2(StateMachine<'static, PIO0, 2>),
    Sm3(StateMachine<'static, PIO0, 3>),
}

impl MotorSm {
    pub fn push_tx(&mut self, mut throttle: u32) {
        // could use a macro or make the sm macro take different methods
        match self {
            Self::Sm0(sm) => sm.tx().push(throttle),
            Self::Sm1(sm) => sm.tx().push(throttle),
            Self::Sm2(sm) => sm.tx().push(throttle),
            Self::Sm3(sm) => sm.tx().push(throttle),
        }
    }
}

macro_rules! init_motor_sm {
    ($val: expr, $config: expr, $pin: expr, $( $variant:ident ),* $(,)?) => {
        match $val {
            $(
                MotorSm::$variant(p) => {
                    p.set_config($config);
                    p.set_pin_dirs(Direction::Out, &[&$pin]);
                    p.set_enable(true);
        }
    )*
        }
    };
}

impl MotorSm {
    pub fn init(&mut self, config: &Config<'static, PIO0>, pin: &Pin<'static, PIO0>) {
        init_motor_sm!(self, config, pin, Sm0, Sm1, Sm2, Sm3);
    }
}

impl MotorPins {
    pub fn into_pio(self, common: &mut Common<'static, PIO0>) -> Pin<'static, PIO0> {
        init_motor_pin!(self, common, Pin6, Pin7, Pin8, Pin9)
    }
}

pub struct Rp2350Dev {
    imu_spi: Option<ImuConcrete>,
    // pwm_channels: Option<PwmChannels<PwmPinConcrete>>,
    usb_driver: Option<RpUsbDriver<'static, USB>>,
    available_slices: Deque<ServoSlice, 2>,
    pending_servo: Option<PwmPinConcrete>,
    available_motors: Deque<MotorPins, 4>,
    available_sm: Deque<MotorSm, 4>,
    pio_common: Common<'static, PIO0>,
}

// preventing having to write the same for every enum slice
macro_rules! init_any_slice {
    ($val:expr, $conf:expr, $( $variant:ident ),* $(,)?) => {
        match $val {
            $(
                ServoSlice::$variant { slice, pin_a, pin_b } => {
                    ServoSlice::init_slice(slice, pin_a, pin_b, $conf)
                }
            )*
        }
    };
}

impl ServoSlice {
    pub fn init(self, conf: PwmConf) -> (PwmPinConcrete, PwmPinConcrete) {
        defmt::info!("Initializing servo slices with macro");
        init_any_slice!(self, conf, Slice1, Slice2)
    }
    // every pin and slice is a different type thats why these generics are neccesary
    fn init_slice<S, A, B>(
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
            panic!("Can't split servo slices");
        };
        defmt::info!("[init_slice] : Returning pin a & b");
        (pin_a, pin_b)
    }
}

pub enum PioDshotChannelError {
    Placeholder,
}

pub struct PioDshotChannel {
    sm: MotorSm,
}

impl PioDshotChannel {
    pub fn new(sm: MotorSm) -> Self {
        Self { sm }
    }
}

impl MotorChannel for PioDshotChannel {
    type Error = PioDshotChannelError;

    fn set_throttle(&mut self, throttle: u16) -> Result<(), Self::Error> {
        let val: u32 = (throttle as u32) << 16;

        self.sm.push_tx(val);
        Ok(())
    }
}

impl ActuatorProvider for Rp2350Dev {
    type ServoPin = PwmPinConcrete;
    type MotorPin = PioDshotChannel;

    fn take_servo(&mut self) -> Option<Self::ServoPin> {
        // let frame decide config parameters, or servo or something?
        let mut pwm_conf = PwmConf::default();
        pwm_conf.divider = 15.to_fixed();
        pwm_conf.top = 39_999;
        pwm_conf.compare_a = 15_000;
        pwm_conf.compare_b = 15_000;
        pwm_conf.enable = true;

        // check pending_servo
        if let Some(servo) = self.pending_servo.take() {
            defmt::info!("Took pending servo");
            return Some(servo);
        }

        // take available_slices, slice them up, return a and put b in pending
        let next_slice = self.available_slices.pop_front()?;
        defmt::info!("Popped servo slice from available_slices");

        let (servo_a, servo_b) = next_slice.init(pwm_conf);

        defmt::info!("Returning servo pins");
        self.pending_servo = Some(servo_b);
        Some(servo_a)
    }

    fn take_motor(&mut self) -> Option<Self::MotorPin> {
        let pio_program = pio_asm!(
            ".side_set 1 opt",
            ".wrap_target",
            "nop side 1 [2]",
            "out pins, 1 [3]",
            "nop side 0 [2]",
            ".wrap",
        );

        let dshot_speed = 600_000;
        let target_hz = dshot_speed * 10;
        let clock = embassy_rp::pio_programs::clock_divider::calculate_pio_clock_divider(target_hz);

        let next_motor_pin = self
            .available_motors
            .pop_front()
            .expect("Couldn't pop motor pin");
        let pin_dshot = next_motor_pin.into_pio(&mut self.pio_common);

        let mut config = embassy_rp::pio::Config::default();

        let loaded_program = self.pio_common.load_program(&pio_program.program);
        config.use_program(&loaded_program, &[&pin_dshot]);

        config.set_out_pins(&[&pin_dshot]);
        config.clock_divider = clock;
        config.shift_out.auto_fill = true;
        config.shift_out.threshold = 16;
        config.shift_out.direction = embassy_rp::pio::ShiftDirection::Left;

        // sm some fixen
        let mut sm_variant = self.available_sm.pop_front()?;
        sm_variant.init(&config, &pin_dshot);

        Some(PioDshotChannel::new(sm_variant))
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

        let mut available_slices = Deque::new();

        let _ = available_slices.push_back(ServoSlice::Slice1 {
            slice: p.PWM_SLICE1,
            pin_a: p.PIN_2,
            pin_b: p.PIN_3,
        });

        let _ = available_slices.push_back(ServoSlice::Slice2 {
            slice: p.PWM_SLICE2,
            pin_a: p.PIN_4,
            pin_b: p.PIN_5,
        });

        // push pins for motors into vector
        // Later I could do something to make unused pins available
        let mut available_motors = Deque::new();

        let _ = available_motors.push_back(MotorPins::Pin6(p.PIN_6));
        let _ = available_motors.push_back(MotorPins::Pin7(p.PIN_7));
        let _ = available_motors.push_back(MotorPins::Pin8(p.PIN_8));
        let _ = available_motors.push_back(MotorPins::Pin9(p.PIN_9));

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

        let usb_driver = RpUsbDriver::new(p.USB, Irqs);

        Self {
            imu_spi: Some(imu_spi_device),
            available_slices,
            pending_servo: None,
            usb_driver: Some(usb_driver),
            available_motors,
            available_sm,
            pio_common: common,
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
