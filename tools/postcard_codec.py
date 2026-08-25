"""
Postcard serialization and deserialization codec for the Rust Flight Controller USB protocol.
Matches `src/usb.rs` types:

ToPc:
  0: Attitude { roll: f32, pitch: f32, yaw: f32 }
  1: SystemState { state: u8, arm_blocks: u32, errors: u32 }
  2: Ack
  3: Log(heapless::String<32>)

FromPc:
  0: Arm
  1: Disarm
"""

from dataclasses import dataclass
import struct
from typing import Union, Tuple, Optional, List


def encode_varint(val: int) -> bytes:
    """Encode an integer into LEB128 unsigned varint bytes."""
    buf = bytearray()
    while True:
        byte = val & 0x7F
        val >>= 7
        if val != 0:
            byte |= 0x80
            buf.append(byte)
        else:
            buf.append(byte)
            break
    return bytes(buf)


def decode_varint(buf: bytes, offset: int = 0) -> Tuple[int, int]:
    """
    Decode a LEB128 unsigned varint from `buf` starting at `offset`.
    Returns (value, new_offset).
    Raises ValueError if `buf` ends before varint termination.
    """
    result = 0
    shift = 0
    while offset < len(buf):
        byte = buf[offset]
        offset += 1
        result |= (byte & 0x7F) << shift
        if not (byte & 0x80):
            return result, offset
        shift += 7
    raise ValueError("Incomplete varint")


@dataclass
class Attitude:
    roll: float
    pitch: float
    yaw: float


@dataclass
class SystemState:
    state: int
    arm_blocks: int
    errors: int

    @property
    def state_name(self) -> str:
        states = {
            0: "INIT",
            1: "DISARMED",
            2: "ARMED",
            3: "FAILSAFE",
            4: "CALIBRATING"
        }
        return states.get(self.state, f"UNKNOWN({self.state})")


@dataclass
class Ack:
    pass


@dataclass
class Log:
    message: str


ToPcMessage = Union[Attitude, SystemState, Ack, Log]


@dataclass
class Rates:
    roll: float
    pitch: float
    yaw: float


@dataclass
class RcChannels:
    rates: Rates
    throttle: float
    arm: bool
    disarm: bool
    mode: int = 0  # 0: FlightMode::Angle, 1: FlightMode::Rate


class FromPcCommand:
    """Commands and RC Channel packets sent from PC to Flight Controller."""
    
    @staticmethod
    def encode_rc_channels(roll: float, pitch: float, yaw: float, throttle: float,
                           arm: bool, disarm: bool, mode: int = 0) -> bytes:
        """
        Postcard serialization for `RcChannels` struct (src/receiver/receiver.rs):
        struct RcChannels {
            rates: Rates { roll: f32, pitch: f32, yaw: f32 },
            throttle: f32,
            arm: bool,
            disarm: bool,
            mode: FlightMode (0: Angle, 1: Rate)
        }
        """
        floats_bytes = struct.pack("<ffff", roll, pitch, yaw, throttle)
        arm_byte = b"\x01" if arm else b"\x00"
        disarm_byte = b"\x01" if disarm else b"\x00"
        mode_varint = encode_varint(mode)
        return floats_bytes + arm_byte + disarm_byte + mode_varint

    @staticmethod
    def encode_arm() -> bytes:
        # Backward compatibility FromPc::Arm variant
        return encode_varint(0)

    @staticmethod
    def encode_disarm() -> bytes:
        # Backward compatibility FromPc::Disarm variant
        return encode_varint(1)

    @staticmethod
    def encode_set_pid(axis: int, kp: float, ki: float, kd: float) -> bytes:
        """
        Variant index 2: SetPid { axis: u8, kp: f32, ki: f32, kd: f32 }
        axis: 0 = Roll, 1 = Pitch, 2 = Yaw
        """
        return encode_varint(2) + struct.pack("<Bfff", axis, kp, ki, kd)

    @staticmethod
    def encode_set_filter(gyro_cutoff: float, dterm_cutoff: float) -> bytes:
        """
        Variant index 3: SetFilter { gyro_cutoff: f32, dterm_cutoff: f32 }
        """
        return encode_varint(3) + struct.pack("<ff", gyro_cutoff, dterm_cutoff)


def decode_to_pc_packet(data: bytes) -> Tuple[Optional[ToPcMessage], int]:
    """
    Attempt to decode one `ToPc` message from `data`.
    Returns (message, bytes_consumed).
    If the buffer has insufficient data for a complete packet, returns (None, 0).
    If an invalid discriminant is encountered, raises ValueError.
    """
    if not data:
        return None, 0

    try:
        variant, offset = decode_varint(data, 0)
    except ValueError:
        return None, 0

    if variant == 0:  # Attitude { roll: f32, pitch: f32, yaw: f32 }
        if len(data) - offset < 12:
            return None, 0
        roll, pitch, yaw = struct.unpack_from("<fff", data, offset)
        return Attitude(roll=roll, pitch=pitch, yaw=yaw), offset + 12

    elif variant == 1:  # SystemState { state: u8, arm_blocks: u32, errors: u32 }
        if len(data) - offset < 1:
            return None, 0
        state = data[offset]
        cur_off = offset + 1
        try:
            arm_blocks, cur_off = decode_varint(data, cur_off)
            errors, cur_off = decode_varint(data, cur_off)
        except ValueError:
            return None, 0
        return SystemState(state=state, arm_blocks=arm_blocks, errors=errors), cur_off

    elif variant == 2:  # Ack
        return Ack(), offset

    elif variant == 3:  # Log(heapless::String<32>)
        try:
            length, cur_off = decode_varint(data, offset)
        except ValueError:
            return None, 0
        if len(data) - cur_off < length:
            return None, 0
        log_bytes = data[cur_off : cur_off + length]
        log_str = log_bytes.decode("utf-8", errors="replace")
        return Log(message=log_str), cur_off + length

    else:
        raise ValueError(f"Unknown ToPc variant discriminant: {variant}")
