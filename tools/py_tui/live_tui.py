#!/usr/bin/env python3
import argparse
import csv
import curses
import math
import os
import re
import select
import sys
import time
from datetime import datetime
import serial
import serial.tools.list_ports

# Ingestelde logmap naast live_tui.py
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
LOGS_DIR = os.path.join(SCRIPT_DIR, "logs")

# Duratie opties in minuten (None = Onbeperkt)
DURATION_OPTIONS = [None, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 15.0, 30.0]


def get_available_ports():
    """Haal beschikbare seriële poorten op."""
    ports = []
    try:
        for p in serial.tools.list_ports.comports():
            desc = p.description if p.description != "n/a" else "Seriële poort"
            ports.append((p.device, desc))
    except Exception:
        pass

    default_devs = ["/dev/ttyACM0", "/dev/ttyACM1", "/dev/ttyUSB0", "/dev/ttyUSB1"]
    existing_devices = [p[0] for p in ports]
    for dev in default_devs:
        if os.path.exists(dev) and dev not in existing_devices:
            ports.append((dev, "USB Serial Device"))

    if not ports:
        ports.append(("/dev/ttyACM1", "Standaard logger (niet gedetecteerd)"))

    return ports


def find_default_port(user_device_arg=None):
    """Vind automatisch de beste poort (bij voorkeur 'USB-serial logger' of /dev/ttyACM1)."""
    if user_device_arg is not None:
        dev_str = str(user_device_arg)
        if dev_str.isdigit():
            return f"/dev/ttyACM{dev_str}"
        elif not dev_str.startswith("/dev/"):
            return f"/dev/{dev_str}"
        return dev_str

    ports = get_available_ports()

    # 1. Zoek naar een poort met 'logger' in de omschrijving
    for dev, desc in ports:
        if "logger" in desc.lower() or "usb-serial logger" in desc.lower():
            return dev

    # 2. Kies /dev/ttyACM1 als die aanwezig is
    for dev, desc in ports:
        if dev == "/dev/ttyACM1":
            return dev

    if ports:
        return ports[0][0]

    return "/dev/ttyACM1"


def generate_log_filename(custom_name=None, allow_overwrite=False):
    """Genereer een logische bestandsnaam met automatische nummering (_1, _2, ...) voor zowel custom namen als stempelnamen."""
    os.makedirs(LOGS_DIR, exist_ok=True)

    if not custom_name or not custom_name.strip():
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        base_name = f"imu_{timestamp}"
    else:
        base_name = custom_name.strip()
        if base_name.lower().endswith(".csv"):
            base_name = base_name[:-4]

    if allow_overwrite:
        return f"{base_name}.csv"

    # 1. Probeert eerst base_name.csv
    first_candidate = f"{base_name}.csv"
    if not os.path.exists(os.path.join(LOGS_DIR, first_candidate)):
        return first_candidate

    # 2. Als base_name.csv al bestaat, probeer base_name_1.csv, base_name_2.csv, ...
    first_num_candidate = f"{base_name}_1.csv"
    if not os.path.exists(os.path.join(LOGS_DIR, first_num_candidate)):
        return first_num_candidate

    counter = 2
    while True:
        candidate = f"{base_name}_{counter}.csv"
        if not os.path.exists(os.path.join(LOGS_DIR, candidate)):
            return candidate
        counter += 1


VALID_KNOWN_KEYS = {
    "accel_x", "accel_y", "accel_z",
    "gyro_x", "gyro_y", "gyro_z",
    "burst_duration", "total_duration", "max_duration", "task_duration",
    "ax", "ay", "az", "gx", "gy", "gz", "temp"
}


def is_valid_key(k):
    """Controleer of een sleutel geldig is en negeer verminkte seriële fragmenten (zoals acceaccel_x of ro_x)."""
    k_clean = k.strip()
    if k_clean in VALID_KNOWN_KEYS:
        return True
    import re
    if re.match(r'^(accel_[xyz]|gyro_[xyz]|burst_duration|total_duration|max_duration|task_duration|temp|ax|ay|az|gx|gy|gz)$', k_clean):
        return True
    return False


def parse_value_with_unit(v_str):
    """Converteer getallen met of zonder eenheden (zoals '123.4', '250us', '1.2ms') naar float."""
    v_clean = v_str.strip()
    try:
        return float(v_clean)
    except ValueError:
        pass
    import re
    m = re.match(r"^([0-9.\-+eE]+)\s*([a-zA-Zµ]+)?$", v_clean)
    if m:
        try:
            val = float(m.group(1))
            unit = (m.group(2) or "").lower()
            if unit in ("us", "µs"):
                return val
            elif unit == "ms":
                return val * 1000.0
            elif unit == "s":
                return val * 1000000.0
            elif unit == "ns":
                return val / 1000.0
            return val
        except ValueError:
            pass
    return None


