use crate::actuators::motor::MotorController;
use crate::actuators::servo::ServoController;
use crate::boards::ActuatorProvider;
use crate::config::frame::{MotorPin, ServoPin};
use crate::config::{ActiveBoard, SERVO_FREQ_HZ};
use crate::helpers::dshot::{create_dshot_frame, mixer_to_dshot_throttle};
use crate::mixers::bi_copter::{self, BiCopMixer, BicopterOutput};
use embassy_time::{Duration, Timer};

use super::*;

pub struct VCopterFrame<P, M> {
    servo_left: ServoController<P>,
    servo_right: ServoController<P>,
    motor_left: MotorController<M>,
    motor_right: MotorController<M>,
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

pub struct FrameOutput {
    pub mixer: BicopterOutput,
    pub actuators: ActuatorOutput,
}

impl Frame for VCopterFrame<ServoPin, MotorPin> {
    type Board = ActiveBoard;
    // type ServoPin = B::ServoPin;
    // type MotorPin = B::MotorPin;
    type Mixer = self::BiCopMixer;

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

            motor_left: MotorController::new(left_motor_pin, true),
            motor_right: MotorController::new(right_motor_pin, false),
            // temp hard coded
            mixer: bi_copter::BiCopMixer::init(5.0, 0.0, 0.0),
        }
    }

    fn apply(&mut self, throttle: f32, pid: Rates) -> FrameOutput {
        // use inner.mixer
        let mixer_out: BicopterOutput = self.mixer.mix(throttle, pid);

        let servo_left_us = self.servo_left.set_duty(mixer_out.servo_left);
        let servo_right_us = self.servo_right.set_duty(mixer_out.servo_right);

        // motors
        let left = mixer_to_dshot_throttle(mixer_out.motor_left);
        let right = mixer_to_dshot_throttle(mixer_out.motor_right);

        let motor_left_throttle = create_dshot_frame(left as u16, false);
        let motor_right_throttle = create_dshot_frame(right as u16, false);

        self.motor_left.apply(motor_left_throttle);
        self.motor_right.apply(motor_right_throttle);

        let info = ActuatorOutput {
            servo_left_us,
            servo_right_us,
            ml_raw_to_dshot: left,
            mr_raw_to_dshot: right,
            motor_left_throttle,
            motor_right_throttle,
        };

        //for telemetry
        FrameOutput {
            mixer: mixer_out,
            actuators: info,
        }
    }

    fn stop_all(&mut self) {
        let _ = self.servo_left.set_duty(0.0);
        let _ = self.servo_right.set_duty(0.0);

        let zero = create_dshot_frame(0, false);
        self.motor_left.apply(zero);
        self.motor_right.apply(zero);

        // self.motor_left.apply(0);
        // self.motor_right.apply(0);
    }

    // async fn arm_motor(&mut self) {
    //     let idle_throttle = self.mixer.get_idle_throttle();
    //     let idle_dshot = mixer_to_dshot_throttle(idle_throttle);
    //     self.motor_left.apply(0);
    //     self.motor_right.apply(0);
    //     Timer::after(Duration::from_millis(300)).await;
    //     // send idle throttle
    //     self.motor_left.apply(idle_dshot);
    //     self.motor_right.apply(idle_dshot);
    //     Timer::after(Duration::from_millis(500)).await;
    //     self.motor_left.apply(0);
    //     self.motor_right.apply(0);
    //     Timer::after(Duration::from_millis(300)).await;
    // }

    async fn set_direction(&mut self, permanent: bool) {
        let cmd_left = if self.motor_left.cw { 7 } else { 8 };
        let cmd_right = if self.motor_right.cw { 7 } else { 8 };
        for _ in 0..10 {
            self.motor_left.apply(cmd_left);
            self.motor_right.apply(cmd_right);
            Timer::after(Duration::from_micros(100)).await;
        }

        if permanent {
            for _ in 0..10 {
                self.motor_left.apply(12);
                self.motor_right.apply(12);
                Timer::after(Duration::from_micros(100)).await;
            }
        }
        todo!()
    }
}
