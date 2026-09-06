use super::PwmPinConcrete;
use super::Rp2350Dev;
use embassy_rp::pwm::Config as PwmConf;
use fixed::traits::ToFixed;

pub fn take_servo(b: &mut Rp2350Dev) -> Option<PwmPinConcrete> {
    // let frame decide config parameters, or servo or something?
    let mut pwm_conf = PwmConf::default();
    pwm_conf.divider = 15.to_fixed();
    pwm_conf.top = 39_999;
    pwm_conf.compare_a = 15_000;
    pwm_conf.compare_b = 15_000;
    pwm_conf.enable = true;

    // check pending_servo
    if let Some(servo) = b.pending_servo.take() {
        defmt::info!("Took pending servo");
        return Some(servo);
    }

    // take available_slices, slice them up, return a and put b in pending
    let next_slice = b.available_slices.pop_front()?;
    defmt::info!("Popped servo slice from available_slices");

    let (servo_a, servo_b) = next_slice.init(pwm_conf);

    defmt::info!("Returning servo pins");
    b.pending_servo = Some(servo_b);
    Some(servo_a)
}
