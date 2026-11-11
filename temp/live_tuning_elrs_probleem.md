# Live tuning via ELRS — probleem, context en advies

Datum: 2026-11-11  
Betrokken: `ground-station` (Tuning/0x7A/0x7B) + `rust_fc` (CRSF, PID_VALS, rate-loop)

## Context

- Live tuning gaat via **ELRS extended frames**: ground station stuurt `0x7A` met
  `TuningCmd::Postcard` (sub-id `0x7F00`), FC antwoordt met `0x7B` / sub-id `0x7F04`
  (`TuneAck`).
- Ping (`0x7F03`) werkt end-to-end; dat bewijst de PC → TX → lucht → RX → FC → terug-route.
- Ground station "Verstuur live" stuurt **4 frames** met 40 ms tussenruimte:
  3× `SetPid` (roll/pitch/yaw) + 1× `SetFilter`.
- De FC zet `PID_VALS[idx].set(...)` **direct bij ontvangst** in de CRSF-handler.
  De rate-loop doet `PID_VALS.get()` en `rate_pids.set_all(...)` alleen bij de
  **overgang naar Armed**. Filters worden alleen bij startup in de rate-loop geladen;
  `SetFilter` past ze daarna niet live aan.

## Symptomen

1. Arm-log laat roll/pitch wel de nieuwe 0.001 zien, maar yaw nog 0.25.
2. Ground station toont maar **1 TuneAck**, terwijl je 3 PID-acks + 1 filter-ack
   verwacht (dus 4).
3. Events in de RX-inspector verdwijnen na korte tijd, ook al staan radio/link uit.
4. FC-log toont bij vrijwel elke tuning-burst:
   `CRSF: PacketError: InvalidLength { len: 0 }`.
5. Geen `tunack dropped (channel full)` in de FC-log.

## Analyse

### 1. Arm-log vs UI (yaw nog oud)

- De UI is **geen readback**: de sliders tonen lokale form-state, niet `PID_VALS`.
  De enige FC-feedback is de laatste TunAck en de arm-log.
- `SetPid` wordt al direct in `receiver/crsf.rs` opgeslagen. Het probleem is dus
  niet "pas bij armen setten"; de rate-loop doet alleen de **get** bij de transitie.
- De 4 tuning-frames komen gespreid binnen (yaw ~80 ms, filter ~120 ms na de klik)
  plus ELRS-latency. Arm je tijdens die burst, dan is de arm-snapshot te vroeg:
  roll/pitch zijn al binnen, yaw nog niet.
- Extra effect: `receive()` retourneert zodra er een RC-frame is
  (`return Ok(map_channels(&ch))`). 0x7A-frames die daarna in de parser-buffer staan,
  worden pas bij de volgende `receive()` verwerkt. De rate-task (hogere prioriteit)
  kan dan al gelogd hebben.
- Gevolg: de arm-log kan yaw oud tonen terwijl de yaw-command milliseconden later
  alsnog wordt opgeslagen en ge-ackt. Bij de **volgende** arm-transitie zou yaw
  0.001 moeten zijn.
- Als yaw ook bij de volgende arm nog 0.25 is, is het yaw-frame onderweg verloren
  of niet verwerkt. Dan is het geen timing- maar een transport/protocolprobleem.

### 2. Maar 1 TuneAck i.p.v. 4

- De FC hoort voor "Verstuur live" **4 acks** te sturen: `as0`, `as1`, `as2` en
  een filter-ack.
- De Tuning-panel toont alleen de **laatste** `lastAck`; dat is per ontwerp 1 regel.
  Gebruik de RX-inspector met filter **TuneAck** om ze allemaal te zien.
- Maar de RX-inspector bewaart op dit moment ook de verborgen `radio`- en
  `link`-events in dezelfde 200-event-ring. Bij ~50 Hz `0x3A` + ~10 Hz `0x14`
  is 200 events maar ~3 s; de TuneAcks worden dus snel uit de buffer geduwd.
  Dat verklaart mede dat je er later nog maar 1 ziet.
