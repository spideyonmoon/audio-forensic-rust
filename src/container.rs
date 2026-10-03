//! Bound ancillary container work before the decoder parses untrusted lengths.
use std::io::SeekFrom;
use symphonia::core::io::MediaSource;

const MAX_METADATA: u64 = 16 * 1024 * 1024;
const MAX_FIELD: usize = 1024 * 1024;
const MAX_PICTURE: usize = 8 * 1024 * 1024;
const MAX_BLOCKS: usize = 1024;
const MAX_RECORDS: usize = 4096;

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("Malformed container metadata: {0}")]
    Invalid(&'static str),
    #[error("Container support limit: {0}")]
    Unsupported(&'static str),
    #[error("Container preflight I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("Container preflight interrupted")]
    Interrupted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FlacInfo {
    pub audio_offset: u64,
    pub stream_info: [u8; 34],
}

pub(crate) fn validate(
    source: &mut dyn MediaSource,
    mut control: impl FnMut() -> bool,
) -> Result<Option<FlacInfo>, Error> {
    source.seek(SeekFrom::Start(0))?;
    let mut marker = [0; 4];
    source.read_exact(&mut marker)?;
    let info = match &marker {
        b"fLaC" => Some(flac(source, &mut control)?),
        b"RIFF" => {
            wave(source, &mut control)?;
            None
        }
        b"RF64" | b"BW64" => return Err(Error::Unsupported("RF64/BW64 WAV is not supported")),
        b"RIFX" => return Err(Error::Unsupported("big-endian RIFX WAV is not supported")),
        _ => {
            return Err(Error::Unsupported(
                "native FLAC or RIFF/WAVE signature required",
            ));
        }
    };
    source.seek(SeekFrom::Start(0))?;
    Ok(info)
}

fn check(control: &mut impl FnMut() -> bool) -> Result<(), Error> {
    if control() {
        Ok(())
    } else {
        Err(Error::Interrupted)
    }
}
fn charge(total: &mut u64, bytes: u64) -> Result<(), Error> {
    *total += bytes;
    if *total > MAX_METADATA {
        return Err(Error::Unsupported("ancillary metadata exceeds 16 MiB"));
    }
    Ok(())
}
fn records(total: &mut usize, count: usize) -> Result<(), Error> {
    *total += count;
    if *total > MAX_RECORDS {
        return Err(Error::Unsupported("metadata records exceed 4096"));
    }
    Ok(())
}
fn checked_end(source: &dyn MediaSource, start: u64, len: u64) -> Result<u64, Error> {
    let end = start
        .checked_add(len)
        .ok_or(Error::Invalid("offset overflow"))?;
    if source.byte_len().is_some_and(|n| end > n) {
        return Err(Error::Invalid("truncated ancillary block"));
    }
    Ok(end)
}

struct Fields<'a>(&'a [u8]);
impl<'a> Fields<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        if n > self.0.len() {
            return Err(Error::Invalid("embedded length exceeds block"));
        }
        let (head, tail) = self.0.split_at(n);
        self.0 = tail;
        Ok(head)
    }
    fn number(&mut self, big: bool) -> Result<usize, Error> {
        let bytes: [u8; 4] = self.take(4)?.try_into().unwrap();
        Ok(if big {
            u32::from_be_bytes(bytes)
        } else {
            u32::from_le_bytes(bytes)
        } as usize)
    }
    fn blob(&mut self, big: bool, cap: usize) -> Result<&'a [u8], Error> {
        let n = self.number(big)?;
        if n > self.0.len() {
            return Err(Error::Invalid("embedded length exceeds block"));
        }
        if n > cap {
            return Err(Error::Unsupported("metadata field exceeds supported size"));
        }
        self.take(n)
    }
}
fn picture(bytes: &[u8]) -> Result<(), Error> {
    let mut fields = Fields(bytes);
    fields.take(4)?;
    fields.blob(true, MAX_FIELD)?;
    fields.blob(true, MAX_FIELD)?;
    fields.take(16)?;
    fields.blob(true, MAX_PICTURE)?;
    Ok(())
}

