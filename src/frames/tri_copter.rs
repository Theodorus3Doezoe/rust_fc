use super::Frame;
use super::MAX_MOTORS;
use crate::actuators::motor::MotorController;
use crate::actuators::provider::ActuatorProvider;
use crate::actuators::servo::ServoController;
use crate::actuators::{dshot::frame, motor};
use crate::config::frame::ConcreteMotor;
use crate::config::frame::ServoPin;
use crate::config::{ActiveBoard, SERVO_FREQ_HZ};
use crate::frames::FrameOutput;
use crate::mixers::Mixer;
use crate::mixers::tri_mixer::TriMixer;
use embassy_time::{Duration, Timer};

pub struct TriCopterFrame<P, M> {
    servo: ServoController<P>,
    motor_left: motor::MotorController<M>,
    motor_right: motor::MotorController<M>,
    motor_back: motor::MotorController<M>,
    mixer: TriMixer,
}

impl Frame for TriCopterFrame<ServoPin, ConcreteMotor> {
    type Board = ActiveBoard;
    const MOTOR_COUNT: usize = 3;
    type Telemetry = frame::DshotTelemetry;

    fn init(provider: &mut Self::Board) -> Self {
        let servo_pin = provider.take_servo().expect("Could not take servo pin");
        let motor_left = provider
            .take_motor()
            .expect("Could not take left motor pin");
        let motor_right = provider
            .take_motor()
            .expect("Could not take right motor pin");
        let motor_back = provider
            .take_motor()
            .expect("Could not take rear motor pin");
        Self {
            servo: ServoController::new(servo_pin, 0.0, false, SERVO_FREQ_HZ),
            motor_left: MotorController::new(motor_left, false),
            motor_right: MotorController::new(motor_right, true),
            motor_back: MotorController::new(motor_back, false),
            mixer: TriMixer::init(),
        }
    }

    fn apply(&mut self, throttle: f32, pid: crate::types::Rates) -> FrameOutput {
        let mixer_out = self.mixer.mix(throttle, pid);
        let servo = self.servo.set_duty(mixer_out.servo_tail);

        self.motor_left.apply(mixer_out.motor_front_left);
        self.motor_right.apply(mixer_out.motor_front_right);
        self.motor_back.apply(mixer_out.motor_tail);

        FrameOutput {
            motors: [
                mixer_out.motor_front_left,
                mixer_out.motor_front_right,
                mixer_out.motor_tail,
                0.0,
            ],
            motor_count: 3,
            servo_us: [servo, 0, 0, 0],
            servo_count: 1,
        }
    }

    fn read_telemetry(&mut self) -> heapless::Vec<Option<Self::Telemetry>, MAX_MOTORS> {
        let mut v = heapless::Vec::new();
        let _ = v.push(self.motor_left.latest_telemetry());
        let _ = v.push(self.motor_right.latest_telemetry());
        let _ = v.push(self.motor_back.latest_telemetry());
        v
    }

    fn stop_all(&mut self) {
        let _ = self.servo.set_duty(0.0);

        self.motor_left.apply(0.0);
        self.motor_right.apply(0.0);
        self.motor_back.apply(0.0);
    }

    async fn set_direction(&mut self, permanent: bool) {
        let cmd_left = if self.motor_left.cw { 7 } else { 8 };
        let cmd_right = if self.motor_right.cw { 7 } else { 8 };
        let cmd_back = if self.motor_back.cw { 7 } else { 8 };
        for _ in 0..10 {
            self.motor_left.command(cmd_left);
            self.motor_right.command(cmd_right);
            self.motor_back.command(cmd_back);
            Timer::after(Duration::from_micros(100)).await;
        }

        if permanent {
            for _ in 0..10 {
                self.motor_left.command(12);
                self.motor_right.command(12);
                self.motor_back.command(12);
                Timer::after(Duration::from_micros(100)).await;
            }
        }
    }

    fn poll_telemetry(&mut self) -> heapless::Vec<Option<u64>, MAX_MOTORS> {
        let mut v = heapless::Vec::new();
        let _ = v.push(self.motor_left.try_recv_raw());
        let _ = v.push(self.motor_right.try_recv_raw());
        let _ = v.push(self.motor_back.try_recv_raw());
        v
    }
}
