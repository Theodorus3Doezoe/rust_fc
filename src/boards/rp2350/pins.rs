use embassy_rp::pwm::{ChannelAPin, ChannelBPin, Config as PwmConf, Pwm, Slice};

use embassy_rp::peripherals::{PIN_5, PIN_6, PIN_7, PIN_8, PIN_9};

use super::{PwmPinConcrete, StaticPeri};
use embassy_rp::peripherals::{PIN_2, PIN_3, PIN_4, PIO0, PWM_SLICE1, PWM_SLICE2};
use embassy_rp::pio::{Common, Pin};

pub enum ServoSlice {
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

impl MotorPins {
    pub fn into_pio(self, common: &mut Common<'static, PIO0>) -> Pin<'static, PIO0> {
        init_motor_pin!(self, common, Pin6, Pin7, Pin8, Pin9)
    }
}
