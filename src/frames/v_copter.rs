use crate::actuators::provider::ActuatorProvider;
use crate::actuators::servo::ServoController;
use crate::actuators::{dshot::frame, motor};
use crate::config::frame::{MotorPin, ServoPin};
use crate::config::{ActiveBoard, SERVO_FREQ_HZ};
use crate::mixers::bi_copter::{self, BiCopMixer, BicopterOutput};
use embassy_time::{Duration, Timer};

use super::*;

pub struct VCopterFrame<P, M> {
    servo_left: ServoController<P>,
    servo_right: ServoController<P>,
    motor_left: motor::MotorController<M>,
    motor_right: motor::MotorController<M>,
    mixer: BiCopMixer,
}

#[derive(Clone, Copy, defmt::Format)]
pub struct ActuatorOutput {
    pub servo_left_us: u16,
    pub servo_right_us: u16,
    pub ml_raw_to_dshot: u16,
    pub mr_raw_to_dshot: u16,
    pub motor_left_throttle: u16,
    pub motor_right_throttle: u16,
}

impl Frame for VCopterFrame<ServoPin, MotorPin> {
    type Board = ActiveBoard;
    // type ServoPin = B::ServoPin;
    // type MotorPin = B::MotorPin;
    const MOTOR_COUNT: usize = 2;
    type Telemetry = frame::DshotTelemetry;

    fn init(provider: &mut Self::Board) -> Self {
        let left_servo_pin = provider
            .take_servo()
            .expect("Frame: Could not take left servo pin from board");

        let right_servo_pin = provider
            .take_servo()
            .expect("Frame: Could not take right servo pin from board");

        let left_motor_pin = provider
            .take_motor()
            .expect("Frame: Could not take left servo pin from board");

        let right_motor_pin = provider
            .take_motor()
            .expect("Frame: Could not take right servo pin from board");

        defmt::info!("[Frame] : Succesfully initialised Actuators");
        Self {
            servo_left: ServoController::new(left_servo_pin, 0.0, false, SERVO_FREQ_HZ),
            servo_right: ServoController::new(right_servo_pin, 0.0, false, SERVO_FREQ_HZ),

            motor_left: motor::MotorController::new(left_motor_pin, true),
            motor_right: motor::MotorController::new(right_motor_pin, false),
            // temp hard coded
            mixer: bi_copter::BiCopMixer::init(0.0, 0.0),
        }
    }

    fn apply(&mut self, throttle: f32, pid: Rates) -> FrameOutput {
        // use inner.mixer
        let mixer_out: BicopterOutput = self.mixer.mix(throttle, pid);

        let servo_left_us = self.servo_left.set_duty(mixer_out.servo_left);
        let servo_right_us = self.servo_right.set_duty(mixer_out.servo_right);

        // motors
        // let left = frame::mixer_to_dshot_throttle(mixer_out.motor_left);
        // let right = frame::mixer_to_dshot_throttle(mixer_out.motor_right);
        //
        // let motor_left_throttle = frame::create_dshot_frame(left, true);
        // let motor_right_throttle = frame::create_dshot_frame(right, true);
        //
        self.motor_left.apply(mixer_out.motor_left);
        self.motor_right.apply(mixer_out.motor_right);

        let info = ActuatorOutput {
            servo_left_us,
            servo_right_us,
            ml_raw_to_dshot: 0,
            mr_raw_to_dshot: 0,
            motor_left_throttle: 0,
            motor_right_throttle: 0,
        };

        //for telemetry
        FrameOutput {
            motors: [mixer_out.motor_left, mixer_out.motor_right, 0.0, 0.0],
            motor_count: 2,
            servo_us: [servo_left_us, servo_right_us, 0, 0],
            servo_count: 2,
        }
    }

    fn read_telemetry(&mut self) -> heapless::Vec<Option<Self::Telemetry>, MAX_MOTORS> {
        let mut v = heapless::Vec::new();
        let _ = v.push(self.motor_left.latest_telemetry());
        let _ = v.push(self.motor_right.latest_telemetry());
        v
    }

    fn stop_all(&mut self) -> FrameOutput {
        let servo_left_us = self.servo_left.set_duty(0.0);
        let servo_right_us = self.servo_right.set_duty(0.0);

        self.motor_left.apply(0.0);
        self.motor_right.apply(0.0);

        FrameOutput {
            motors: [0.0, 0.0, 0.0, 0.0],
            motor_count: 2,
            servo_us: [servo_left_us, servo_right_us, 0, 0],
            servo_count: 2,
        }
    }

    async fn set_direction(&mut self, permanent: bool) {
        let cmd_left = if self.motor_left.cw { 7 } else { 8 };
        let cmd_right = if self.motor_right.cw { 7 } else { 8 };
        for _ in 0..10 {
            self.motor_left.command(cmd_left);
            self.motor_right.command(cmd_right);
            Timer::after(Duration::from_micros(100)).await;
        }

        if permanent {
            for _ in 0..10 {
                self.motor_left.command(12);
                self.motor_right.command(12);
                Timer::after(Duration::from_micros(100)).await;
            }
        }
    }
}
