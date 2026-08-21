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

### 2. Uitgaande Commando's (`FromPc` berichten)
- **`arm`**: Stuurt het `FromPc::Arm` (`0x00`) commando naar de Flight Controller.
- **`disarm`**: Stuurt het `FromPc::Disarm` (`0x01`) commando naar de Flight Controller.

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
# Automatisch een USB seriele poort zoeken en verbinden
python3 tools/usb_tui.py

# Specifieke poort opgeven
python3 tools/usb_tui.py -p /dev/ttyACM0

# Simulatielogica starten (zonder fysieke hardware)
python3 tools/usb_tui.py --sim
```

**Sneltoetsen & Commando's in TUI:**
- **`a`**: Direct **ARM** commando verzenden.
- **`d`**: Direct **DISARM** commando verzenden (vervangt `q`).
- **`p`** / **`s`**: **Device Selection Menu** openen om van USB poort of naar de Simulator te wisselen.
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
