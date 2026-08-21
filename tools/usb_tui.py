#!/usr/bin/env python3
"""
CLI & TUI tool for monitoring Flight Controller telemetry and sending commands over USB Serial.
Supports commands defined in `src/usb.rs`: `arm` and `disarm`.
"""

import sys
import os
import time
import argparse
import threading
import math
from datetime import datetime
from typing import Optional, List

import serial
import serial.tools.list_ports

from prompt_toolkit.application import Application
from prompt_toolkit.application.current import get_app
from prompt_toolkit.completion import WordCompleter
from prompt_toolkit.key_binding import KeyBindings
from prompt_toolkit.layout.containers import HSplit, VSplit, Window
from prompt_toolkit.layout.controls import FormattedTextControl
from prompt_toolkit.layout.layout import Layout
from prompt_toolkit.widgets import Frame, TextArea
from prompt_toolkit.styles import Style

# Import local postcard codec
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import postcard_codec as codec
from postcard_codec import Attitude, SystemState, Ack, Log, FromPcCommand


class SerialInterface:
    """Manages serial communication or simulated telemetry."""

    def __init__(self, port: Optional[str] = None, baud: int = 115200, sim: bool = False):
        self.port = port
        self.baud = baud
        self.sim = sim
        self.ser: Optional[serial.Serial] = None
        self.is_connected = False
        self.running = False
        self.rx_count = 0
        self.tx_count = 0
        self.error_count = 0

        # State storage
        self.latest_attitude: Optional[Attitude] = Attitude(0.0, 0.0, 0.0)
        self.latest_state: Optional[SystemState] = SystemState(state=1, arm_blocks=0, errors=0) # 1=DISARMED
        self.log_messages: List[str] = []

        # Callback for UI notification
        self.on_update_callback = None

    def log(self, text: str):
        timestamp = datetime.now().strftime("%H:%M:%S")
        entry = f"[{timestamp}] {text}"
        self.log_messages.append(entry)
        if len(self.log_messages) > 100:
            self.log_messages.pop(0)
        if self.on_update_callback:
            self.on_update_callback()

    def start(self):
        self.running = True
        if self.sim:
            self.log("Starting in SIMULATION mode...")
            self.is_connected = True
            threading.Thread(target=self._sim_worker, daemon=True).start()
        else:
            threading.Thread(target=self._serial_worker, daemon=True).start()

    def stop(self):
        self.running = False
        if self.ser and self.ser.is_open:
            try:
                self.ser.close()
            except Exception:
                pass
        self.is_connected = False

    def change_device(self, target: str):
        """Switch serial port or simulation mode dynamically."""
        self.stop()
        time.sleep(0.1)
        if target.lower() in ("sim", "simulator", "3"):
            self.sim = True
            self.port = None
            self.log("[CONN] Switched to SIMULATOR mode")
        else:
            self.sim = False
            self.port = target
            self.log(f"[CONN] Switched to device port: {target}")
        self.start()

    def send_cmd(self, cmd_name: str) -> bool:
        cmd_name = cmd_name.strip().lower()
        if cmd_name == "arm":
            payload = FromPcCommand.encode_arm()
            label = "ARM"
        elif cmd_name == "disarm":
            payload = FromPcCommand.encode_disarm()
            label = "DISARM"
        else:
            self.log(f"[ERROR] Unknown command: '{cmd_name}'")
            return False

        if self.sim:
            self.tx_count += 1
            self.log(f"[TX] Sent command: {label}")
            # In simulation, respond with Ack and state change after a short delay
            def sim_respond():
                time.sleep(0.05)
                # Ack message
                self.rx_count += 1
                self.log(f"[RX] Received ToPc::Ack for {label}")
                if label == "ARM":
                    if self.latest_state:
                        self.latest_state.state = 2 # ARMED
                        self.latest_state.arm_blocks = 0
                    self.log("[SYS] System State changed to ARMED")
                elif label == "DISARM":
                    if self.latest_state:
                        self.latest_state.state = 1 # DISARMED
                    self.log("[SYS] System State changed to DISARMED")
                if self.on_update_callback:
                    self.on_update_callback()

            threading.Thread(target=sim_respond, daemon=True).start()
            return True

        if not self.ser or not self.ser.is_open:
            self.log(f"[ERROR] Cannot send {label}: Serial port not connected")
            return False

        try:
            self.ser.write(payload)
            self.ser.flush()
            self.tx_count += 1
            self.log(f"[TX] Sent {label} ({payload.hex()})")
            return True
        except Exception as e:
            self.error_count += 1
            self.log(f"[ERROR] Failed to send {label}: {e}")
            return False

    def _serial_worker(self):
        buf = bytearray()
        while self.running:
            if not self.is_connected:
                if not self.port:
                    self.port = auto_detect_port()
                if self.port:
                    try:
                        self.ser = serial.Serial(self.port, self.baud, timeout=0.1)
                        self.is_connected = True
                        self.log(f"[CONN] Connected to {self.port} at {self.baud} baud")
                    except Exception as e:
                        self.is_connected = False
                        self.log(f"[CONN] Waiting for serial port ({self.port})... ({e})")
                        time.sleep(1.5)
                        continue
                else:
                    self.log("[CONN] Searching for serial port (or pass --sim / -p PORT)...")
                    time.sleep(2.0)
                    continue

            try:
                raw = self.ser.read(64)
                if raw:
                    buf.extend(raw)
                    while buf:
                        try:
                            msg, consumed = codec.decode_to_pc_packet(bytes(buf))
                            if msg is None or consumed == 0:
                                break
                            buf = buf[consumed:]
                            self.rx_count += 1
                            self._handle_msg(msg)
                        except ValueError as ve:
                            # Realign by discarding first byte
                            buf.pop(0)
                            self.error_count += 1
                else:
                    time.sleep(0.01)
            except Exception as e:
                self.error_count += 1
                self.log(f"[ERROR] Serial read error: {e}")
                self.is_connected = False
                if self.ser:
                    try:
                        self.ser.close()
                    except Exception:
                        pass
                time.sleep(1.0)

    def _handle_msg(self, msg: codec.ToPcMessage):
        if isinstance(msg, Attitude):
            self.latest_attitude = msg
        elif isinstance(msg, SystemState):
            self.latest_state = msg
            self.log(f"[RX] SystemState: state={msg.state_name}({msg.state}), arm_blocks={msg.arm_blocks}, errors={msg.errors}")
        elif isinstance(msg, Ack):
            self.log("[RX] ToPc::Ack received!")
        elif isinstance(msg, Log):
            self.log(f"[FC LOG] {msg.message}")

        if self.on_update_callback:
            self.on_update_callback()

    def _sim_worker(self):
        """Generates realistic synthetic telemetry in simulation mode."""
        t = 0.0
        log_counter = 0
        self.log("[SIM] Flight Controller boot sequence complete.")
        self.log("[SIM] Telemetry broadcast active.")

        while self.running:
            time.sleep(0.05) # 20Hz update
            t += 0.05
            roll = math.sin(t * 1.5) * 5.0
            pitch = math.cos(t * 1.2) * 3.0
            yaw = (t * 10.0) % 360.0

            self.latest_attitude = Attitude(roll=roll, pitch=pitch, yaw=yaw)
            self.rx_count += 1

            if int(t * 20) % 20 == 0: # 1Hz
                state_val = self.latest_state.state if self.latest_state else 1
                self.latest_state = SystemState(state=state_val, arm_blocks=0, errors=0)

            log_counter += 1
            if log_counter >= 100: # Every 5s
                log_counter = 0
                self.log(f"[SIM LOG] Periodic FC pulse t={t:.1f}s")

            if self.on_update_callback:
                self.on_update_callback()