// Decode only the bounded special comment that Symphonia interprets as a picture.
fn base64(bytes: &[u8]) -> Result<Vec<u8>, Error> {
    let symbols = bytes
        .strip_suffix(b"==")
        .or_else(|| bytes.strip_suffix(b"="))
        .unwrap_or(bytes);
    if symbols.len() % 4 == 1 {
        return Err(Error::Invalid("base64 picture length"));
    }
    let mut out = Vec::with_capacity(symbols.len() * 3 / 4);
    let mut bits = 0u32;
    let mut count = 0;
    for x in symbols {
        let value = match x {
            b'A'..=b'Z' => x - b'A',
            b'a'..=b'z' => x - b'a' + 26,
            b'0'..=b'9' => x - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return Err(Error::Invalid("base64 picture symbol")),
        };
        bits = (bits << 6) | u32::from(value);
        count += 6;
        if count >= 8 {
            count -= 8;
            out.push((bits >> count) as u8);
        }
    }
    Ok(out)
}
fn comments(bytes: &[u8], total_records: &mut usize) -> Result<(), Error> {
    let mut fields = Fields(bytes);
    fields.blob(false, MAX_FIELD)?;
    let count = fields.number(false)?;
    if count > fields.0.len() / 4 {
        return Err(Error::Invalid("comment count exceeds block"));
    }
    records(total_records, count)?;
    for _ in 0..count {
        let comment = String::from_utf8_lossy(fields.blob(false, MAX_FIELD)?);
        if let Some((key, value)) = comment.split_once('=') {
            // Match the dependency's Unicode lowercase handling, including K.
            if key.to_lowercase() == "metadata_block_picture" {
                picture(&base64(value.as_bytes())?)?;
            }
        }
    }
    Ok(())
}
fn cues(bytes: &[u8], total_records: &mut usize) -> Result<(), Error> {
    let mut fields = Fields(bytes);
    fields.take(395)?;
    let tracks = fields.take(1)?[0];
    records(total_records, usize::from(tracks))?;
    for _ in 0..tracks {
        fields.take(35)?;
        let indices = fields.take(1)?[0];
        records(total_records, usize::from(indices))?;
        fields.take(usize::from(indices) * 12)?;
    }
    Ok(())
}
fn flac(
    source: &mut dyn MediaSource,
    control: &mut impl FnMut() -> bool,
) -> Result<FlacInfo, Error> {
    let mut total = 0;
    let mut total_records = 0;
    let mut stream_info = [0; 34];
    for index in 0..MAX_BLOCKS {
        check(control)?;
        let mut header = [0; 4];
        source.read_exact(&mut header)?;
        let kind = header[0] & 127;
        let len = u32::from_be_bytes([0, header[1], header[2], header[3]]) as u64;
        charge(&mut total, len + 4)?;
        let start = source.stream_position()?;
        let end = checked_end(source, start, len)?;
        if (index == 0 && kind != 0) || (kind == 0 && (index != 0 || len != 34)) {
            return Err(Error::Invalid(
                "FLAC requires one leading 34-byte STREAMINFO",
            ));
        }
        match kind {
            0 | 4 | 5 | 6 => {
                let mut bytes = vec![0; len as usize];
                source.read_exact(&mut bytes)?;
                match kind {
                    0 => stream_info.copy_from_slice(&bytes),
                    4 => comments(&bytes, &mut total_records)?,
                    5 => cues(&bytes, &mut total_records)?,
                    6 => picture(&bytes)?,
                    _ => unreachable!(),
                }
            }
            2 if len < 4 => return Err(Error::Invalid("short application block")),
            3 => {
                if len % 18 != 0 {
                    return Err(Error::Invalid("seek table length"));
                }
                records(&mut total_records, (len / 18) as usize)?;
            }
            127 => return Err(Error::Invalid("reserved FLAC metadata type")),
            _ => {}
        }
        source.seek(SeekFrom::Start(end))?;
        if header[0] & 128 != 0 {
            return Ok(FlacInfo {
                audio_offset: end,
                stream_info,
            });
        }
    }
    Err(Error::Unsupported("ancillary blocks exceed 1024"))
}

