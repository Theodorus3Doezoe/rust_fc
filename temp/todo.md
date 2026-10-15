# Todo restructure

## 0. Fixen (klaar)
- [x] `boards/mod.rs`: `dshot::channel::MotorChannel` → `actuators::channel::MotorChannel`
- [x] `v_copter.rs`: `use crate::actuators::dshot::{frame, motor}` → `motor` uit `actuators::motor`
- [x] `v_copter.rs`: `apply(f32)` i.p.v. frames; `read_telemetry` → `latest_telemetry`; `stop_all` → `apply(0.0)`; `set_direction` → `command(cmd)`

## 1. Fase A (klaar)
- [x] `actuators/channel.rs`: generieke `MotorChannel` (f32, `latest_telemetry`, `esc_command`)
- [x] `actuators/motor.rs`: `MotorController<C>`
- [x] `frame.rs`: `throttle_to_dshot_value` met 0.0 = stop
- [x] `dshot_pio.rs`: `esc_command` (shift `<< 16`) + helper gebruikt

## 2. Fase B (klaar, op `DSHOT_SPEED_HZ` na)
- [x] `dshot/state.rs`: `EscLink` (`Signal<u16>` + 3× `AtomicU32`: raw + erpm + periode) + statics + `claim_link()`; `latest()` geeft `Option<DshotTelemetry>`
- [x] `dshot/driver.rs`: `EscDriver` trait (async `send_frame`, `read_pair`)
- [x] `dshot/dshot_channel.rs`: `DshotChannel { link }` impl `MotorChannel` (encode + telemetrie via link)
- [x] board: `PioEscDriver { sm }` impl `EscDriver`; `take_esc_driver()` = oude take_motor-logica zonder kanaal; `type MotorPin = DshotChannel`
- [x] `actuators/provider.rs`: `ActuatorProvider` uit `boards/` + `type Esc: EscDriver`; default `take_motor` = `claim_link()` + `DshotChannel`
- [x] `tasks/esc_telemetry.rs`: `link.wait → driver.send_frame → driver.read_pair → decode → link.publish`
- [x] `config.rs`: alias `esc::Driver = <ActiveBoard as ActuatorProvider>::Esc`
- [ ] `config.rs`: `DSHOT_SPEED_HZ` (nu nog `600_000` hardcoded in `dshot_pio.rs:115`)
- [x] `platform.rs` / `main.rs`: drivers ophalen, koppelen aan `ESC_LINKS` op volgorde, taak spawnen
- [x] `Frame::MOTOR_COUNT` const + itereren in `rate.rs` (geen hardcoded 2 meer bij consumenten)
- [x] `v_copter.rs`: `set_direction` `todo!()` eruit

## 3. Opruimen (warnings)
- [ ] `tasks/rate.rs`: unused imports (`ActiveBoard`, `ImuBurst`)
- [ ] `tasks/rx.rs`: unused `e`
- [ ] `sensors/calibrated_imu.rs`: unused `i`

## 4. Hardware test (FC aansluiten)
- [ ] flash + arm → `[ESC]` ML ~95%+ decode
- [ ] disarm: motor stopt echt (0 = stop, niet 48)
- [ ] TX loopt door zonder ESC-antwoord (MR blijft `None`)

## Later
- [ ] notch filter in `filters/notch.rs` (rate loop, niet in motor/channel)