def auto_detect_port() -> Optional[str]:
    """Find serial port matching USB vendor/product or common ttyACM/ttyUSB names."""
    ports = serial.tools.list_ports.comports()
    for p in ports:
        # Match USB VID:PID 0xc0de:0xcafe
        if p.vid == 0xC0DE and p.pid == 0xCAFE:
            return p.device
        if "MyDrone" in (p.manufacturer or "") or "FlightController" in (p.product or ""):
            return p.device
    for p in ports:
        if "ttyACM" in p.device or "ttyUSB" in p.device or "COM" in p.device:
            return p.device
    return None


def list_ports():
    """Print available serial ports."""
    ports = serial.tools.list_ports.comports()
    if not ports:
        print("No serial ports found.")
        return
    print("Available serial ports:")
    for p in ports:
        vid_pid = f"{p.vid:04x}:{p.pid:04x}" if p.vid and p.pid else "N/A"
        desc = p.description or "No description"
        print(f"  - {p.device:<15} [{vid_pid}] {desc}")


def run_tui(serial_if: SerialInterface):
    """Launch the interactive TUI using prompt_toolkit."""

    # UI State controls
    header_control = FormattedTextControl()
    att_control = FormattedTextControl()
    sys_control = FormattedTextControl()
    log_control = FormattedTextControl()

    command_completer = WordCompleter(
        ["arm", "disarm", "clear", "help", "quit", "exit"],
        ignore_case=True
    )

    input_field = TextArea(
        height=1,
        prompt="fc> ",
        completer=command_completer,
        multiline=False,
        focus_on_click=True
    )

    def render_horizon_bar(value: float, max_val: float = 30.0, width: int = 20) -> str:
        norm = max(-1.0, min(1.0, value / max_val))
        pos = int((norm + 1.0) / 2.0 * (width - 1))
        bar = ["="] * width
        bar[pos] = "O"
        half = width // 2
        bar[half] = "|" if bar[half] != "O" else "O"
        return "".join(bar)

    def update_header():
        conn_str = "SIMULATOR" if serial_if.sim else (serial_if.port or "AUTO-SEARCH")
        status_color = "ansigreen" if serial_if.is_connected else "ansired"
        status_text = "CONNECTED" if serial_if.is_connected else "DISCONNECTED"

        return [
            ("class:title", " 🚁 RUST_FC USB TERMINAL & TELEMETRY MONITOR "),
            ("", " | "),
            ("class:label", "Port: "),
            ("class:value", f"{conn_str} "),
            ("", "| "),
            ("class:label", "Status: "),
            (f"class:{status_color}", f"{status_text} "),
            ("", "| "),
            ("class:label", "RX: "),
            ("class:value", f"{serial_if.rx_count} "),
            ("", "| "),
            ("class:label", "TX: "),
            ("class:value", f"{serial_if.tx_count} "),
            ("", "| "),
            ("class:label", "ERR: "),
            ("class:value", f"{serial_if.error_count} "),
        ]

    def update_attitude():
        att = serial_if.latest_attitude
        if not att:
            return [("class:dim", "Waiting for Attitude data...")]

        r_bar = render_horizon_bar(att.roll)
        p_bar = render_horizon_bar(att.pitch)

        return [
            ("class:label", f"  Roll:  "),
            ("class:value", f"{att.roll:+7.2f}° "),
            ("class:bar", f" [{r_bar}]\n"),
            ("class:label", f"  Pitch: "),
            ("class:value", f"{att.pitch:+7.2f}° "),
            ("class:bar", f" [{p_bar}]\n"),
            ("class:label", f"  Yaw:   "),
            ("class:value", f"{att.yaw:7.2f}°\n"),
        ]

    def update_system():
        sys_st = serial_if.latest_state
        if not sys_st:
            return [("class:dim", "Waiting for SystemState data...")]

        state_color = "ansigreen" if sys_st.state == 2 else ("ansiyellow" if sys_st.state == 1 else "ansired")

        return [
            ("class:label", "  State Name: "),
            (f"class:{state_color}", f"{sys_st.state_name:<12} "),
            ("class:label", "State ID: "),
            ("class:value", f"{sys_st.state}\n"),
            ("class:label", "  Arm Blocks: "),
            ("class:value", f"0x{sys_st.arm_blocks:08X}\n"),
            ("class:label", "  Error Code: "),
            ("class:value", f"{sys_st.errors}\n"),
        ]

    def update_log():
        lines = serial_if.log_messages[-25:]
        formatted = []
        for line in lines:
            if "[RX]" in line or "Ack" in line:
                formatted.append(("ansigreen", line + "\n"))
            elif "[TX]" in line:
                formatted.append(("ansicyan", line + "\n"))
            elif "[ERROR]" in line or "[FAIL]" in line:
                formatted.append(("ansired", line + "\n"))
            elif "[FC LOG]" in line:
                formatted.append(("ansiyellow", line + "\n"))
            else:
                formatted.append(("", line + "\n"))
        return formatted if formatted else [("class:dim", "No log events yet.\n")]

    def refresh_ui():
        header_control.text = update_header()
        att_control.text = update_attitude()
        sys_control.text = update_system()
        log_control.text = update_log()
        try:
            get_app().invalidate()
        except Exception:
            pass

    serial_if.on_update_callback = refresh_ui

    current_ports_cache = []

    def show_device_menu():
        nonlocal current_ports_cache
        ports = serial.tools.list_ports.comports()
        current_ports_cache = ports
        serial_if.log("================ 🔌 DEVICE SELECTION MENU ================")
        if not ports:
            serial_if.log("  No physical serial ports found.")
        else:
            for idx, p in enumerate(ports, 1):
                vid_pid = f" ({p.vid:04X}:{p.pid:04X})" if p.vid and p.pid else ""
                desc = p.description or ""
                serial_if.log(f"  [{idx}] {p.device:<16} {vid_pid} {desc}")
        sim_idx = len(ports) + 1
        serial_if.log(f"  [{sim_idx}] SIMULATOR        (Simulated Flight Controller)")
        serial_if.log("---------------------------------------------------------")
        serial_if.log(f"Type 'connect <nr>' or number (1-{sim_idx}) to select device.")
        refresh_ui()

    def show_help():
        serial_if.log("Available Hotkeys & Commands:")
        serial_if.log("  [a] / arm     - Send ARM command to Flight Controller")
        serial_if.log("  [d] / disarm  - Send DISARM command to Flight Controller")
        serial_if.log("  [p] / device  - Open Device/Port selection menu")
        serial_if.log("  [h] / help    - Display this help message")
        serial_if.log("  [c] / clear   - Clear event log box")
        serial_if.log("  quit / exit   - Exit the TUI application")
        refresh_ui()

    command_completer = WordCompleter(
        ["arm", "disarm", "device", "port", "connect", "clear", "help", "quit", "exit"],
        ignore_case=True
    )

    input_field = TextArea(
        height=1,
        prompt="fc> ",
        completer=command_completer,
        multiline=False,
        focus_on_click=True
    )

    # Keybindings
    kb = KeyBindings()

    @kb.add("c-c")
    def _exit(event):
        event.app.exit()

    @kb.add("a")
    @kb.add("A")
    def _hotkey_arm(event):
        serial_if.send_cmd("arm")
        refresh_ui()

    @kb.add("d")
    @kb.add("D")
    def _hotkey_disarm(event):
        serial_if.log("🚨 DISARM TRIGGERED VIA 'D' KEY!")
        serial_if.send_cmd("disarm")
        refresh_ui()

    @kb.add("p")
    @kb.add("P")
    @kb.add("s")
    @kb.add("S")
    def _hotkey_device_menu(event):
        show_device_menu()

    @kb.add("h")
    @kb.add("H")
    def _hotkey_help(event):
        show_help()

    @kb.add("c")
    @kb.add("C")
    def _hotkey_clear(event):
        serial_if.log_messages.clear()
        refresh_ui()

    def handle_command(buf):
        text = input_field.text.strip()
        input_field.text = ""
        if not text:
            return

        cmd = text.lower()
        parts = cmd.split()

        if cmd in ("quit", "exit"):
            get_app().exit()
            return
        elif cmd in ("c", "clear"):
            serial_if.log_messages.clear()
            refresh_ui()
            return
        elif cmd in ("h", "help"):
            show_help()
            return
        elif cmd in ("p", "s", "port", "device", "menu"):
            show_device_menu()
            return
        elif cmd in ("a", "arm"):
            serial_if.send_cmd("arm")
        elif cmd in ("d", "disarm"):
            serial_if.send_cmd("disarm")
        elif parts[0] in ("connect", "select", "use") and len(parts) > 1:
            target = parts[1]
            if target.isdigit():
                idx = int(target)
                ports = current_ports_cache or serial.tools.list_ports.comports()
                if 1 <= idx <= len(ports):
                    serial_if.change_device(ports[idx - 1].device)
                elif idx == len(ports) + 1:
                    serial_if.change_device("sim")
                else:
                    serial_if.log(f"[ERROR] Invalid device number: {idx}")
            else:
                serial_if.change_device(target)
        elif cmd.isdigit():
            idx = int(cmd)
            ports = current_ports_cache or serial.tools.list_ports.comports()
            if 1 <= idx <= len(ports):
                serial_if.change_device(ports[idx - 1].device)
            elif idx == len(ports) + 1:
                serial_if.change_device("sim")
            else:
                serial_if.log(f"[ERROR] Invalid device number: {idx}")
        else:
            serial_if.log(f"Unknown command '{text}'. Press 'h' for help or 'p' for device menu.")

    input_field.accept_handler = handle_command

    # Layout assembly
    root_layout = HSplit([
        Frame(Window(content=header_control, height=1), title="Flight Controller Connection"),
        VSplit([
            HSplit([
                Frame(Window(content=att_control, height=5), title="Attitude (AHRS)"),
                Frame(Window(content=sys_control, height=5), title="System State"),
            ], width=44),
            Frame(Window(content=log_control), title="Event & Packet Log"),
        ]),
        Frame(
            input_field,
            title="Hotkeys: [a] Arm | [d] Disarm | [p] Devices | [h] Help | [c] Clear | [quit] Exit"
        ),
    ])

    style = Style.from_dict({
        "title": "bold cyan",
        "label": "bold white",
        "value": "bold yellow",
        "bar": "green",
        "dim": "gray",
        "frame.border": "cyan",
    })

    app = Application(
        layout=Layout(root_layout, focused_element=input_field),
        key_bindings=kb,
        style=style,
        full_screen=True,
        refresh_interval=0.1
    )

    serial_if.start()
    try:
        refresh_ui()
        app.run()
    finally:
        serial_if.stop()


