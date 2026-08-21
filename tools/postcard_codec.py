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


class FromPcCommand:
    """Commands sent from PC to Flight Controller (FromPc enum)."""
    
    @staticmethod
    def encode_arm() -> bytes:
        # Variant index 0
        return encode_varint(0)

    @staticmethod
    def encode_disarm() -> bytes:
        # Variant index 1
        return encode_varint(1)


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