- Daarnaast is er een **ELRS-downlinkbeperking**: de RX-telemetriequeue heeft voor
  extended frames maar een paar general-response-slots (in ELRS 3.x: twee slots,
  head-dropping). Vier snelle acks kunnen elkaar daar overschrijven vóór ze over
  de lucht gaan. De FC-log laat dan **geen** `tunack dropped` zien, want de drop
  gebeurt in de RX-telemetriequeue, niet in `TUNING_CHANNEL`.
- `TUNING_CHANNEL` heeft capaciteit 4; vier acks passen. De FC-drop-warning is dus
  waarschijnlijk niet de oorzaak. (Wel controleren of die warning echt nooit voorkomt.)
- Conclusie: 4 acks worden door de FC gemaakt, maar de combinatie van
  **RX-telemetrie-slots + 200-event-buffer** zorgt dat je er maar 1 ziet.
  Dit is zeer waarschijnlijk, niet 100% bewezen; sequentieel één command sturen
  en op de ack wachten is de test.

### 3. Events verdwijnen

- `useXboxBridge` bewaart alle `RxSight`s in één ring van 200:
  `setRx((r) => [...r.slice(-199), sight])`.
- Het typefilter in `RxPanel` werkt **na** die ring. Verborgen `radio`/`link`-events
  bezetten dus nog steeds de 200 plaatsen en duwen zichtbare ping/TuneAck-events eruit.
- Fixrichting: filter vóór de ring (alleen zichtbare types opslaan), of een ring
  per type, of in de backend alleen geselecteerde types sturen.

### 4. `InvalidLength { len: 0 }`

- De `crsf`-crate synct op elk adresbyte (`0xC8/0xEA/0xEC/0xEE`) en leest de
  volgende byte als lengte. `len == 0` is ongeldig (CRSF min. 2), dus de parser
  gooit 2 bytes weg en resynct.
- Dit gebeurt wanneer de parser één byte uit lijn raakt en op een `0xC8` in een
  **payload** landt die gevolgd wordt door `0x00`. In RC-/telemetrie-payloads
  komen veel `0x00`-bytes voor; de tuning-frames voegen extra frames en zeros toe.
- Waarom raakt hij uit lijn?
  - De RX schrijft telemetrie/MSP naar de FC met een **lengteprefix-byte** vóór
    elk frame (`SerialCRSF::forwardMessage` / `queueMSPFrameTransmission`).
  - De FC leest vaste blokken van 64 bytes en `receive()` retourneert bij het
    eerste RC-frame; de rest blijft in `PacketParser<128>` staan.
  - Bij drukte kan de parser achterlopen; `PacketParser` dropt bytes als de
    ring vol is. Eén gemiste byte → resync op een toevallig adresbyte.
- Het is dus een **resync-warning**, geen direct bewijs dat de tuning faalt.
  Maar het kan wél een TuneMsg kosten, en het maakt de tuning minder betrouwbaar.
- `len: 0` zegt niets over de luchtlink; het is parser-alignement aan de FC-UART.

## Belangrijke info

- `PID_VALS[idx].set(...)` gebeurt al direct in `receiver/crsf.rs`.
- `rate_pids.set_all(...)` gebeurt alleen bij `State::Armed`-transitie.
- `FILTER_CUTOFFS` wordt bij `SetFilter` opgeslagen en ge-ackt, maar:
  - `RatePID::set_all()` werkt alleen kp/ki/kd bij;
  - dterm-filters worden alleen bij startup uit `pid_configs()` gemaakt;
  - de gyro-cutoff uit `FILTER_CUTOFFS[0]` wordt nergens gebruikt
    (`GYRO_FILTER_CUTOFF_HZ = 80` is een constante).
  - `TuneAck { ok: true }` betekent dus "opgeslagen", niet "actief in de loop".
- `AtomicPid` bestaat uit drie losse `AtomicF32`-velden; `set()`/`get()` zijn niet
  atomisch als geheel. Een gelijktijdige lezer kan theoretisch een mix van oud/nieuw
  zien (torn read). In de getoonde log is yaw volledig oud, dus dat lijkt hier niet
  de oorzaak, maar het is een latent risico.
