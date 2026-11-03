# Backlog (later, niet testvlucht-blokkerend)

## Flashen via USB (zonder losse probe)
- RP2350 heeft USB-bootloader: BOOTSEL ingedrukt houden + reset → `RPI-RP2`-drive → `firmware.uf2` erop slepen
- Geen probe nodig voor flashen; wel nog voor defmt-RTT-logs (of later vervangen door USB-CDC-logging)
- Check: `picotool` kan ook reboot-naar-BOOTSEL en UF2 laden vanaf WSL zodra USB ge-attachd is

## Interrupt-executors
- `executor-interrupt`-feature staat al aan in `Cargo.toml`, maar alles draait nu nog op de thread-executor
- Plan: rate-loop (8 kHz) naar interrupt-executor met hoge prioriteit, de rest (rx/usb/telemetry) op thread-mode
- Let op: shared state (`AtomicF32`/`SYSTEM`) blijft werken, maar blokkeer-niks in interrupt-context (geen `await` op langdurige dingen)

## Logger-abstractie
- Eén trait (bv `Logger`: `log_attitude()`, `log_esc()`, `log_event()`) met meerdere sinks erachter:
  - defmt/RTT (nu, debuggen via probe)
  - USB-CDC (live naar TUI)
  - blackbox (RAM/flash-ringbuffer, dumpen na vlucht)
  - CRSF-telemetrie terug over ELRS (later, via geparkeerde tx-helft)
- Doel: call-sites (`rate.rs`, `attitude.rs`) loggen één keer, backlog-items kiezen per sink aan/uit zonder die code aan te raken
