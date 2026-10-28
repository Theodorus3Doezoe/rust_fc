pub mod bi_copter;
pub mod tri_mixer;
use crate::config::MOTOR_IDLE;
use crate::types::Rates;

pub trait Mixer {
    type Output;
    fn mix(&self, throttle: f32, pid: Rates) -> Self::Output;
}

pub fn desaturate_motors<const N: usize>(mut motors: [f32; N]) -> [f32; N] {
    if N == 0 {
        return motors;
    }

    let mut min = motors[0];
    let mut max = motors[0];
    for &m in motors.iter().skip(1) {
        min = min.min(m);
        max = max.max(m);
    }

    let span = max - min;
    let allowed = 1.0 - MOTOR_IDLE;

    if span > allowed {
        let scale = allowed / span;
        for m in motors.iter_mut() {
            *m = MOTOR_IDLE + (*m - min) * scale;
        }
    } else if max > 1.0 {
        let offset = max - 1.0;
        for m in motors.iter_mut() {
            *m -= offset;
        }
    } else if min < MOTOR_IDLE {
        let offset = MOTOR_IDLE - min;
        for m in motors.iter_mut() {
            *m += offset;
        }
    }

    clamp_motors(&mut motors);
    motors
}

pub fn desaturate_servos<const N: usize>(mut servos: [f32; N]) -> [f32; N] {
    let peak = servos.iter().fold(0.0f32, |peak, &s| peak.max(s.abs()));
    if peak > 1.0 {
        for s in servos.iter_mut() {
            *s /= peak;
        }
    }
    clamp_servos(&mut servos);
    servos
}

fn clamp_motors(motors: &mut [f32]) {
    for m in motors.iter_mut() {
        *m = m.clamp(MOTOR_IDLE, 1.0);
    }
}

fn clamp_servos(servos: &mut [f32]) {
    for s in servos.iter_mut() {
        *s = s.clamp(-1.0, 1.0);
    }
}
