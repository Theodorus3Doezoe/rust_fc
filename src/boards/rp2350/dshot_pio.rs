use super::{MotorPinConcrete, Rp2350Dev};
use crate::actuators::DshotChannel::MotorChannel;
use embassy_rp::peripherals::PIO0;
use embassy_rp::pio::program::pio_asm;
use embassy_rp::pio::{Config, Direction, Pin, StateMachine};

pub enum MotorSm {
    Sm0(StateMachine<'static, PIO0, 0>),
    Sm1(StateMachine<'static, PIO0, 1>),
    Sm2(StateMachine<'static, PIO0, 2>),
    Sm3(StateMachine<'static, PIO0, 3>),
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

    pub fn push_tx(&mut self, throttle: u32) {
        // could use a macro or make the sm macro take different methods
        match self {
            Self::Sm0(sm) => sm.tx().push(throttle),
            Self::Sm1(sm) => sm.tx().push(throttle),
            Self::Sm2(sm) => sm.tx().push(throttle),
            Self::Sm3(sm) => sm.tx().push(throttle),
        }
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

pub fn take_motor(b: &mut Rp2350Dev) -> Option<MotorPinConcrete> {
    let pio_program = pio_asm!(
        ".side_set 1 opt",
        ".wrap_target",
        "pull",
        "set y, 15",
        "bitloop:",
        "nop side 1 [2]",
        "out pins, 1 [3]",
        "nop side 0 [2]",
        "jmp y-- bitloop",
        "set x, 24",
        "gap:",
        "nop side 0 [7]",
        "jmp x-- gap",
        ".wrap",
    );
    let dshot_speed = 600_000;
    let target_hz = dshot_speed * 10;
    let clock = embassy_rp::pio_programs::clock_divider::calculate_pio_clock_divider(target_hz);

    let next_motor_pin = b
        .available_motors
        .pop_front()
        .expect("Couldn't pop motor pin");
    let pin_dshot = next_motor_pin.into_pio(&mut b.pio_common);

    let mut config = embassy_rp::pio::Config::default();

    let loaded_program = b.pio_common.load_program(&pio_program.program);
    config.use_program(&loaded_program, &[&pin_dshot]);

    config.set_out_pins(&[&pin_dshot]);
    config.clock_divider = clock;
    // config.shift_out.auto_fill = true;
    config.shift_out.threshold = 16;
    config.shift_out.direction = embassy_rp::pio::ShiftDirection::Left;

    // sm some fixen
    let mut sm_variant = b.available_sm.pop_front()?;
    sm_variant.init(&config, &pin_dshot);

    Some(PioDshotChannel::new(sm_variant))
}