def main():
    parser = argparse.ArgumentParser(
        description="Rust FC USB Serial CLI & TUI tool (supported commands: arm, disarm)"
    )
    parser.add_argument("-p", "--port", type=str, help="Serial port device (e.g. /dev/ttyACM0 or COM3)")
    parser.add_argument("-b", "--baud", type=int, default=115200, help="Baud rate (default: 115200)")
    parser.add_argument("-s", "--sim", action="store_true", help="Enable simulation mode (no hardware required)")
    parser.add_argument("-l", "--list", action="store_true", help="List available serial ports and exit")
    parser.add_argument("command", nargs="?", choices=["arm", "disarm", "status"], help="Optional single command to execute")

    args = parser.parse_args()

    if args.list:
        list_ports()
        return

    if args.command:
        # Non-interactive CLI mode
        port = args.port or auto_detect_port()
        ser_if = SerialInterface(port=port, baud=args.baud, sim=args.sim)
        ser_if.start()

        if args.command == "status":
            print(f"Connecting to {port or 'Simulator'}...")
            time.sleep(1.0)
            if ser_if.latest_attitude:
                print(f"Attitude: Roll={ser_if.latest_attitude.roll:.2f}, Pitch={ser_if.latest_attitude.pitch:.2f}, Yaw={ser_if.latest_attitude.yaw:.2f}")
            if ser_if.latest_state:
                print(f"System State: {ser_if.latest_state.state_name} (State ID: {ser_if.latest_state.state})")
        else:
            print(f"Sending '{args.command}' command...")
            time.sleep(0.2)
            success = ser_if.send_cmd(args.command)
            time.sleep(0.5)
            if success:
                print(f"Command '{args.command}' sent successfully.")
            else:
                print(f"Failed to send command '{args.command}'.")
        ser_if.stop()
        return

    # Interactive TUI mode
    ser_if = SerialInterface(port=args.port, baud=args.baud, sim=args.sim)
    run_tui(ser_if)


if __name__ == "__main__":
    main()
