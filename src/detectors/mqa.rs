//! Bounded observation of the reference's reverse-engineered signalling fields.
//! A matching marker is a candidate, never structural confirmation or confidence.
use crate::model::{DetectorStatus, MqaCandidate, MqaObservation};

const MAGIC: u64 = 0xbe0498c88;
const MASK: u64 = (1 << 36) - 1;
pub(crate) const MAX_CANDIDATES: usize = 64;

struct Pending {
    candidate: MqaCandidate,
    slot: usize,
    rate: u8,
    provenance: u8,
}

pub(crate) struct MqaScanner {
    depth: Option<u32>,
    supported: bool,
    limit: u64,
    frames: u64,
    windows: [u64; 3],
    matches: u64,
    retained: Vec<Pending>,
}

pub(crate) fn rate_from_code(code: u8) -> u32 {
    let base = if code & 1 == 1 { 48000 } else { 44100 };
    let exponent = ((code >> 3) & 1) | (((code >> 2) & 1) << 1) | (((code >> 1) & 1) << 2);
    let mut multiple = 1u32 << exponent;
    if multiple > 16 {
        multiple *= 2;
    }
    base * multiple
}

impl MqaScanner {
    pub fn new(rate: u32, channels: usize, depth: Option<u32>) -> Self {
        Self {
            depth,
            supported: channels == 2 && matches!(depth, Some(16 | 24)),
            limit: u64::from(rate) * 3,
            frames: 0,
            windows: [0; 3],
            matches: 0,
            retained: Vec::with_capacity(MAX_CANDIDATES),
        }
    }

    /// Input is integer PCM left-aligned in 32 bits, as delivered by the decoder.
    pub fn push(&mut self, left: Option<i32>, right: Option<i32>) {
        if !self.supported || self.frames >= self.limit {
            return;
        }
        let (Some(left), Some(right)) = (left, right) else {
            self.supported = false;
            return;
        };
        let xor = (left ^ right) as u32;
        for pending in &mut self.retained {
            if pending.candidate.payload_complete {
                continue;
            }
            let offset = self.frames - pending.candidate.sync_end_frame;
            let bit = ((xor >> (16 + pending.slot)) & 1) as u8;
            if (3..=6).contains(&offset) {
                pending.rate = (pending.rate << 1) | bit;
            }
            if offset == 6 {
                pending.candidate.rate_code = Some(pending.rate);
                pending.candidate.rate_field_hz = Some(rate_from_code(pending.rate));
            }
            if (29..=33).contains(&offset) {
                pending.provenance = (pending.provenance << 1) | bit;
            }
            if offset == 33 {
                pending.candidate.provenance_code = Some(pending.provenance);
                pending.candidate.studio_flag = Some(pending.provenance > 8);
                pending.candidate.payload_complete = true;
            }
        }
        for slot in 0..3 {
            self.windows[slot] =
                ((self.windows[slot] << 1) | u64::from((xor >> (16 + slot)) & 1)) & MASK;
            if self.frames >= 35 && self.windows[slot] == MAGIC {
                self.matches += 1;
                if self.retained.len() < MAX_CANDIDATES {
                    self.retained.push(Pending {
                        slot,
                        rate: 0,
                        provenance: 0,
                        candidate: MqaCandidate {
                            source_bit_plane: self.depth.unwrap() - 16 + slot as u32,
                            sync_end_frame: self.frames,
                            rate_code: None,
                            rate_field_hz: None,
                            provenance_code: None,
                            studio_flag: None,
                            payload_complete: false,
                        },
                    });
                }
            }
        }
        self.frames += 1;
    }