class ImuTuiApp:
    def __init__(self, stdscr, initial_device=None, baud=115200, duration=None, log_input=None, burst_limit=None, total_limit=None):
        self.stdscr = stdscr
        self.baud = baud
        self.duration_idx = 0
        if duration is not None:
            for i, opt in enumerate(DURATION_OPTIONS):
                if opt == duration:
                    self.duration_idx = i
                    break

        self.port = find_default_port(initial_device)

        self.custom_log_name = log_input or ""
        self.allow_overwrite = False

        # Limieten in microseconden (µs)
        self.burst_limit = burst_limit
        self.total_limit = total_limit
        self.overrun_count = 0

        self.ser = None
        self.ser_error = None

        self.recording = False
        self.record_start_time = 0.0
        self.stop_reason = ""
        self.log_file = None
        self.csv_writer = None
        self.current_log_path = ""
        self.header_written = False
        self.session_keys = []

        self.latest_parsed = {}
        self.history = {}

        # Gecachte lognaam om 25 FPS schijf-I/O vertraging te voorkomen
        self.cached_log_name = ""

        # Modals: None, 'DEVICE_SELECT', 'NAME_INPUT', 'LIMITS_INPUT', 'SUMMARY'
        self.active_modal = None
        self.modal_cursor = 0
        self.modal_ports = []
        self.input_text = ""
        self.limits_field = 0  # 0 voor burst, 1 voor total
        self.burst_input_str = str(burst_limit) if burst_limit else ""
        self.total_input_str = str(total_limit) if total_limit else ""

        self.init_curses()
        self.update_log_name_cache()
        self.connect_serial()

    def update_log_name_cache(self):
        """Update de gecachte logbestandsnaam alleen wanneer de instellingen veranderen."""
        try:
            self.cached_log_name = generate_log_filename(self.custom_log_name, self.allow_overwrite)
        except Exception:
            self.cached_log_name = "imu_log.csv"

    def init_curses(self):
        curses.curs_set(0)
        try:
            curses.mousemask(0)  # Schakel muisklikken uit om vastlopen in de terminal te voorkomen
        except Exception:
            pass
        self.stdscr.nodelay(True)
        self.stdscr.timeout(40)
        curses.use_default_colors()

        if curses.has_colors():
            curses.start_color()
            curses.init_pair(1, curses.COLOR_CYAN, -1)     # Header / Titels
            curses.init_pair(2, curses.COLOR_GREEN, -1)    # Waarden / Status OK
            curses.init_pair(3, curses.COLOR_YELLOW, -1)   # Waarschuwingen / IDLE
            curses.init_pair(4, curses.COLOR_RED, -1)      # Opname / Overrun actief
            curses.init_pair(5, curses.COLOR_BLACK, curses.COLOR_CYAN) # Menu selectie
            curses.init_pair(6, curses.COLOR_WHITE, curses.COLOR_BLUE) # Modal header

    def connect_serial(self):
        if self.ser and self.ser.is_open:
            try:
                self.ser.close()
            except Exception:
                pass
        self.ser = None
        self.ser_error = None
        try:
            self.ser = serial.Serial(self.port, self.baud, timeout=0.01)
        except Exception as e:
            self.ser_error = str(e)

    def start_recording(self):
        os.makedirs(LOGS_DIR, exist_ok=True)

        name = generate_log_filename(self.custom_log_name, self.allow_overwrite)
        self.current_log_path = os.path.join(LOGS_DIR, name)
        try:
            file_exists_and_not_empty = os.path.exists(self.current_log_path) and os.path.getsize(self.current_log_path) > 0
            mode = "w" if (self.allow_overwrite or not file_exists_and_not_empty) else "a"
            self.log_file = open(self.current_log_path, mode, newline="")
            self.csv_writer = csv.writer(self.log_file)
            self.header_written = (mode == "a" and file_exists_and_not_empty)
        except Exception as e:
            self.ser_error = f"Log fout: {e}"
            return

        self.history = {}
        self.session_keys = []
        self.overrun_count = 0
        self.record_start_time = time.time()
        self.recording = True
        self.stop_reason = ""

    def stop_recording(self, reason="Handmatig gestopt"):
        if not self.recording:
            return
        self.recording = False
        self.stop_reason = reason

        if self.log_file and self.csv_writer and self.history:
            try:
                expected_keys = [
                    "accel_x", "accel_y", "accel_z",
                    "gyro_x", "gyro_y", "gyro_z",
                    "burst_duration", "total_duration", "task_duration",
                    "ax", "ay", "az",
                    "gx", "gy", "gz"
                ]
                keys = [k for k in expected_keys if k in self.history] + sorted([k for k in self.history if k not in expected_keys])

                self.csv_writer.writerow([])
                self.csv_writer.writerow(["# STATISTIEKEN SAMENVATTING"])
                if self.burst_limit or self.total_limit:
                    lim_str = f"# LIMIETEN -> burst_limit: {self.burst_limit or 'Geen'} us, total_limit: {self.total_limit or 'Geen'} us | Overruns: {self.overrun_count}"
                    self.csv_writer.writerow([lim_str])
                self.csv_writer.writerow(["statistiek"] + keys)

                mins = ["MIN"]
                maxs = ["MAX"]
                means = ["GEMIDDELDE_MEAN"]
                mads = ["GEM_AFWIJKING_MAD"]
                stdevs = ["STDEV"]

                for k in keys:
                    vals = self.history[k]
                    if vals:
                        n = len(vals)
                        v_min = min(vals)
                        v_max = max(vals)
                        mean = sum(vals) / n
                        mad = sum(abs(v - mean) for v in vals) / n
                        stdev = math.sqrt(sum((v - mean) ** 2 for v in vals) / n)

                        mins.append(f"{v_min:.6f}")
                        maxs.append(f"{v_max:.6f}")
                        means.append(f"{mean:.6f}")
                        mads.append(f"{mad:.6f}")
                        stdevs.append(f"{stdev:.6f}")
                    else:
                        mins.append("")
                        maxs.append("")
                        means.append("")
                        mads.append("")
                        stdevs.append("")

                self.csv_writer.writerow(means)
                self.csv_writer.writerow(mins)
                self.csv_writer.writerow(maxs)
                self.csv_writer.writerow(mads)
                self.csv_writer.writerow(stdevs)
                self.log_file.flush()
            except Exception:
                pass

        if self.log_file:
            try:
                self.log_file.close()
            except Exception:
                pass
            self.log_file = None
            self.csv_writer = None

        self.update_log_name_cache()
        self.active_modal = "SUMMARY"

    def read_serial(self):
        if not self.ser or not self.ser.is_open:
            return

        try:
            while self.ser.in_waiting > 0:
                raw_line = self.ser.readline().decode("utf-8", errors="ignore")
                # Verwijder ANSI escape codes en carriage returns
                line = re.sub(r'\x1b\[[0-9;]*[a-zA-Z]', '', raw_line)
                line = line.replace('\r', ' ').strip()
                if not line:
                    continue

                parts = line.split()
                parsed = {}
                for part in parts:
                    if ":" in part:
                        k, v = part.split(":", 1)
                        k = k.strip()
                        if not is_valid_key(k):
                            continue
                        val = parse_value_with_unit(v)
                        if val is not None:
                            parsed[k] = val

                if not parsed:
                    continue

                self.latest_parsed.update(parsed)

                # Controleer overruns
                if "burst_duration" in parsed and self.burst_limit and parsed["burst_duration"] > self.burst_limit:
                    self.overrun_count += 1
                if "total_duration" in parsed and self.total_limit and parsed["total_duration"] > self.total_limit:
                    self.overrun_count += 1

                # Houd bekende keys bij voor consistente CSV kolommen
                for k in parsed.keys():
                    if k not in self.session_keys:
                        self.session_keys.append(k)

                if self.recording:
                    for k, v in parsed.items():
                        if k not in self.history:
                            self.history[k] = []
                        self.history[k].append(v)

                    if self.csv_writer:
                        if not self.header_written:
                            self.csv_writer.writerow(["timestamp"] + self.session_keys)
                            self.header_written = True
                        # Gebruik nieuwste bekende waarden van self.latest_parsed zodat er geen lege cellen in de CSV ontstaan
                        row = [datetime.now().isoformat()] + [self.latest_parsed.get(k, "") for k in self.session_keys]
                        self.csv_writer.writerow(row)
                        self.log_file.flush()

        except Exception as e:
            self.ser_error = f"Seriële leesfout: {e}"

    def run(self):
        while True:
            self.read_serial()

            target_duration = DURATION_OPTIONS[self.duration_idx]
            if self.recording and target_duration is not None:
                elapsed = time.time() - self.record_start_time
                if elapsed >= target_duration * 60.0:
                    self.stop_recording(f"Duratie ({target_duration} m) verstreken")

            self.draw()

            ch = self.stdscr.getch()
            if ch != -1:
                if not self.handle_input(ch):
                    break

        if self.recording:
            self.stop_recording("Applicatie afgesloten")

    def handle_input(self, ch):
        if self.active_modal == "DEVICE_SELECT":
            return self.handle_device_modal_input(ch)
        elif self.active_modal == "NAME_INPUT":
            return self.handle_name_modal_input(ch)
        elif self.active_modal == "LIMITS_INPUT":
            return self.handle_limits_modal_input(ch)
        elif self.active_modal == "SUMMARY":
            return self.handle_summary_modal_input(ch)

        # Globale hotkeys
        if ch in (ord("q"), ord("Q")):
            return False

        elif ch in (ord("d"), ord("D")):
            self.modal_ports = get_available_ports()
            self.modal_cursor = 0
            self.active_modal = "DEVICE_SELECT"

        elif ch in (ord("n"), ord("N")):
            self.input_text = self.custom_log_name
            self.active_modal = "NAME_INPUT"

        elif ch in (ord("l"), ord("L")):
            self.burst_input_str = str(self.burst_limit) if self.burst_limit else ""
            self.total_input_str = str(self.total_limit) if self.total_limit else ""
            self.limits_field = 0
            self.active_modal = "LIMITS_INPUT"

        elif ch in (ord("a"), ord("A")):
            self.allow_overwrite = not self.allow_overwrite
            self.update_log_name_cache()

        elif ch in (ord("s"), ord("S"), ord(" "), 10, 13):
            if self.recording:
                self.stop_recording("Handmatig gestopt")
            else:
                self.start_recording()

        elif ch in (curses.KEY_UP, ord("k")):
            if self.duration_idx < len(DURATION_OPTIONS) - 1:
                self.duration_idx += 1

        elif ch in (curses.KEY_DOWN, ord("j")):
            if self.duration_idx > 0:
                self.duration_idx -= 1

        elif ch in (ord("c"), ord("C")):
            self.history = {}
            self.overrun_count = 0

        return True

    def handle_device_modal_input(self, ch):
        if ch in (curses.KEY_UP, ord("k")):
            if self.modal_cursor > 0:
                self.modal_cursor -= 1
        elif ch in (curses.KEY_DOWN, ord("j")):
            if self.modal_cursor < len(self.modal_ports) - 1:
                self.modal_cursor += 1
        elif ch in (10, 13, ord(" ")):
            if self.modal_ports:
                selected_port = self.modal_ports[self.modal_cursor][0]
                self.port = selected_port
                self.connect_serial()
            self.active_modal = None
        elif ch in (27, ord("q"), ord("Q")):
            self.active_modal = None
        return True

    def handle_name_modal_input(self, ch):
        if ch in (10, 13):
            self.custom_log_name = self.input_text.strip()
            self.update_log_name_cache()
            self.active_modal = None
        elif ch in (27,):
            self.active_modal = None
        elif ch in (curses.KEY_BACKSPACE, 127, 8):
            self.input_text = self.input_text[:-1]
        elif ch in (ord("a"), ord("A")) and len(self.input_text) == 0:
            self.allow_overwrite = not self.allow_overwrite
            self.update_log_name_cache()
        elif 32 <= ch <= 126:
            self.input_text += chr(ch)
        return True

    def handle_limits_modal_input(self, ch):
        if ch in (9, curses.KEY_DOWN, curses.KEY_UP, ord("j"), ord("k")):  # Tab of Pijltjes wisselt veld
            self.limits_field = 1 - self.limits_field
        elif ch in (10, 13):  # Enter bevestigt
            try:
                self.burst_limit = float(self.burst_input_str) if self.burst_input_str.strip() else None
            except ValueError:
                self.burst_limit = None
            try:
                self.total_limit = float(self.total_input_str) if self.total_input_str.strip() else None
            except ValueError:
                self.total_limit = None
            self.active_modal = None
        elif ch in (27,):  # Esc
            self.active_modal = None
        elif ch in (curses.KEY_BACKSPACE, 127, 8):
            if self.limits_field == 0:
                self.burst_input_str = self.burst_input_str[:-1]
            else:
                self.total_input_str = self.total_input_str[:-1]
        elif (48 <= ch <= 57) or ch == 46:  # Cijfers en punt
            if self.limits_field == 0:
                self.burst_input_str += chr(ch)
            else:
                self.total_input_str += chr(ch)
        return True

    def handle_summary_modal_input(self, ch):
        if ch in (10, 13, 27, ord(" "), ord("q"), ord("Q"), ord("c"), ord("C")):
            self.active_modal = None
        return True

    def draw(self):
        self.stdscr.erase()
        height, width = self.stdscr.getmaxyx()

        if height < 15 or width < 60:
            try:
                self.stdscr.addstr(0, 0, "Scherm te klein! Vergoot je terminal venster.", curses.A_BOLD)
            except curses.error:
                pass
            self.stdscr.refresh()
            return

        # 1. Header Banner
        title = " 📡 IMU LIVE TUI & DATA LOGGER "
        try:
            self.stdscr.attron(curses.color_pair(1) | curses.A_BOLD)
            self.stdscr.addstr(0, max(0, (width - len(title)) // 2), title)
            self.stdscr.attroff(curses.color_pair(1) | curses.A_BOLD)
            self.stdscr.hline(1, 0, curses.ACS_HLINE, width)
        except curses.error:
            pass

        # Status regel
        status_y = 2
        port_info = f"Poort: {self.port} ({'VERBONDEN' if self.ser and self.ser.is_open else 'GEEN VERBINDING'})"
        dur_opt = DURATION_OPTIONS[self.duration_idx]
        dur_str = f"Duratie: {dur_opt}m" if dur_opt else "Duratie: Onbeperkt"

        lim_info = ""
        if self.burst_limit or self.total_limit:
            lim_info = f" [Limiet: B={self.burst_limit or '-'}us, T={self.total_limit or '-'}us]"
        if self.overrun_count > 0:
            lim_info += f" ⚠ Overruns: {self.overrun_count}"

        if self.recording:
            elapsed = time.time() - self.record_start_time
            m, s = divmod(int(elapsed), 60)
            rec_str = f"[ RECORDING 🔴 {m:02d}:{s:02d}{lim_info} ]"
            status_attr = curses.color_pair(4) | curses.A_BOLD
        else:
            rec_str = f"[ GESTOPT ⏹{lim_info} ]"
            status_attr = curses.color_pair(3)

        try:
            self.stdscr.addstr(status_y, 2, port_info)
            self.stdscr.addstr(status_y, width // 2 - 12, dur_str)
            self.stdscr.addstr(status_y, max(0, width - len(rec_str) - 2), rec_str[:width - 35], status_attr)
        except curses.error:
            pass

        if self.ser_error:
            try:
                err_msg = f"⚠ FOUT: {self.ser_error[:width - 10]}"
                self.stdscr.addstr(3, 2, err_msg, curses.color_pair(4) | curses.A_BOLD)
            except curses.error:
                pass

        # 2. Main Data Display: Twee kolommen (Live Waarden vs Statistieken)
        start_y = 5
        panel_h = height - start_y - 4
        col_w = (width - 6) // 2

        self.draw_box(start_y, 2, panel_h, col_w, " Live Sensorgegevens ")
        self.draw_live_data(start_y + 1, 4, panel_h - 2, col_w - 4)

        self.draw_box(start_y, col_w + 4, panel_h, col_w, " Sessie Statistieken ")
        self.draw_stats_data(start_y + 1, col_w + 6, panel_h - 2, col_w - 4)

        # 3. Onderaan: Logbestand & Sneltoetsen balk
        log_name_disp = generate_log_filename(self.custom_log_name, self.allow_overwrite)
        overwrite_flag = " [Overschrijven: AAN]" if self.allow_overwrite else " [Auto-increment: AAN]"
        log_info = f"Logbestand: logs/{log_name_disp}{overwrite_flag}"

        controls = "[S/Space] Start/Stop | [D] Poort | [N] Naam | [L] Limieten | [↑/↓] Duratie | [Q] Quit"

        try:
            self.stdscr.hline(height - 3, 0, curses.ACS_HLINE, width)
            self.stdscr.addstr(height - 2, 2, log_info[:width - 4], curses.color_pair(1))
            self.stdscr.addstr(height - 1, max(0, (width - len(controls)) // 2), controls, curses.A_REVERSE)
        except curses.error:
            pass

        # 4. Actieve Overlay / Modal schermen
        if self.active_modal == "DEVICE_SELECT":
            self.draw_device_modal(height, width)
        elif self.active_modal == "NAME_INPUT":
            self.draw_name_modal(height, width)
        elif self.active_modal == "LIMITS_INPUT":
            self.draw_limits_modal(height, width)
        elif self.active_modal == "SUMMARY":
            self.draw_summary_modal(height, width)

        self.stdscr.refresh()

    def draw_box(self, y, x, h, w, title=""):
        try:
            self.stdscr.attron(curses.color_pair(1))
            self.stdscr.addch(y, x, curses.ACS_ULCORNER)
            self.stdscr.hline(y, x + 1, curses.ACS_HLINE, w - 2)
            self.stdscr.addch(y, x + w - 1, curses.ACS_URCORNER)

            for i in range(1, h - 1):
                self.stdscr.addch(y + i, x, curses.ACS_VLINE)
                self.stdscr.addch(y + i, x + w - 1, curses.ACS_VLINE)

            self.stdscr.addch(y + h - 1, x, curses.ACS_LLCORNER)
            self.stdscr.hline(y + h - 1, x + 1, curses.ACS_HLINE, w - 2)
            self.stdscr.addch(y + h - 1, x + w - 1, curses.ACS_LRCORNER)
            self.stdscr.attroff(curses.color_pair(1))

            if title:
                self.stdscr.addstr(y, x + 2, title, curses.color_pair(1) | curses.A_BOLD)
        except curses.error:
            pass

    def draw_live_data(self, start_y, start_x, max_h, max_w):
        if not self.latest_parsed:
            try:
                self.stdscr.addstr(start_y, start_x, "Wachten op seriële gegevens...", curses.color_pair(3))
            except curses.error:
                pass
            return

        expected_keys = [
            "accel_x", "accel_y", "accel_z",
            "gyro_x", "gyro_y", "gyro_z",
            "burst_duration", "total_duration", "task_duration",
            "ax", "ay", "az",
            "gx", "gy", "gz"
        ]
        keys = [k for k in expected_keys if k in self.latest_parsed] + sorted([k for k in self.latest_parsed if k not in expected_keys])

        max_k_len = max((len(k) for k in keys), default=9)
        max_k_len = max(9, max_k_len)

        row = 0
        for k in keys:
            if row >= max_h:
                break
            val = self.latest_parsed[k]

            # Grafische balk (meter) breedte dynamisch berekenen zodat niks uitsteekt
            bar_w = max(5, max_w - max_k_len - 15)
            if bar_w % 2 == 0:
                bar_w += 1

            if "accel" in k or k.startswith("a"):
                max_scale = 2.0
            elif "duration" in k:
                if "burst" in k and self.burst_limit:
                    max_scale = self.burst_limit
                elif "total" in k and self.total_limit:
                    max_scale = self.total_limit
                else:
                    max_scale = 1000.0
            else:
                max_scale = 100.0

            norm_val = max(-1.0, min(1.0, val / max_scale))
            half_w = bar_w // 2
            filled = int(abs(norm_val) * half_w)

            center = half_w
            bar_chars = ["░"] * bar_w
            bar_chars[center] = "│"

            if norm_val > 0:
                for i in range(center + 1, min(bar_w, center + 1 + filled)):
                    bar_chars[i] = "█"
            elif norm_val < 0:
                for i in range(max(0, center - filled), center):
                    bar_chars[i] = "█"

            bar_str = "".join(bar_chars)

            # Limiet overschrijding controle
            is_overrun = False
            if "burst" in k and self.burst_limit and val > self.burst_limit:
                is_overrun = True
            elif "total" in k and self.total_limit and val > self.total_limit:
                is_overrun = True

            overrun_tag = " ⚠" if is_overrun else ""
            line_str = f"{k:<{max_k_len}} {val:>8.2f} [{bar_str}]{overrun_tag}"

            color = curses.color_pair(4) | curses.A_BOLD if is_overrun else curses.color_pair(2)
            try:
                self.stdscr.addstr(start_y + row, start_x, line_str[:max_w], color)
            except curses.error:
                pass
            row += 1

    def draw_stats_data(self, start_y, start_x, max_h, max_w):
        if not self.history or not any(self.history.values()):
            try:
                self.stdscr.addstr(start_y, start_x, "Geen opgemeten samples.", curses.color_pair(3))
            except curses.error:
                pass
            return

        expected_keys = [
            "accel_x", "accel_y", "accel_z",
            "gyro_x", "gyro_y", "gyro_z",
            "burst_duration", "total_duration", "task_duration",
            "ax", "ay", "az",
            "gx", "gy", "gz"
        ]
        keys = [k for k in expected_keys if k in self.history] + sorted([k for k in self.history if k not in expected_keys])
        max_k_len = max((len(k) for k in keys), default=8)

        header = f"{'Signaal':<{max_k_len}} | {'Min':<7} | {'Max':<7} | {'Gem':<7}"
        try:
            self.stdscr.addstr(start_y, start_x, header[:max_w], curses.A_BOLD)
            self.stdscr.hline(start_y + 1, start_x, curses.ACS_HLINE, min(max_w, len(header)))
        except curses.error:
            pass

        row = 2
        for k in keys:
            if row >= max_h:
                break
            vals = self.history[k]
            if not vals:
                continue
            n = len(vals)
            v_min = min(vals)
            v_max = max(vals)
            mean = sum(vals) / n

            is_overrun = False
            if "burst" in k and self.burst_limit and v_max > self.burst_limit:
                is_overrun = True
            elif "total" in k and self.total_limit and v_max > self.total_limit:
                is_overrun = True

            color = curses.color_pair(4) | curses.A_BOLD if is_overrun else curses.A_NORMAL

            line = f"{k:<{max_k_len}} | {v_min:<7.2f} | {v_max:<7.2f} | {mean:<7.2f}"
            try:
                self.stdscr.addstr(start_y + row, start_x, line[:max_w], color)
            except curses.error:
                pass
            row += 1

    # --- Overlay Modals ---
    def draw_device_modal(self, h, w):
        box_w = min(60, w - 4)
        box_h = min(15, h - 6)
        top_y = (h - box_h) // 2
        left_x = (w - box_w) // 2

        self.draw_box(top_y, left_x, box_h, box_w, " Selecteer Seriële Poort ")

        try:
            self.stdscr.addstr(top_y + 1, left_x + 2, "Gebruik ↑/↓ en Enter om een poort te kiezen:", curses.A_DIM)
        except curses.error:
            pass

        list_h = box_h - 4
        for i in range(list_h):
            idx = i
            if idx >= len(self.modal_ports):
                break
            dev, desc = self.modal_ports[idx]
            item_str = f"{dev:<15} ({desc})"[:box_w - 6]

            attr = curses.color_pair(5) if idx == self.modal_cursor else curses.A_NORMAL
            try:
                prefix = "> " if idx == self.modal_cursor else "  "
                self.stdscr.addstr(top_y + 3 + i, left_x + 2, prefix + item_str, attr)
            except curses.error:
                pass

    def draw_name_modal(self, h, w):
        box_w = min(65, w - 4)
        box_h = 9
        top_y = (h - box_h) // 2
        left_x = (w - box_w) // 2

        self.draw_box(top_y, left_x, box_h, box_w, " Logbestand Naam Instellen ")

        preview_name = generate_log_filename(self.input_text, self.allow_overwrite)
        try:
            self.stdscr.addstr(top_y + 1, left_x + 2, "Voer bestandnaam in (laat leeg voor datumstempel):")
            self.stdscr.addstr(top_y + 2, left_x + 2, f"Preview: logs/{preview_name}", curses.color_pair(3) | curses.A_BOLD)

            input_disp = self.input_text + "█"
            self.stdscr.addstr(top_y + 4, left_x + 2, f"Naam: {input_disp:<40}", curses.A_UNDERLINE | curses.A_BOLD)

            ow_str = "[X] Duplicaten overschrijven" if self.allow_overwrite else "[X] Auto-increment nummering (_1, _2, ...)"
            self.stdscr.addstr(top_y + 6, left_x + 2, f"{ow_str} (Druk op 'a' om te wisselen)")
            self.stdscr.addstr(top_y + 7, left_x + 2, "[Enter] Bevestigen | [Esc] Annuleren", curses.A_DIM)
        except curses.error:
            pass

    def draw_limits_modal(self, h, w):
        box_w = min(65, w - 4)
        box_h = 10
        top_y = (h - box_h) // 2
        left_x = (w - box_w) // 2

        self.draw_box(top_y, left_x, box_h, box_w, " ⚙ Taskduratie Limieten Instellen (µs) ")

        try:
            self.stdscr.addstr(top_y + 1, left_x + 2, "Stel drempelwaarden in voor overruns (laat leeg = geen):", curses.A_DIM)

            b_disp = self.burst_input_str + ("█" if self.limits_field == 0 else "")
            t_disp = self.total_input_str + ("█" if self.limits_field == 1 else "")

            b_attr = curses.A_UNDERLINE | curses.A_BOLD if self.limits_field == 0 else curses.A_NORMAL
            t_attr = curses.A_UNDERLINE | curses.A_BOLD if self.limits_field == 1 else curses.A_NORMAL

            self.stdscr.addstr(top_y + 3, left_x + 2, f"Max Burst Duration (µs): {b_disp:<15}", b_attr)
            self.stdscr.addstr(top_y + 5, left_x + 2, f"Max Total Duration (µs): {t_disp:<15}", t_attr)

            self.stdscr.addstr(top_y + 8, left_x + 2, "[Tab/Pijltjes] Wissel veld | [Enter] Opslaan | [Esc] Annuleren", curses.A_DIM)
        except curses.error:
            pass

    def draw_summary_modal(self, h, w):
        box_w = min(78, w - 4)
        box_h = min(19, h - 4)
        top_y = (h - box_h) // 2
        left_x = (w - box_w) // 2

        self.draw_box(top_y, left_x, box_h, box_w, " SAMENVATTING STATISTIEKEN ")

        total_samples = max((len(v) for v in self.history.values()), default=0)
        dur_str = f"{time.time() - self.record_start_time:.1f} s" if self.record_start_time else "0 s"

        max_dur_list = []
        for k in self.history:
            if "duration" in k and self.history[k]:
                max_v = max(self.history[k])
                max_dur_list.append(f"{k}: {max_v:.2f}µs")
        dur_info_str = (" | Max: " + ", ".join(max_dur_list)) if max_dur_list else ""
        overrun_str = f" | ⚠ Overruns: {self.overrun_count}" if self.overrun_count > 0 else ""

        try:
            self.stdscr.addstr(top_y + 1, left_x + 2, f"Reden stop : {self.stop_reason}", curses.color_pair(3) | curses.A_BOLD)
            self.stdscr.addstr(top_y + 2, left_x + 2, f"Tijdsduur  : {dur_str} | Samples: {total_samples}{overrun_str}"[:box_w - 4])
            if dur_info_str:
                self.stdscr.addstr(top_y + 3, left_x + 2, f"⏱ {dur_info_str.strip(' | ')}"[:box_w - 4], curses.color_pair(2) | curses.A_BOLD)

            header = f"{'Signaal':<15} | {'Min':<8} | {'Max':<8} | {'Gemiddelde':<10} | {'Gem. Afw.':<9} | {'Stdev':<7}"
            self.stdscr.addstr(top_y + 5, left_x + 2, header[:box_w - 4], curses.A_BOLD)
            self.stdscr.hline(top_y + 6, left_x + 2, curses.ACS_HLINE, min(box_w - 4, len(header)))
        except curses.error:
            pass

        expected_keys = [
            "accel_x", "accel_y", "accel_z",
            "gyro_x", "gyro_y", "gyro_z",
            "burst_duration", "total_duration", "task_duration",
            "ax", "ay", "az",
            "gx", "gy", "gz"
        ]
        keys = [k for k in expected_keys if k in self.history] + sorted([k for k in self.history if k not in expected_keys])

        row = 0
        max_rows = box_h - 9
        for k in keys:
            if row >= max_rows:
                break
            vals = self.history[k]
            if not vals:
                continue
            n = len(vals)
            v_min = min(vals)
            v_max = max(vals)
            mean = sum(vals) / n
            mad = sum(abs(v - mean) for v in vals) / n
            stdev = math.sqrt(sum((v - mean) ** 2 for v in vals) / n)

            is_overrun = False
            if "burst" in k and self.burst_limit and v_max > self.burst_limit:
                is_overrun = True
            elif "total" in k and self.total_limit and v_max > self.total_limit:
                is_overrun = True

            color = curses.color_pair(4) | curses.A_BOLD if is_overrun else curses.A_NORMAL

            line = f"{k:<15} | {v_min:<8.2f} | {v_max:<8.2f} | {mean:<10.2f} | {mad:<9.2f} | {stdev:<7.2f}"
            try:
                self.stdscr.addstr(top_y + 7 + row, left_x + 2, line[:box_w - 4], color)
            except curses.error:
                pass
            row += 1

        try:
            self.stdscr.addstr(top_y + box_h - 2, left_x + 2, "Druk op [Enter] of [Esc] om te sluiten", curses.A_DIM)
        except curses.error:
            pass


def main(stdscr):
    parser = argparse.ArgumentParser(description="Rich Interactive IMU Live TUI & Logger")
    parser.add_argument("--device", "-d", type=str, default=None, help="Device index (bijv. 0) of poortnaam (Standaard: auto-detect USB-serial logger)")
    parser.add_argument("--baud", "-b", type=int, default=115200, help="Baud rate (standaard 115200)")
    parser.add_argument("--duration", "-m", type=float, default=None, help="Initiële duratie in minuten")
    parser.add_argument("--log", "-l", type=str, default=None, help="Initiële lognaam")
    parser.add_argument("--burst-limit", "-bl", type=float, default=None, help="Burst duration limiet in us")
    parser.add_argument("--total-limit", "-tl", type=float, default=None, help="Total duration limiet in us")
    args = parser.parse_args()

    app = ImuTuiApp(
        stdscr=stdscr,
        initial_device=args.device,
        baud=args.baud,
        duration=args.duration,
        log_input=args.log,
        burst_limit=args.burst_limit,
        total_limit=args.total_limit,
    )
    app.run()


if __name__ == "__main__":
    try:
        curses.wrapper(main)
    except KeyboardInterrupt:
        print("\nProgramma afgesloten door gebruiker.")
