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
- [x] `config.rs`: `DSHOT_SPEED_HZ` (nu nog `600_000` hardcoded in `dshot_pio.rs:115`)
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

## Notch (klaar)
- [x] `filters/notch.rs`: `Notch`-wrapper om de `biquad`-crate (DirectForm1, update-drempel 5%)
- [x] `filters/rpm_filter.rs`: `RpmFilter<SLOTS>` — MOTOR_COUNT × 2 harmonischen × 3 assen
- [x] rate loop: gyro → gyro_filter → notch → PID; eRPM → Hz via `MOTOR_POLE_PAIRS`
- [x] config: `MOTOR_POLE_PAIRS` + `MOTOR_COUNT` + `DSHOT_SPEED_HZ`
- [x] live test: `[NOTCH]` beweegt mee met de RPM (16-243 Hz) — vaste-frequentie-test overgeslagen (crate-formules zijn getest)

## Open na test (16 okt)
- [ ] IMU accel: raw-LSB test (motoren uit vs idle) — verwacht 1g (8192 LSB), nu ~5,5g bij draaiende motoren
- [ ] IMU mount: vaste rotatie (board-alignment) of fysiek draaien — Euler 149°/36° terwijl board plat ligt
- [ ] `[ACT]`-log: stale velden (`ML_raw`, `Motors [L, R]`) tonen 0 terwijl de mixer waarden stuurt
- [ ] tuning: param-commando's via USB (bank) → daarna ESB-dongle + RAM-blackbox + dynamic notch
