// use crate::actuators::dshot::dshot_channel::MotorChannel;
use crate::actuators::dshot::driver::EscDriver;

use crate::boards::rp2350::Rp2350Dev;
use crate::config::DSHOT_SPEED_HZ;
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

macro_rules! wait_tx_space {
    ($sm:expr, $val:expr, $($variant:ident),* $(,)?) => {
        match $sm {
            $(MotorSm::$variant(sm) => sm.tx().wait_push($val).await,)*
        }
    };
}

macro_rules! wait_rx_word {
    ($sm:expr, $($variant:ident),* $(,)?) => {
        match $sm {
            $(MotorSm::$variant(sm) => sm.rx().wait_pull().await,)*
        }
    };
}

impl MotorSm {
    pub fn init(&mut self, config: &Config<'static, PIO0>, pin: &Pin<'static, PIO0>) {
        init_motor_sm!(self, config, pin, Sm0, Sm1, Sm2, Sm3);
    }

    pub async fn push_tx(&mut self, value: u32) {
        wait_tx_space!(self, value, Sm0, Sm1, Sm2, Sm3)
    }

    pub async fn read_word(&mut self) -> u32 {
        wait_rx_word!(self, Sm0, Sm1, Sm2, Sm3)
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

// impl MotorChannel for PioDshotChannel {
//     type Telemetry = DshotTelemetry;
//     type Error = PioDshotChannelError;
//
//     fn set_throttle(&mut self, throttle: f32) -> Result<(), Self::Error> {
//         let throttle = throttle_to_dshot_value(throttle);
//
//         let frame = create_dshot_frame(throttle, true);
//         self.sm.push_tx((frame as u32) << 16);
//         Ok(())
//     }
//
//     fn latest_telemetry(&mut self) -> Option<Self::Telemetry> {
//         self.sm.read_telemetry()
//     }
//     fn esc_command(&mut self, cmd: u8) -> Result<(), Self::Error> {
//         let frame = create_dshot_frame(cmd as u16, true);
//         self.sm.push_tx((frame as u32) << 16);
//         Ok(())
//     }
// }

pub struct PioEscDriver {
    sm: MotorSm,
}

impl EscDriver for PioEscDriver {
    async fn send_frame(&mut self, frame: u16) {
        self.sm.push_tx((frame as u32) << 16).await;
    }

    async fn read_pair(&mut self) -> (u32, u32) {
        (self.sm.read_word().await, self.sm.read_word().await)
    }
}

pub fn take_esc_driver(b: &mut Rp2350Dev) -> Option<PioEscDriver> {
    let pio_program = pio_file!(
        "src/boards/rp2350/pio/bidir_dshot.pio",
        options(max_program_size = 32)
    );
    let dshot_speed = DSHOT_SPEED_HZ;
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
    let loaded_program = b.motor_program.as_ref().expect("Motor program not loaded");
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
    let mut sm = b.available_sm.pop_front()?;
    sm.init(&config, &pin_dshot);

    Some(PioEscDriver { sm })
}
