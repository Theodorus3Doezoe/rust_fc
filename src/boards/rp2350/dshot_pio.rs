use super::{MotorPinConcrete, Rp2350Dev};
use crate::actuators::DshotChannel::MotorChannel;
use crate::helpers::dshot::{decode_dshot_telemetry, DshotTelemetry};
use embassy_rp::gpio::Pull;
use embassy_rp::peripherals::PIO0;
use embassy_rp::pio::program::pio_file;
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

macro_rules! read_motor_rx {
    ($sm: expr) => {{
        let rx = $sm.rx();
        let pairs = (rx.level() / 2) as usize;
        let mut decoded = None;

        for _ in 0..pairs {
            let first = rx.pull();
            let second = rx.pull();

            if let Some(telemetry) =
                decode_dshot_telemetry(((first as u64) << 32) | second as u64)
            {
                decoded = Some(telemetry);
            }
        }

        decoded
    }};
}

impl MotorSm {
    pub fn init(&mut self, config: &Config<'static, PIO0>, pin: &Pin<'static, PIO0>) {
        init_motor_sm!(self, config, pin, Sm0, Sm1, Sm2, Sm3);
    }

    pub fn push_tx(&mut self, throttle: u32) {
        // could use a macro or make the sm macro take different methods
        match self {
            Self::Sm0(sm) => {
                let tx = sm.tx();
                if tx.empty() {
                    tx.push(throttle);
                }
            }
            Self::Sm1(sm) => {
                let tx = sm.tx();
                if tx.empty() {
                    tx.push(throttle);
                }
            }
            Self::Sm2(sm) => {
                let tx = sm.tx();
                if tx.empty() {
                    tx.push(throttle);
                }
            }
            Self::Sm3(sm) => {
                let tx = sm.tx();
                if tx.empty() {
                    tx.push(throttle);
                }
            }
        }
    }

    pub fn read_telemetry(&mut self) -> Option<DshotTelemetry> {
        match self {
            Self::Sm0(sm) => read_motor_rx!(sm),
            Self::Sm1(sm) => read_motor_rx!(sm),
            Self::Sm2(sm) => read_motor_rx!(sm),
            Self::Sm3(sm) => read_motor_rx!(sm),
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

    fn read_telemetry(&mut self) -> Option<DshotTelemetry> {
        self.sm.read_telemetry()
    }
}

pub fn take_motor(b: &mut Rp2350Dev) -> Option<MotorPinConcrete> {
    let pio_program = pio_file!(
        "src/boards/rp2350/bidir_dshot.pio",
        options(max_program_size = 32)
    );
    let dshot_speed = 600_000;
    let target_hz = dshot_speed * 40;
    let clock = embassy_rp::pio_programs::clock_divider::calculate_pio_clock_divider(target_hz);

    let next_motor_pin = b
        .available_motors
        .pop_front()
        .expect("Couldn't pop motor pin");
    let mut pin_dshot = next_motor_pin.into_pio(&mut b.pio_common);
    pin_dshot.set_pull(Pull::Up);

    let mut config = embassy_rp::pio::Config::default();

    if b.motor_program.is_none() {
        let loaded = b.pio_common.load_program(&pio_program.program);
        b.motor_program = Some(loaded);
    }
    let loaded_program = b
        .motor_program
        .as_ref()
        .expect("Motor program not loaded");
    config.use_program(loaded_program, &[]);

    config.set_jmp_pin(&pin_dshot);
    config.set_set_pins(&[&pin_dshot]);
    config.set_in_pins(&[&pin_dshot]);
    config.clock_divider = clock;
    config.shift_out.auto_fill = false;
    config.shift_out.threshold = 32;
    config.shift_out.direction = embassy_rp::pio::ShiftDirection::Left;

    config.shift_in.auto_fill = false;
    config.shift_in.threshold = 32;
    config.shift_in.direction = embassy_rp::pio::ShiftDirection::Left;

    config.fifo_join = embassy_rp::pio::FifoJoin::Duplex;

    // sm some fixen
    let mut sm_variant = b.available_sm.pop_front()?;
    sm_variant.init(&config, &pin_dshot);

    Some(PioDshotChannel::new(sm_variant))
}
