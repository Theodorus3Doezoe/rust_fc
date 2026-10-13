# Visie: restructure actuators / ESC

## Doelen
- Board kent geen DShot: alleen hardware (SM, pins, FIFO's).
- Rate loop await nooit: geen FIFO-waits of telemetrie-lezen in de 8 kHz loop.
- Alle protocolkennis (encoding, CRC, decode) op één plek: `actuators/dshot/`.
- Naamgeving: "channel" = control-kant, "driver" = hardware-kant.

## Lagen

```
v_copter → MotorController<C> → MotorChannel (trait, f32 0..1)
                                    │
                                    │ DshotChannel (bord-vrij): mixerwaarde → frame
                                    ▼
                                  EscLink   ← postbus (heen: Signal, terug: atomen)
                                    ▼
              esc_task: wait → driver.send_frame → driver.read_pair → decode → publish
                                    ▼
                                  EscDriver (trait) ← PioEscDriver (bezit de SM)
```

| Laag | Rol | Kent DShot | Kent HW |
|---|---|---|---|
| `MotorChannel` | control-contract (f32) | nee | nee |
| `DshotChannel` | DShot-impl van MotorChannel | ja | nee |
| `EscLink` | postbus rate loop ↔ taak | nee | nee |
| `EscDriver` | driver-contract (async, ruwe bits) | nee | nee |
| `PioEscDriver` | SM-wrapper (push / read) | nee | ja |
| `esc_task` | brug + decode → eRPM | decode | nee |

## Flow
- Heen: mixer f32 → `DshotChannel` (0 = stop, 1..47 = commando, 48..2047 = throttle) → frame (CRC + tlm-bit) → `link.send()` → taak → SM → ESC.
- Terug: ESC → SM rx → `read_pair()` → decode → `link.publish(eRPM, periode)` → `latest_telemetry()` → frame / USB.

## Waarom zo
- Rate loop: `send()` = `Signal::set` (non-blocking), `latest()` = één atomic load. Geen awaits, geen jitter.
- De taak wacht async op FIFO-ruimte én op het ESC-antwoord (~22 µs na frame-einde); dat mag de rate loop niet doen.
- De SM heeft één eigenaar (de taak via de driver); de rate loop raakt de SM niet meer aan.
- Ander board (STM32/DMA) = eigen `EscDriver`; `DshotChannel` en de taak blijven gelijk.

## Opstarten
- `platform`: board init → `frame::init(&mut board)` → daarna drivers uit de doos halen + `ESC_LINKS[k]` koppelen → `esc_task` spawnen.
- `take_motor()` (default op `ActuatorProvider`, verhuist naar `actuators/provider.rs`): board zet de SM klaar (in zijn doos), default pakt de volgende link (`claim_link()`) en geeft `DshotChannel { link }` aan het frame.
- Links: `[EscLink; N]` statics + `AtomicUsize`-teller; beide kanten krijgen `&'static EscLink`.

## Status (13 okt)
- Klaar (fase A, gecommit): generieke `MotorChannel` (f32) + `MotorController`; `PioDshotChannel` op de nieuwe trait; 0 = stop; `esc_command`.
- Fase B gestart: stap 1 = `EscLink` in `state.rs`; zie `temp/todo.md` sectie 2.
