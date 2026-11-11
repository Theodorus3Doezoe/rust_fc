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

## Actueel plan – testvlucht (28 okt, samen afgaan)
Doel: eerst build groen, dan tri-frame, dan ELRS. Dit lijstje werken we samen af.

### A. Build groen (klaar, alleen warnings over)
- [x] `src/frames/v_copter.rs`: `apply()` ombouwen naar nieuwe `FrameOutput {motors, motor_count, servo_us, servo_count}` (nu nog oude `{mixer, actuators}`)
- [x] `src/tasks/rate.rs:149-154`: logging ombouwen naar nieuwe `FrameOutput`-velden
- [x] `cargo check` schoon (nu 8x E0609/E0560)
- [ ] warnings weg: `tri_copter.rs:10-11` unused imports, `rx.rs:50` unused `e`, `calibrated_imu.rs:36` unused `i`

### B. Tri-frame (code klaar, bench-test open)
- [x] `src/frames/tri_copter.rs`: rename `VCopterFrame` → `TriCopterFrame`, `MOTOR_COUNT 2 → 3`
- [x] `apply/read_telemetry/stop_all/set_direction` implementeren (nu `todo!()`)
- [x] `config.rs`: `ActiveFrame` op tri zetten voor test
- [ ] bench zonder props: arm → `[ESC] ML ~95%+`, disarm = stop, failsafe bij RX-verlies

### C. ELRS (bezig — link bewezen, status 6 nov)
Bewezen: 2× LilyGO **V1** (geen V2!), ELRS 3.6.4, EU868, phrase sven_rx, LQ 100%, RSSI ≈ −40 dBm.
Let op: V1 → RST = GPIO14 (geen 23); laptop→TX-baud werkend gekregen (was 921600-notitie, bleek baud-mismatch met ruis als gevolg).
- [x] LilyGO-RX flashen als DIY TTGO **V1** 900MHz RX + binden (LQ 100%)
- [x] LilyGO-TX flashen als DIY TTGO **V1** 900MHz TX, CRSF op USB (3/1), handshake + OLED ok
- [x] RcChannels: `arm/disarm` → één `armed: bool`, `rx.rs` mee omgezet
- [x] Board-UART: `UartRead`/`UartWrite`-traits, UART0/PIN_0/PIN_1 + DMA_CH2/CH3, `take_crsf_uart()` met `split()` (tx geparkeerd voor later)
- [x] `uart.rs`: trait-impls voor `UartTx`/`UartRx`-helften (`cargo check` weer groen)
- [x] `src/receiver/crsf.rs`: parser met `crsf`-crate (`PacketParser`, `RcChannels`-mapping, `CrsfError`) — `cargo check` groen
- [x] `receiver::Concrete` omzetten naar CRSF + wiring in `platform.rs` (tx-helft geparkeerd in `Platform`)
- [x] USB-gelijk trekken: `tools/postcard_codec.py` + `usb_tui.py` naar `armed` (18B-pakket, Rust-kant volgt automatisch via derive)
- [ ] **Check:** RX→FC-baud (FC-UART staat op 420000 — verifiëren wat de RX uitstuurt)
- [ ] **Check:** arm-kanaal/polariteit met RadioMaster (B0?) + pitch-teken + stick-bereik
- [ ] Tuning/telemetrie-transport: zie nieuw plan hieronder (D) — wordt ELRS extended frames, niet USB

### D. Live tuning over ELRS extended frames (nieuw, vervangt USB-tuning als eerstvolgende)
Ontwerp bewezen in source, contract vastgelegd. Regels: alleen disarmed + bench, Model Match uit, link vereist, 5 s-timeout als vangnet. Normaal vliegen = nul tuning-overhead.
- [x] `crsf.rs`: `next_raw_packet()` i.p.v. `next_packet()` + dispatch op type-byte (`0x16` via `from_raw`, rest negeren)
- [x] `0x7A`: CRC-check handmatig (DVB-S2 over `data[2..len-1]`), sub-id als `u16`LE, `TuningCmd`-enum (`0x7F00–0x7F03`)
- [x] `0x7F00`: postcard-tunnel — `TunMsg`-enum (`SetPid`/`SetFilter`, FC leidend, dashboard volgt); `Arm`/`Disarm` bestaan niet (armen loopt via CH9)
- [x] gain-statics: `AtomicPid` + `PID_VALS[3]` + `FILTER_CUTOFFS[3]`, defaults uit `config::pid`; `SetPid`/`SetFilter`-armen schrijven ze weg
- [x] rate-loop leest live mee: helper bouwt `PidConfig`s uit statics, `set_all()` per tick (iedere 8e tick mag ook)
- [x] `0x7F03` ping: payload (≤8B, seq+timestamp+random door dashboard) → `Signal`-handoff voor echo
- [x] `0x7F01`/`0x7F02`: STREAM-`AtomicBool` aan/uit (+ rate-byte, default 10, clamp 1–10); LAST_MSP-`AtomicU32` bij elk `0x7A`-frame
- [ ] `0x1E`-attitude (RPY i16LE, rad×10000) + `0x7B`-antwoorden (ping-echo; SetPid-ack later) de UART op via `platform.crsf_tx`
- [ ] nieuwe `telemetry_task` (10Hz-ticker, alleen disarmed + link vers < 5s); spawnen in `main.rs`
- [ ] cutoff-live-pad op `GyroFilter` + `DtermFilter` (`set_cutoff`, coeffs herberekenen)
- [ ] verificatie-volgorde: ping heen-en-weer (bewijst ook adres-byte terugweg) → attitude → PID-tuning → pas daarna bereik-test open veld
- [ ] limiet accepteren: telemetrie terug is enkele frames/sec (trage horizon ~1–5 Hz); ratio omhoog = RC-rate omlaag
- [ ] **Later:** live tuning persistent opslaan in flash (nu alleen RAM — na reboot alles kwijt); laden bij boot, expliciet save-commando, defaults als fallback. Vastgelegd formaat:
  ```rust
  struct SavedConfig {
      magic: u32,      // 0x54494E47
      versie: u16,
      gains: [f32; 9], // 3 assen × kp/ki/kd
      filters: [f32; 2],
      crc32: u32,      // crc over alles hierboven, excl. zichzelf
  }
  ```