fn wave(source: &mut dyn MediaSource, control: &mut impl FnMut() -> bool) -> Result<(), Error> {
    let mut header = [0; 8];
    source.read_exact(&mut header)?;
    if &header[4..] != b"WAVE" {
        return Err(Error::Unsupported("RIFF form is not WAVE"));
    }
    let riff_len = u32::from_le_bytes(header[..4].try_into().unwrap());
    let riff_end = u64::from(riff_len) + 8;
    let mut total = 0;
    let mut total_records = 0;
    let mut frame_bytes = None;
    for _ in 0..MAX_BLOCKS {
        check(control)?;
        let pos = source.stream_position()?;
        if pos + 8 > riff_end {
            return Err(Error::Invalid("missing WAV data chunk"));
        }
        source.read_exact(&mut header)?;
        let len = u32::from_le_bytes(header[4..].try_into().unwrap());
        let start = pos + 8;
        let end = start + u64::from(len);
        // The locked decoder treats this streaming sentinel as a literal frame
        // count, losing a short final packet and inventing a multi-hour duration.
        // Reject it consistently for full and prefix analysis before probing.
        if &header[..4] == b"data" && len == u32::MAX {
            return Err(Error::Unsupported(
                "unknown-length WAV data is not supported",
            ));
        }
        if end > riff_end {
            return Err(Error::Invalid("WAV chunk exceeds RIFF bounds"));
        }
        if &header[..4] == b"data" {
            let frame_bytes = frame_bytes.ok_or(Error::Invalid("WAV data precedes format"))?;
            if len % frame_bytes != 0 {
                return Err(Error::Invalid("WAV data contains a partial PCM frame"));
            }
            return Ok(());
        }
        charge(&mut total, u64::from(len) + 8)?;
        checked_end(source, start, u64::from(len))?;
        match &header[..4] {
            b"fmt " => {
                if frame_bytes.is_some() {
                    return Err(Error::Invalid("duplicate WAV format chunk"));
                }
                if len < 16 {
                    return Err(Error::Invalid("short WAV format chunk"));
                }
                let mut fmt = [0; 16];
                source.read_exact(&mut fmt)?;
                let channels = u16::from_le_bytes([fmt[2], fmt[3]]);
                if channels == 0 {
                    return Err(Error::Invalid("zero WAV channels"));
                }
                if channels > 2 {
                    return Err(Error::Unsupported("mono/stereo WAV required"));
                }
                let format = u16::from_le_bytes([fmt[0], fmt[1]]);
                let bits = u16::from_le_bytes([fmt[14], fmt[15]]);
                if !matches!(format, 1 | 3 | 65534) {
                    return Err(Error::Unsupported("integer/float PCM WAV required"));
                }
                let pcm_format = if format == 65534 {
                    if len < 40 {
                        return Err(Error::Invalid("short extensible WAV format"));
                    }
                    let mut extension = [0; 24];
                    source.read_exact(&mut extension)?;
                    // Check the whole standard GUID: ambisonic/custom layouts
                    // do not share the ordinary mono/stereo measurement basis.
                    let subtype = u32::from_le_bytes(extension[8..12].try_into().unwrap());
                    if !matches!(subtype, 1 | 3)
                        || extension[12..] != [0, 0, 16, 0, 128, 0, 0, 170, 0, 56, 155, 113]
                    {
                        return Err(Error::Unsupported("integer/float PCM WAV subtype required"));
                    }
                    let extra_size = u16::from_le_bytes(extension[..2].try_into().unwrap());
                    if u32::from(extra_size) + 18 > len || extra_size < 22 {
                        return Err(Error::Invalid("extensible WAV extension size"));
                    }
                    if extra_size != 22 {
                        return Err(Error::Unsupported("extensible WAV extension size"));
                    }
                    let valid = u16::from_le_bytes(extension[2..4].try_into().unwrap());
                    if valid == 0 || valid > bits || (subtype == 3 && valid != bits) {
                        return Err(Error::Invalid("extensible WAV valid bits"));
                    }
                    let mask = u32::from_le_bytes(extension[4..8].try_into().unwrap());
                    if !matches!((channels, mask), (1, 0 | 1 | 4) | (2, 0 | 3)) {
                        return Err(Error::Unsupported(
                            "WAV channel mask requires ordinary mono/stereo",
                        ));
                    }
                    subtype as u16
                } else {
                    format
                };
                let supported_width = match pcm_format {
                    1 => matches!(bits, 8 | 16 | 24 | 32),
                    3 => matches!(bits, 32 | 64),
                    _ => unreachable!(),
                };
                if !supported_width {
                    return Err(Error::Unsupported("WAV sample width"));
                }
                let alignment = u16::from_le_bytes([fmt[12], fmt[13]]);
                if alignment != channels * (bits / 8) {
                    return Err(Error::Invalid(
                        "WAV block alignment does not match PCM geometry",
                    ));
                }
                let rate = u32::from_le_bytes(fmt[4..8].try_into().unwrap());
                let byte_rate = u32::from_le_bytes(fmt[8..12].try_into().unwrap());
                if rate.checked_mul(u32::from(alignment)) != Some(byte_rate) {
                    return Err(Error::Invalid("WAV byte rate does not match PCM geometry"));
                }
                frame_bytes = Some(u32::from(alignment));
            }
            b"LIST" => {
                if len < 4 {
                    return Err(Error::Invalid("short WAV list"));
                }
                let mut form = [0; 4];
                source.read_exact(&mut form)?;
                if &form == b"INFO" {
                    while source.stream_position()? + 8 <= end {
                        check(control)?;
                        source.read_exact(&mut header)?;
                        let n = u32::from_le_bytes(header[4..].try_into().unwrap()) as u64;
                        let next = source.stream_position()? + n;
                        if next > end {
                            return Err(Error::Invalid("WAV INFO length exceeds list"));
                        }
                        if n > MAX_FIELD as u64 {
                            return Err(Error::Unsupported("WAV INFO field exceeds 1 MiB"));
                        }
                        records(&mut total_records, 1)?;
                        let padded = next + n % 2;
                        if padded > end {
                            return Err(Error::Invalid("WAV INFO padding exceeds list"));
                        }
                        source.seek(SeekFrom::Start(padded))?;
                    }
                }
            }
            _ => {}
        }
        source.seek(SeekFrom::Start(end + u64::from(len % 2)))?;
    }
    Err(Error::Unsupported("ancillary chunks exceed 1024"))
}