    pub fn finish(self) -> MqaObservation {
        let status = if !self.supported {
            DetectorStatus::Unsupported
        } else if self.matches > 0 {
            DetectorStatus::Hit
        } else if self.frames < 36 {
            DetectorStatus::Inconclusive
        } else {
            DetectorStatus::NotDetected
        };
        MqaObservation { status, scanned_frames: self.frames, metadata_evaluated: false,
            sync_matches: self.matches, candidates_truncated: self.matches > MAX_CANDIDATES as u64,
            candidates: self.retained.into_iter().map(|p| p.candidate).collect(),
            caveats: vec![
                "A sync-word match is a candidate observation, not confirmation of MQA, source history, or a decoded/unfolded payload.".into(),
                "Fields follow the pinned Python reverse-engineered mapping; repetition and packet consistency have not been independently validated.".into(),
                "Only native 16/24-bit integer stereo PCM within the first three analyzed seconds is scanned. Metadata is not evaluated.".into(),
                "No match within this interval does not exclude signalling elsewhere.".into(),
            ] }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(depth: u32, slot: usize, start: usize, count: usize) -> Vec<(i32, i32)> {
        let mut words = vec![(0, 0); count];
        let plane = 16 + slot;
        for (value, width, offset) in [
            (MAGIC, 36, start),
            (9, 4, start + 35 + 3),
            (17, 5, start + 35 + 29),
        ] {
            for index in 0..width {
                if offset + index < count {
                    words[offset + index].0 |=
                        (((value >> (width - 1 - index)) & 1) as i32) << plane;
                }
            }
        }
        assert!(matches!(depth, 16 | 24));
        words
    }

    #[test]
    fn candidates_preserve_fields_in_every_plane_and_depth() {
        for depth in [16, 24] {
            for slot in 0..3 {
                let mut scan = MqaScanner::new(44100, 2, Some(depth));
                for (l, r) in fixture(depth, slot, 40, 256) {
                    scan.push(Some(l), Some(r));
                }
                let result = scan.finish();
                assert_eq!(result.status, DetectorStatus::Hit);
                assert_eq!(result.sync_matches, 1);
                let c = &result.candidates[0];
                assert_eq!(c.source_bit_plane, depth - 16 + slot as u32);
                assert_eq!(c.sync_end_frame, 75);
                assert_eq!(c.rate_field_hz, Some(96000));
                assert_eq!(c.provenance_code, Some(17));
                assert_eq!(c.studio_flag, Some(true));
                assert!(c.payload_complete);
            }
        }
    }

    #[test]
    fn partial_payload_does_not_invent_fields() {
        for length in [76, 80, 82, 108] {
            let mut scan = MqaScanner::new(44100, 2, Some(16));
            for (l, r) in fixture(16, 0, 40, length) {
                scan.push(Some(l), Some(r));
            }
            let c = scan.finish().candidates.remove(0);
            assert!(!c.payload_complete);
            assert_eq!(c.studio_flag, None);
            assert_eq!(
                c.rate_field_hz,
                if length >= 82 { Some(96000) } else { None }
            );
        }
    }

    #[test]
    fn rates_match_pinned_mapping() {
        for (code, rate) in [
            (0, 44100),
            (1, 48000),
            (8, 88200),
            (9, 96000),
            (4, 176400),
            (5, 192000),
            (12, 352800),
            (13, 384000),
        ] {
            assert_eq!(rate_from_code(code), rate);
        }
    }

    #[test]
    fn silence_and_unsupported_inputs_do_not_claim_a_hit() {
        for (channels, depth, status) in [
            (2, 16, DetectorStatus::NotDetected),
            (1, 16, DetectorStatus::Unsupported),
            (2, 32, DetectorStatus::Unsupported),
        ] {
            let mut scan = MqaScanner::new(44100, channels, Some(depth));
            for _ in 0..256 {
                scan.push(Some(0), Some(0));
            }
            assert_eq!(scan.finish().status, status);
        }
        let mut scan = MqaScanner::new(44100, 2, Some(16));
        scan.push(None, None);
        assert_eq!(scan.finish().status, DetectorStatus::Unsupported);
    }

    #[test]
    fn storage_is_bounded_but_all_sync_matches_are_counted() {
        let mut scan = MqaScanner::new(44100, 2, Some(16));
        for _ in 0..100 {
            for (l, r) in fixture(16, 0, 0, 100) {
                scan.push(Some(l), Some(r));
            }
        }
        let result = scan.finish();
        assert_eq!(result.sync_matches, 100);
        assert_eq!(result.candidates.len(), MAX_CANDIDATES);
        assert!(result.candidates_truncated);
    }

    #[test]
    fn scan_stops_at_three_seconds() {
        let mut scan = MqaScanner::new(8000, 2, Some(16));
        for _ in 0..24000 {
            scan.push(Some(0), Some(0));
        }
        for (l, r) in fixture(16, 0, 0, 100) {
            scan.push(Some(l), Some(r));
        }
        let result = scan.finish();
        assert_eq!(result.scanned_frames, 24000);
        assert_eq!(result.sync_matches, 0);
    }
}
