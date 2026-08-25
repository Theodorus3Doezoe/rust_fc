"""
Xbox One Controller Input Handler for Flight Controller CLI & TUI.

Control Mapping:
- Roll: Right Stick X-axis (-1.0 to +1.0)
- Pitch: Left Stick Y-axis (-1.0 to +1.0, forward/backward)
- Yaw: LT (negative yaw) & RT (positive yaw) -> Range [-1.0, +1.0]
- Throttle: D-Pad UP (+0.05) & D-Pad DOWN (-0.05) (Cruise Control: 0.0 to 1.0)
- ARM: Button A
- DISARM: Button B
"""

from dataclasses import dataclass
import os
import struct
import threading
import time
from typing import Callable, Optional


@dataclass
class FlightStickState:
    roll: float = 0.0      # -1.0 (left) to +1.0 (right)
    pitch: float = 0.0     # -1.0 (backward) to +1.0 (forward)
    yaw: float = 0.0       # -1.0 (LT max) to +1.0 (RT max)
    throttle: float = 0.0  # 0.0 (idle) to 1.0 (full)
    arm_pressed: bool = False
    disarm_pressed: bool = False
    connected: bool = False
    device_name: str = "Disconnected"


class XboxControllerReader:
    """Reads input from Linux /dev/input/js* or evdev, with fallback simulation mode."""

    def __init__(self, device_path: Optional[str] = None, sim: bool = False):
        self.device_path = device_path
        self.sim = sim
        self.state = FlightStickState()
        self.running = False
        self._thread: Optional[threading.Thread] = None

        # Raw trigger values [0.0, 1.0]
        self._lt_raw = 0.0
        self._rt_raw = 0.0

        # Callbacks
        self.on_arm_callback: Optional[Callable[[], None]] = None
        self.on_disarm_callback: Optional[Callable[[], None]] = None
        self.on_state_change: Optional[Callable[[FlightStickState], None]] = None

    def start(self):
        self.running = True
        if self.sim:
            self.state.connected = True
            self.state.device_name = "Xbox Controller (Simulator)"
            self._thread = threading.Thread(target=self._sim_loop, daemon=True)
            self._thread.start()
        else:
            self._thread = threading.Thread(target=self._read_loop, daemon=True)
            self._thread.start()

    def stop(self):
        self.running = False
        self.state.connected = False

    def set_sim_input(self, roll: Optional[float] = None, pitch: Optional[float] = None,
                      yaw: Optional[float] = None, throttle_delta: float = 0.0):
        """Helper to modify simulator state for manual testing."""
        if roll is not None:
            self.state.roll = max(-1.0, min(1.0, round(roll, 3)))
        if pitch is not None:
            self.state.pitch = max(-1.0, min(1.0, round(pitch, 3)))
        if yaw is not None:
            self.state.yaw = max(-1.0, min(1.0, round(yaw, 3)))
        if throttle_delta != 0.0:
            self.state.throttle = max(0.0, min(1.0, round(self.state.throttle + throttle_delta, 2)))
        self._notify()

    def trigger_sim_arm(self):
        self.state.arm_pressed = True
        if self.on_arm_callback:
            self.on_arm_callback()
        self._notify()

    def trigger_sim_disarm(self):
        self.state.disarm_pressed = True
        if self.on_disarm_callback:
            self.on_disarm_callback()
        self._notify()

    def _notify(self):
        if self.on_state_change:
            self.on_state_change(self.state)

    def _detect_attached_xbox_sysfs(self) -> Optional[str]:
        base = "/sys/bus/usb/devices"
        if not os.path.exists(base):
            return None
        for dev in os.listdir(base):
            p_vendor = os.path.join(base, dev, "idVendor")
            p_product = os.path.join(base, dev, "idProduct")
            p_name = os.path.join(base, dev, "product")
            if os.path.exists(p_vendor) and os.path.exists(p_product):
                try:
                    vid = open(p_vendor).read().strip().lower()
                    pid = open(p_product).read().strip().lower()
                    name = open(p_name).read().strip() if os.path.exists(p_name) else "Controller"
                    if vid != "1d6b" and (vid == "045e" or "controller" in name.lower() or "xbox" in name.lower()):
                        return f"Xbox USB {vid}:{pid} ({name})"
                except Exception:
                    pass
        return None

    def _read_loop_pyusb(self) -> bool:
        """Direct PyUSB read loop for Xbox One Controllers (045e:*) bypassing Linux kernel drivers."""
        try:
            import usb.core
            import usb.util
        except ImportError:
            return False

        dev = usb.core.find(idVendor=0x045e)
        if not dev:
            return False

        try:
            try:
                if dev.is_kernel_driver_active(0):
                    dev.detach_kernel_driver(0)
            except Exception:
                pass

            try:
                dev.set_configuration()
            except Exception:
                pass

            cfg = dev.get_active_configuration()
            if not cfg:
                return False

            intf = cfg[(0, 0)]
            ep_in = None
            for ep in intf:
                if usb.util.endpoint_direction(ep.bEndpointAddress) == usb.util.ENDPOINT_IN:
                    ep_in = ep
                    break

            if not ep_in:
                return False

            self.state.connected = True
            self.state.device_name = f"Xbox Controller (PyUSB Direct)"
            self._notify()

            last_dpad_up = False
            last_dpad_down = False
            last_dpad_up_time = 0.0
            last_dpad_down_time = 0.0
            last_arm_time = 0.0
            last_disarm_time = 0.0

            while self.running:
                try:
                    data = ep_in.read(64, timeout=500)
                    if not data or len(data) < 18:
                        continue

                    if data[0] == 0x20 and len(data) >= 18:
                        buttons, lt, rt, lx, ly, rx, ry = struct.unpack_from("<HHHhhhh", data, 4)

                        now = time.time()
                        arm_now = bool(buttons & 0x0010)
                        disarm_now = bool(buttons & 0x0020)

                        if arm_now:
                            if not self.state.arm_pressed or (now - last_arm_time > 0.15):
                                if self.on_arm_callback:
                                    self.on_arm_callback()
                                last_arm_time = now

                        if disarm_now:
                            if not self.state.disarm_pressed or (now - last_disarm_time > 0.15):
                                if self.on_disarm_callback:
                                    self.on_disarm_callback()
                                last_disarm_time = now

                        self.state.arm_pressed = arm_now
                        self.state.disarm_pressed = disarm_now

                        pitch_raw = -ly / 32767.0
                        roll_raw = rx / 32767.0
                        self.state.pitch = max(-1.0, min(1.0, round(pitch_raw if abs(pitch_raw) > 0.05 else 0.0, 3)))
                        self.state.roll = max(-1.0, min(1.0, round(roll_raw if abs(roll_raw) > 0.05 else 0.0, 3)))

                        self._lt_raw = lt / 1023.0
                        self._rt_raw = rt / 1023.0
                        self.state.yaw = max(-1.0, min(1.0, round(self._rt_raw - self._lt_raw, 3)))

                        dpad_up = bool(buttons & 0x0100)
                        dpad_down = bool(buttons & 0x0200)

                        if dpad_up:
                            if not last_dpad_up:
                                self.state.throttle = min(1.0, round(self.state.throttle + 0.05, 2))
                                last_dpad_up_time = now
                            elif now - last_dpad_up_time >= 0.10:
                                self.state.throttle = min(1.0, round(self.state.throttle + 0.05, 2))
                                last_dpad_up_time = now
                        
                        if dpad_down:
                            if not last_dpad_down:
                                self.state.throttle = max(0.0, round(self.state.throttle - 0.05, 2))
                                last_dpad_down_time = now
                            elif now - last_dpad_down_time >= 0.10:
                                self.state.throttle = max(0.0, round(self.state.throttle - 0.05, 2))
                                last_dpad_down_time = now

                        last_dpad_up = dpad_up
                        last_dpad_down = dpad_down

                        self._notify()
                except Exception:
                    time.sleep(0.05)
            return True
        except Exception:
            return False

    def _read_loop(self):
        """Read loop for Linux Joystick API /dev/input/js* with PyUSB fallback."""
        js_dev = self.device_path
        if not js_dev:
            for i in range(8):
                path = f"/dev/input/js{i}"
                if os.path.exists(path):
                    js_dev = path
                    break

        if not js_dev or not os.path.exists(js_dev):
            # Try PyUSB direct read first before giving up
            if self._read_loop_pyusb():
                return

            sysfs_dev = self._detect_attached_xbox_sysfs()
            if sysfs_dev:
                self.state.connected = False
                self.state.device_name = f"{sysfs_dev} (Run with sudo to access PyUSB)"
            else:
                self.state.connected = False
                self.state.device_name = "No Gamepad Found (Press 'x' for Controller Sim)"
            self._notify()
            return

        try:
            with open(js_dev, "rb") as fd:
                self.state.connected = True
                self.state.device_name = f"Xbox Controller ({os.path.basename(js_dev)})"
                self._notify()

                # Linux joystick event struct: uint32 time, int16 value, uint8 type, uint8 number
                EVENT_FORMAT = "IhBB"
                EVENT_SIZE = struct.calcsize(EVENT_FORMAT)

                while self.running:
                    data = fd.read(EVENT_SIZE)
                    if not data:
                        break
                    _, val, evt_type, num = struct.unpack(EVENT_FORMAT, data)

                    # Ignore init flag (0x80)
                    evt_type &= ~0x80

                    if evt_type == 0x01:  # Button event
                        if num == 0 and val == 1:  # A button pressed -> ARM
                            self.state.arm_pressed = True
                            if self.on_arm_callback:
                                self.on_arm_callback()
                        elif num == 1 and val == 1:  # B button pressed -> DISARM
                            self.state.disarm_pressed = True
                            if self.on_disarm_callback:
                                self.on_disarm_callback()

                    elif evt_type == 0x02:  # Axis event
                        norm_val = val / 32767.0

                        # Axis 1: Left Stick Y (Pitch: forward is negative in JS API, invert so forward is +1.0)
                        if num == 1:
                            # Apply small deadzone
                            clean_val = -norm_val if abs(norm_val) > 0.05 else 0.0
                            self.state.pitch = max(-1.0, min(1.0, round(clean_val, 3)))

                        # Axis 3: Right Stick X (Roll: right is +1.0)
                        elif num == 3:
                            clean_val = norm_val if abs(norm_val) > 0.05 else 0.0
                            self.state.roll = max(-1.0, min(1.0, round(clean_val, 3)))

                        # Axis 2: LT (Left Trigger: negative yaw)
                        elif num == 2:
                            self._lt_raw = max(0.0, (val + 32767) / 65534.0)
                            combined_yaw = self._rt_raw - self._lt_raw
                            self.state.yaw = max(-1.0, min(1.0, round(combined_yaw, 3)))

                        # Axis 5: RT (Right Trigger: positive yaw)
                        elif num == 5:
                            self._rt_raw = max(0.0, (val + 32767) / 65534.0)
                            combined_yaw = self._rt_raw - self._lt_raw
                            self.state.yaw = max(-1.0, min(1.0, round(combined_yaw, 3)))

                        # Axis 7: D-Pad Y (Hat Y: -32767 UP, +32767 DOWN)
                        elif num == 7:
                            if val < -10000:  # D-Pad UP -> Throttle UP (Cruise Control)
                                self.state.throttle = min(1.0, round(self.state.throttle + 0.05, 2))
                            elif val > 10000:  # D-Pad DOWN -> Throttle DOWN
                                self.state.throttle = max(0.0, round(self.state.throttle - 0.05, 2))

                    self._notify()

        except Exception as e:
            self.state.connected = False
            self.state.device_name = f"Disconnected ({e})"
            self._notify()

    def _sim_loop(self):
        """Simulated controller loop for testing."""
        t = 0.0
        while self.running:
            time.sleep(0.05)
            t += 0.05
            # Small natural drift visualization in simulator if no manual input overrides
            if self.state.roll == 0.0 and self.state.pitch == 0.0:
                pass
            self._notify()
