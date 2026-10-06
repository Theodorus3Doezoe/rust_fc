// this file contains dshot frame creation

use nalgebra::ComplexField;

const GCR_DECODE: [u8; 32] = [
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x09, 0x0A, 0x0B, 0xFF, 0x0D, 0x0E, 0x0F,
    0xFF, 0xFF, 0x02, 0x03, 0xFF, 0x05, 0x06, 0x07, 0xFF, 0x00, 0x08, 0x01, 0xFF, 0x04, 0x0C, 0xFF,
];

const TELEMETRY_BITS: u32 = 21;

#[derive(Clone, Copy, defmt::Format)]
pub struct DshotTelemetry {
    pub raw: u16,
    pub erpm_period_us: u32,
    pub erpm: u32,
}

pub fn create_dshot_frame(throttle: u16, telemetry: bool) -> u16 {
    let throttle = throttle.min(2047);

    let data = (throttle << 1) | (telemetry as u16);

    let crc = (!(data ^ (data >> 4) ^ (data >> 8))) & 0x0F;

    (data << 4) | crc
}

fn push_run(levels: &mut u64, bits: &mut u32, level_high: bool, run_samples: u32) -> bool {
    let remaining = TELEMETRY_BITS.saturating_sub(*bits);
    let run_bits = ((run_samples + 1) / 3).clamp(1, 3).min(remaining);

    *levels = (*levels << run_bits)
        | if level_high {
            (1u64 << run_bits) - 1
        } else {
            0
        };
    *bits += run_bits;

    *bits >= TELEMETRY_BITS
}

pub fn decode_dshot_telemetry(samples: u64) -> Option<DshotTelemetry> {
    if samples >> 63 != 0 {
        return None;
    }

    let mut levels: u64 = 0;
    let mut bits: u32 = 0;
    let mut level_high = false;
    let mut run: u32 = 0;

    for index in 0..64 {
        let sample_high = (samples >> (63 - index)) & 1 != 0;

        if sample_high == level_high {
            run += 1;
            continue;
        }

        if push_run(&mut levels, &mut bits, level_high, run) {
            break;
        }

        level_high = sample_high;
        run = 1;
    }

    if bits < TELEMETRY_BITS {
        push_run(&mut levels, &mut bits, level_high, run);
    }

    if bits < TELEMETRY_BITS {
        return None;
    }

    let word = (levels >> (bits - TELEMETRY_BITS)) as u32;
    let gcr = (word ^ (word >> 1)) & 0xFFFFF;

    let mut value: u16 = 0;
    for quintet in 0..4 {
        let code = ((gcr >> (15 - quintet * 5)) & 0x1F) as usize;
        let nibble = GCR_DECODE[code];

        if nibble == 0xFF {
            return None;
        }

        value = (value << 4) | nibble as u16;
    }

    let crc = (!((value >> 4) ^ (value >> 8) ^ (value >> 12))) & 0x0F;
    if crc != value & 0x0F {
        return None;
    }

    let raw = value >> 4;
    let mut erpm_period_us = 0;
    let mut erpm = 0;

    if raw & 0x100 != 0 {
        erpm_period_us = ((raw & 0x1FF) as u32) << (raw >> 9);

        if erpm_period_us > 0 && raw != 0xFFF {
            erpm = 60_000_000 / erpm_period_us;
        } else {
            erpm_period_us = 0;
        }
    }

    Some(DshotTelemetry {
        raw,
        erpm_period_us,
        erpm,
    })
}

pub fn mixer_to_dshot_throttle(mixer_value: f32) -> u16 {
    let clamped = mixer_value.clamp(0.0, 1.0);

    let dshot = 48.0 + clamped * (2047.0 - 48.0);
    dshot.round() as u16
}