- De Tuning-UI heeft geen "read current PID"-commando; ze kan de FC-status dus niet
  hydrateren. Lokale sliderwaarden zijn geen bewijs van FC-state.

## Advies (prioriteit)

1. **Tuning sequentieel maken (request/ack).**
   - Stuur één `SetPid`/`SetFilter`, wacht op de bijbehorende `TuneAck` (met timeout
     en retry), en stuur pas dan de volgende.
   - Voeg een **sequence number** toe aan het tuning-frame en de ack, en houd per
     axis de laatste ack bij in de UI.
   - Waarom: dit voorkomt dat 4 snelle acks elkaar in de RX-telemetriequeue
     overschrijven en maakt "maar 1 ack" direct zichtbaar/diagnoseerbaar.
   - Als alternatief: één gecombineerd tuning-frame (alle assen + filter) en één
     ack; minder frames over de trage telemetrie-downlink.

2. **Eventbuffer scheiden van filters.**
   - Filter vóór de 200-ring, of gebruik een ring per `RxSightKind`.
   - Waarom: verborgen 0x3A/link-stats mogen je ping/TuneAck-historie niet opeten.
   - Dit maakt ook de TuneAck-inspectie betrouwbaar.

3. **PID-toepassing expliciet kiezen.**
   - Huidige situatie: set direct, get alleen bij arm-transitie. Dat is verdedigbaar,
     maar vereist dat alle tuning-acks binnen zijn vóór het armen.
   - Robuuster: zet bij ontvangst een `TUNE_DIRTY`-flag; laat de rate-loop die
     checken en de gains herladen (bijv. elke 8e tick of direct bij dirty).
     Doe dat alleen als disarmed (of gate op armed), passend bij het latere
     stream/armed-beleid.
   - Waarom: dan maakt de volgorde "tune vs arm" niet meer uit en is de UI-ack
     ook echt de actieve waarde.

4. **Filters: nu niet als live presenteren.**
   - Ofwel `SetFilter` echt live implementeren (dterm-cutoffs + gyro-cutoff in de
     lopende filters), ofwel de filtercontrols verbergen/dimmen en de ack-tekst
     "opgeslagen, niet actief" maken.
   - Waarom: nu wordt `ok: true` ge-ackt terwijl de gyro-cutoff dood is en de
     dterm-filters pas bij een herstart zouden veranderen. Dat is misleidend.

5. **FC-parser robuuster maken.**
   - `receive()` volledig laten leegparsen (laatste RC-frame onthouden en pas
     teruggeven als de parser leeg is), of `PacketParser`-capaciteit vergroten
     en/of kleinere UART-leesblokken gebruiken.
   - Waarom: minder `InvalidLength`-resyncs en minder kans dat een TuneMsg
     onderweg sneuvelt.
   - Optioneel: tellers voor goede/bad frames en een log bij resync, zodat je
     ziet of een tuning-frame daadwerkelijk is gedropt.

6. **Ack/print verbeteren.**
   - FC: log per `SetPid`/`SetFilter`: axis, requested, stored, `ok`.
   - FC: log bij arm per axis de laatste update (timestamp/revisie), niet alleen
     de momentopname.
   - UI: toon per axis de laatste ack en formatteer floats (0.0010 i.p.v.
     0.0010000000474974513).
   - Waarom: nu is de arm-log een momentopname zonder herkomst en toont de UI
     alleen de laatste ack; daardoor lijkt het sneller inconsistent dan het is.

## Snelle verificatie

1. Zet RX-inspector op **TuneAck** en wis de lijst.
2. Stuur **één** `SetPid` (bijv. alleen roll) en wacht op de ack; herhaal voor pitch
   en yaw en filter.
3. Als je sequentieel 4 acks ziet, is het probleem de burst/ELRS-telemetriequeue.
4. Disarm, wacht tot alle acks binnen zijn, arm opnieuw en vergelijk de arm-log.
5. Als yaw dan 0.001 is: het was de burst/arm-race. Als yaw 0.25 blijft: het
   yaw-frame gaat onderweg verloren (parser/ELRS) en moet met de sequence/ack-retry
   worden opgelost.
