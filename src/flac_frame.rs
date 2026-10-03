//! Validate that a decoder packet contains exactly one complete FLAC frame.
//! The locked decoder does not reject unconsumed bytes after its first frame.
//! This checks encoded layout without reconstructing PCM or allocating buffers.
use crate::decode::Failure;
use symphonia::core::io::{BitReaderLtr, FiniteBitStream, ReadBitsLtr};

fn invalid(message: &str) -> Failure {
    Failure::Decode(format!("FLAC frame layout: {message}"))
}

struct Layout<'a, F> {
    bits: BitReaderLtr<'a>,
    control: F,
}

impl<F: FnMut() -> Result<(), Failure>> Layout<'_, F> {
    fn read(&mut self, count: u32) -> Result<u32, Failure> {
        self.bits
            .read_bits_leq32(count)
            .map_err(|_| invalid("truncated subframe"))
    }

    fn skip(&mut self, count: u32) -> Result<(), Failure> {
        self.bits
            .ignore_bits(count)
            .map_err(|_| invalid("truncated subframe"))
    }

    fn residual(&mut self, frames: u32, order: u32) -> Result<(), Failure> {
        let parameter_bits = match self.read(2)? {
            0 => 4,
            1 => 5,
            _ => return Err(invalid("reserved residual coding method")),
        };
        let partitions = 1 << self.read(4)?;
        if frames % partitions != 0 || frames / partitions <= order {
            return Err(invalid("invalid residual partition geometry"));
        }
        for partition in 0..partitions {
            (self.control)()?;
            let count = frames / partitions - if partition == 0 { order } else { 0 };
            let parameter = self.read(parameter_bits)?;
            if parameter == (1 << parameter_bits) - 1 {
                let width = self.read(5)?;
                self.skip(count * width)?;
            } else {
                for i in 0..count {
                    if i % 1024 == 0 {
                        (self.control)()?;
                    }
                    // Cap individual unary reads so adversarial quotients cannot
                    // defer cooperative cancellation for a whole packet.
                    loop {
                        let zeros = self
                            .bits
                            .read_unary_zeros_capped(4096)
                            .map_err(|_| invalid("truncated Rice quotient"))?;
                        if zeros < 4096 {
                            break;
                        }
                        (self.control)()?;
                    }
                    self.skip(parameter)?;
                }
            }
        }
        Ok(())
    }

    fn subframe(&mut self, frames: u32, precision: u32) -> Result<(), Failure> {
        (self.control)()?;
        let header = self.read(8)?;
        if header & 128 != 0 {
            return Err(invalid("reserved subframe header bit"));
        }
        let wasted = if header & 1 != 0 {
            self.bits
                .read_unary_zeros_capped(33)
                .map_err(|_| invalid("truncated wasted-bit count"))?
                + 1
        } else {
            0
        };
        if wasted >= precision {
            return Err(invalid("wasted bits exhaust sample precision"));
        }
        let precision = precision - wasted;
        match header >> 1 {
            0 => self.skip(precision),
            1 => self.skip(frames * precision),
            kind @ 8..=12 => {
                let order = kind - 8;
                if order > frames {
                    return Err(invalid("fixed predictor exceeds block length"));
                }
                self.skip(order * precision)?;
                self.residual(frames, order)
            }
            kind @ 32..=63 => {
                let order = kind - 31;
                if order > frames {
                    return Err(invalid("LPC predictor exceeds block length"));
                }
                self.skip(order * precision)?;
                let coefficient_bits = self.read(4)? + 1;
                if coefficient_bits == 16 {
                    return Err(invalid("reserved LPC coefficient precision"));
                }
                self.skip(5)?; // Signed predictor shift; the decoder validates it.
                self.skip(order * coefficient_bits)?;
                self.residual(frames, order)
            }
            _ => Err(invalid("reserved subframe type")),
        }
    }
}

pub(crate) fn validate(
    bytes: &[u8],
    frames: u64,
    channels: usize,
    precision: u32,
    control: impl FnMut() -> Result<(), Failure>,
) -> Result<(), Failure> {
    if bytes.len() < 10 || bytes.len() > 16 * 1024 * 1024 || !(1..=65535).contains(&frames) {
        return Err(invalid("unsupported or truncated frame size"));
    }
    if bytes[0] != 255 || bytes[1] & 0xfe != 0xf8 || !(4..=32).contains(&precision) {
        return Err(invalid("invalid frame header or precision"));
    }
    let number_bytes = match bytes[4].leading_ones() {
        0 => 1,
        n @ 2..=7 => n as usize,
        _ => return Err(invalid("invalid coded frame/sample number")),
    };
    let block_bytes = match bytes[2] >> 4 {
        6 => 1,
        7 => 2,
        _ => 0,
    };
    let rate_bytes = match bytes[2] & 15 {
        12 => 1,
        13 | 14 => 2,
        _ => 0,
    };
    let header_bytes = 4 + number_bytes + block_bytes + rate_bytes + 1;
    if header_bytes + 2 >= bytes.len() {
        return Err(invalid("truncated frame header"));
    }
    let assignment = bytes[3] >> 4;
    let frame_channels = match assignment {
        0..=7 => usize::from(assignment) + 1,
        8..=10 => 2,
        _ => return Err(invalid("reserved channel assignment")),
    };
    if frame_channels != channels {
        return Err(invalid("channel assignment changed"));
    }
    let frame_precision = match (bytes[3] >> 1) & 7 {
        0 => precision,
        1 => 8,
        2 => 12,
        4 => 16,
        5 => 20,
        6 => 24,
        _ => return Err(invalid("unsupported frame precision code")),
    };
    if frame_precision != precision {
        return Err(invalid("sample precision changed"));
    }
    let mut layout = Layout {
        bits: BitReaderLtr::new(&bytes[header_bytes..bytes.len() - 2]),
        control,
    };
    for channel in 0..channels {
        let side = matches!((assignment, channel), (8 | 10, 1) | (9, 0));
        layout.subframe(frames as u32, precision + u32::from(side))?;
    }
    let padding = layout.bits.bits_left();
    if padding >= 8 || (padding != 0 && layout.read(padding as u32)? != 0) {
        return Err(invalid("unconsumed bytes or nonzero frame padding"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_unterminated_rice_quotient_checks_control_before_exhaustion() {
        // Fixed order zero, Rice parameter zero, then an adversarial quotient.
        // Frame CRC is intentionally irrelevant: this check runs before decode.
        let mut bytes = vec![255, 248, 0x64, 8, 0, 255, 0, 0x10];
        bytes.extend([0; 16384]);
        let mut checks = 0;
        let error = validate(&bytes, 256, 1, 16, || {
            checks += 1;
            if checks == 4 {
                Err(Failure::TimedOut)
            } else {
                Ok(())
            }
        })
        .unwrap_err();
        assert!(matches!(error, Failure::TimedOut));
        assert_eq!(checks, 4);
    }
}
