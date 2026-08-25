# Flight Controller USB CLI & TUI Tool

Een Python Terminal User Interface (TUI) en Command Line Interface (CLI) voor interactie met de Flight Controller via USB Serial (`postcard` wire protocol).

Ondersteunt alle commando's en datatypes die zijn gedefinieerd in [`src/usb.rs`](../src/usb.rs).

---

## 🚀 Capabilities

### 1. Inkomende Telemetrie (`ToPc` berichten)
- **`Attitude`**: Roll (°), Pitch (°), Yaw (°) met visuele horizon indicator bars.
- **`SystemState`**: Status naam (`DISARMED`, `ARMED`, etc.), Arm blocks bitmasker (`0x00000000`) en error count.
- **`Ack`**: Ontvangstbevestigingen van het board na een verzonden commando.
- **`Log`**: Real-time logberichten afkomstig van het Flight Controller board.

### 3. Xbox One Controller FlightStick Besturing
- **Right Stick X-as**: **Roll** (genormaliseerd van `-1.0` [links] tot `+1.0` [rechts]).
- **Left Stick Y-as**: **Pitch** (genormaliseerd van `-1.0` [achter] tot `+1.0` [vooruit]).
- **LT / RT Triggers**: **Yaw** (LT = negatieve yaw tot `-1.0`, RT = positieve yaw tot `+1.0`).
- **D-Pad Omhoog / Omlaag**: **Throttle** (Cruise control stijl: verhoogt/verlaagt gasniveau tussen `0.0` en `1.0`).
- **A Knop**: Verzendt het **`ARM`** commando naar de Flight Controller.
- **B Knop**: Verzendt het **`DISARM`** commando naar de Flight Controller.

### 4. 🎛️ Live PID & Filter Tuning
- **Roll PID**: `tune roll <kp> <ki> <kd>` (bijv. `tune roll 1.2 0.05 0.15`)
- **Pitch PID**: `tune pitch <kp> <ki> <kd>` (bijv. `tune pitch 1.4 0.05 0.18`)
- **Yaw PID**: `tune yaw <kp> <ki> <kd>` (bijv. `tune yaw 2.0 0.10 0.05`)
- **Gyro Filter Cutoff**: `tune gyro <hz>` (bijv. `tune gyro 100`)
- **D-Term Filter Cutoff**: `tune dterm <hz>` (bijv. `tune dterm 40`)

---

## 🛠️ Installatie & Benodigdheden

Zorg dat de Python dependencies zijn geïnstalleerd:

```bash
pip install -r tools/requirements.txt
```

---

## 🖥️ Gebruik

### 1. Interactieve TUI Mode
Start de volledige Terminal UI:

```bash
# Met automatische Xbox controller detectie & USB Flight Controller:
python3 tools/usb_tui.py

# Xbox Controller + Serial simulatie inschakelen (zonder hardware):
python3 tools/usb_tui.py --xbox-sim --sim
```

**Sneltoetsen & Commando's in TUI:**
- **`a`** (of **A-knop op Xbox Controller**): Verzend **ARM** commando.
- **`d`** (of **B-knop op Xbox Controller**): Verzend **DISARM** commando.
- **`t`**: Open het **PID & Filter Tuning Menu**.
- **`x`**: Schakel **Xbox Controller Simulator** in/uit.
- **`p`** / **`s`**: **Device Selection Menu** openen.
- **`h`**: **Help** informatie tonen.
- **`c`**: **Clear** logvenster.
- `Ctrl+C` of typ `quit` : Sluit de applicatie.

---

### 2. Directe CLI Commando's
Voer eenmalige commando's rechtstreeks uit via de terminal:

```bash
# Systeemstatus opvragen
python3 tools/usb_tui.py status

# ARM commando sturen naar USB poort
python3 tools/usb_tui.py arm -p /dev/ttyACM0

# DISARM commando sturen
python3 tools/usb_tui.py disarm -p /dev/ttyACM0

# Beschikbare seriële poorten tonen
python3 tools/usb_tui.py --list
```

---

## 📁 Bestanden in `tools/`

- [`usb_tui.py`](file:///home/svend/rust_fc/tools/usb_tui.py): De hoofd-CLI/TUI applicatie.
- [`postcard_codec.py`](file:///home/svend/rust_fc/tools/postcard_codec.py): Pure Python implementatie van het Postcard wire protocol voor `ToPc` en `FromPc`.
- [`requirements.txt`](file:///home/svend/rust_fc/tools/requirements.txt): Python dependencies (`pyserial`, `prompt-toolkit`, `rich`).
- [`README.md`](file:///home/svend/rust_fc/tools/README.md): Documentatie.
