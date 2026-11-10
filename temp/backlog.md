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

## FFT-notch (Betaflight-stijl dynamic notch)
- Bestaande RPM-notch volgt ESC-toerental; deze analyseert het **gyrosignaal zelf**: `no_std`-FFT (bv `microfft`) op gedecimeerde gyrodata + piekdetectie → notches op de luidste pieken
- Voorwaarde: eerst blackbox (anders niet te valideren), draaien als aparte laag-frequente task naast de 8kHz-loop (te zwaar voor in de loop zelf)
- Daarna pas: statische frame-notch op gemeten resonanties kan eventueel vervallen als de FFT ze meepakt

## Web-TUI (postcard over WebSerial)
- Protocol blijft `postcard` (geen JSON): `FromPc`-tuning + `ToPc`-telemetrie, website praat via WebSerial direct met de FC
- JS-codec voor postcard nabouwen (vibe-code), live PID-tuning + VQF/attitude-visualisatie, geen herflash-flow (flashen blijft handmatig via BOOTSEL/UF2)

## ELRS-Lua-menu voor in het veld (geparkeerd)
- Pas relevant als tunen zonder laptop nodig blijkt; USB/web-TUI heeft voorrang (meer werk voor slechtere UI)
- Vereist: CRSF-telemetrie + device-discovery + parameter-lezen/schrijven op de FC, plus Lua-script voor het EdgeTX-menu
- Bestaande ELRS-Lua (link-instellingen) gewoon blijven gebruiken, dat kost niks

## Hoogte-modus: left stick = omhoog/omlaag i.p.v. throttle (later)
- In Acro kan dit niet zinvol: zonder hoogtemeting is "omhoog" gewoon throttle met een ander jasje (handmatig blijven corrigeren)
- Echte altitude-hold vereist een hoogtesensor: barometer (absoluut, buiten goed / binnen rumoerig) of ToF (precies 0–2m, binnen ideaal) — hardware staat nog niet op het board
- Als de sensor er is: cascaded regeling (hoogte → verticale snelheid → throttle) als extra `FlightMode` naast Angle/Rate; stick wordt dan klimsnelheid i.p.v. gasstand
