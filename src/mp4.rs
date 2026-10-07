//! Bounded ISO BMFF preflight for the frozen F01 ALAC scope. Never buffers mdat.
use crate::{
    container::Error,
    metadata::{ArtworkDescriptor, Collector, TechnicalMetadata},
};
use std::io::SeekFrom;
use symphonia::core::io::MediaSource;

const MAX_MOOV: u64 = 16 * 1024 * 1024;
const MAX_BOXES: usize = 4096;
const MAX_SAMPLES: u32 = 1_048_576;
const MAX_PACKET: u32 = 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AlacInfo {
    pub rate: u32,
    pub channels: u32,
    pub bits: u32,
    pub frames: u64,
    pub cookie: [u8; 48],
    pub cookie_len: usize,
    pub source_len: u64,
}

#[derive(Clone, Copy)]
struct Atom<'a> {
    kind: [u8; 4],
    data: &'a [u8],
    offset: u64,
}

fn be32(bytes: &[u8], at: usize) -> Result<u32, Error> {
    Ok(u32::from_be_bytes(
        bytes
            .get(at..at + 4)
            .ok_or(Error::Invalid("short MP4 field"))?
            .try_into()
            .unwrap(),
    ))
}
fn be64(bytes: &[u8], at: usize) -> Result<u64, Error> {
    Ok(u64::from_be_bytes(
        bytes
            .get(at..at + 8)
            .ok_or(Error::Invalid("short MP4 field"))?
            .try_into()
            .unwrap(),
    ))
}
fn check(control: &mut impl FnMut() -> bool) -> Result<(), Error> {
    if control() {
        Ok(())
    } else {
        Err(Error::Interrupted)
    }
}

// The collector's borrow need not share the source/parser lifetime.
fn atoms<'a>(
    bytes: &'a [u8],
    offset: u64,
    count: &mut usize,
    control: &mut impl FnMut() -> bool,
) -> Result<Vec<Atom<'a>>, Error> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        check(control)?;
        *count += 1;
        if *count > MAX_BOXES {
            return Err(Error::Unsupported("MP4 boxes exceed 4096"));
        }
        let size = be32(bytes, at)?;
        let kind = bytes
            .get(at + 4..at + 8)
            .ok_or(Error::Invalid("short MP4 box"))?
            .try_into()
            .unwrap();
        let (size, head) = if size == 1 {
            (be64(bytes, at + 8)?, 16)
        } else {
            (u64::from(size), 8)
        };
        if size < head {
            return Err(Error::Invalid("MP4 box size is smaller than header"));
        }
        let size = usize::try_from(size).map_err(|_| Error::Invalid("MP4 box size overflow"))?;
        let end = at
            .checked_add(size)
            .filter(|&n| n <= bytes.len())
            .ok_or(Error::Invalid("MP4 child exceeds parent"))?;
        out.push(Atom {
            kind,
            data: &bytes[at + head as usize..end],
            offset: offset + at as u64 + head,
        });
        at = end;
    }
    Ok(out)
}
fn one<'a>(list: &[Atom<'a>], kind: &[u8; 4]) -> Result<Atom<'a>, Error> {
    let mut found = list.iter().filter(|b| &b.kind == kind);
    let value = *found
        .next()
        .ok_or(Error::Invalid("missing required MP4 box"))?;
    if found.next().is_some() {
        return Err(Error::Invalid("duplicate required MP4 box"));
    }
    Ok(value)
}
fn full(bytes: &[u8]) -> Result<(), Error> {
    if be32(bytes, 0)? != 0 {
        return Err(Error::Unsupported("MP4 full-box version/flags unsupported"));
    }
    Ok(())
}
fn table(bytes: &[u8], stride: usize) -> Result<u32, Error> {
    full(bytes)?;
    let count = be32(bytes, 4)?;
    if count == 0 {
        return Err(Error::Invalid("empty MP4 sample table"));
    }
    if count > MAX_SAMPLES {
        return Err(Error::Unsupported("MP4 table exceeds 1048576 entries"));
    }
    if bytes.len() != 8 + count as usize * stride {
        return Err(Error::Invalid("MP4 table count/length mismatch"));
    }
    Ok(count)
}

pub(crate) fn read(
    source: &mut dyn MediaSource,
    control: &mut impl FnMut() -> bool,
    mut sink: Option<&mut Collector<'_>>,
) -> Result<AlacInfo, Error> {
    check(control)?;
    let end = source.seek(SeekFrom::End(0))?;
    if source.byte_len().is_some_and(|n| n != end) {
        return Err(Error::Invalid("MP4 source length changed"));
    }
    let mut at = 0u64;
    let mut count = 0;
    let mut moov = None;
    let mut media = Vec::new();
    while at < end {
        check(control)?;
        count += 1;
        if count > MAX_BOXES {
            return Err(Error::Unsupported("MP4 boxes exceed 4096"));
        }
        source.seek(SeekFrom::Start(at))?;
        let mut header = [0; 8];
        source.read_exact(&mut header)?;
        let size = u32::from_be_bytes(header[..4].try_into().unwrap());
        let kind: [u8; 4] = header[4..].try_into().unwrap();
        let (size, head) = if size == 1 {
            let mut large = [0; 8];
            source.read_exact(&mut large)?;
            (u64::from_be_bytes(large), 16)
        } else if size == 0 {
            (end - at, 8)
        } else {
            (u64::from(size), 8)
        };
        let next = at
            .checked_add(size)
            .filter(|&n| n <= end)
            .ok_or(Error::Invalid("truncated MP4 box or offset overflow"))?;
        if size < head {
            return Err(Error::Invalid("MP4 box size is smaller than header"));
        }
        let payload = size - head;
        match &kind {
            b"ftyp" => {
                if at != 0 || !(8..=1024).contains(&payload) || payload % 4 != 0 {
                    return Err(Error::Invalid("invalid MP4 ftyp"));
                }
            }
            b"moov" => {
                if moov.is_some() {
                    return Err(Error::Invalid("multiple MP4 moov boxes"));
                }
                if payload > MAX_MOOV {
                    return Err(Error::Unsupported("MP4 moov exceeds 16 MiB"));
                }
                let mut bytes = vec![0; payload as usize];
                source.read_exact(&mut bytes)?;
                moov = Some((bytes, at + head));
            }
            b"mdat" => media.push((at + head, next)),
            b"free" | b"skip" | b"wide" => {}
            b"moof" | b"mfra" | b"sidx" => {
                return Err(Error::Unsupported("fragmented MP4 is unsupported"));
            }
            _ => return Err(Error::Unsupported("MP4 top-level box outside F01 scope")),
        }
        at = next;
    }
    if media.is_empty() {
        return Err(Error::Invalid("MP4 mdat missing"));
    }
    let (bytes, offset) = moov.ok_or(Error::Invalid("MP4 moov missing"))?;
    let movie = atoms(&bytes, offset, &mut count, control)?;
    if movie.iter().any(|a| &a.kind == b"mvex") {
        return Err(Error::Unsupported("fragmented MP4 is unsupported"));
    }
    let tracks: Vec<_> = movie.iter().filter(|a| &a.kind == b"trak").collect();
    if tracks.len() != 1 {
        return Err(Error::Unsupported(
            "MP4 requires one audio track; video/multiple tracks unsupported",
        ));
    }
    let track = atoms(tracks[0].data, tracks[0].offset, &mut count, control)?;
    let mdia = one(&track, b"mdia")?;
    let mdia = atoms(mdia.data, mdia.offset, &mut count, control)?;
    let handler = one(&mdia, b"hdlr")?;
    if handler.data.get(8..12) != Some(b"soun") {
        return Err(Error::Unsupported("MP4 track is not audio"));
    }
    let mdhd = one(&mdia, b"mdhd")?;
    let (timescale, frames) = match be32(mdhd.data, 0)? {
        0 => (be32(mdhd.data, 12)?, u64::from(be32(mdhd.data, 16)?)),
        0x01000000 => (be32(mdhd.data, 20)?, be64(mdhd.data, 24)?),
        _ => return Err(Error::Unsupported("MP4 mdhd version/flags unsupported")),
    };
    let minf = one(&mdia, b"minf")?;
    let minf = atoms(minf.data, minf.offset, &mut count, control)?;
    let dinf = one(&minf, b"dinf")?;
    let dinf = atoms(dinf.data, dinf.offset, &mut count, control)?;
    let dref = one(&dinf, b"dref")?;
    full(dref.data)?;
    if be32(dref.data, 4)? != 1 {
        return Err(Error::Unsupported(
            "MP4 external/multiple data references unsupported",
        ));
    }
    let refs = atoms(&dref.data[8..], dref.offset + 8, &mut count, control)?;
    if refs.len() != 1 || refs[0].kind != *b"url " || refs[0].data != [0, 0, 0, 1] {
        return Err(Error::Unsupported(
            "MP4 external data references unsupported",
        ));
    }
    let stbl = one(&minf, b"stbl")?;
    let stbl = atoms(stbl.data, stbl.offset, &mut count, control)?;
    let stsd = one(&stbl, b"stsd")?;
    full(stsd.data)?;
    if be32(stsd.data, 4)? != 1 {
        return Err(Error::Unsupported("MP4 requires one sample description"));
    }
    let entries = atoms(&stsd.data[8..], stsd.offset + 8, &mut count, control)?;
    if entries.len() != 1 {
        return Err(Error::Invalid("MP4 sample description count mismatch"));
    }
    let entry = entries[0];
    if entry.kind != *b"alac" {
        return Err(Error::Unsupported(
            "M4A codec is not ALAC (AAC/Opus/DRM unsupported)",
        ));
    }
    // Codec identity takes precedence over codec-specific table/edit checks
    // (ordinary AAC carries sample groups and a nonzero priming edit).
    for atom in &stbl {
        if ![
            *b"stsd", *b"stts", *b"stsc", *b"stsz", *b"stco", *b"co64", *b"stss", *b"ctts",
        ]
        .contains(&atom.kind)
        {
            return Err(Error::Unsupported("MP4 sample table outside F01 scope"));
        }
    }
    if track.iter().any(|a| &a.kind == b"edts") {
        let edts = one(&track, b"edts")?;
        let edits = atoms(edts.data, edts.offset, &mut count, control)?;
        let elst = one(&edits, b"elst")?;
        let version = be32(elst.data, 0)?;
        let identity = match version {
            0 => {
                elst.data.len() == 20
                    && be32(elst.data, 4)? == 1
                    && be32(elst.data, 12)? == 0
                    && be32(elst.data, 16)? == 65536
            }
            0x01000000 => {
                elst.data.len() == 28
                    && be32(elst.data, 4)? == 1
                    && be64(elst.data, 16)? == 0
                    && be32(elst.data, 24)? == 65536
            }
            _ => false,
        };
        if !identity {
            return Err(Error::Unsupported(
                "MP4 nonidentity edit list is unsupported",
            ));
        }
    }
    if entry.data.len() < 28 {
        return Err(Error::Invalid("short ALAC sample entry"));
    }
    let version = u16::from_be_bytes(entry.data[8..10].try_into().unwrap());
    if entry.data[6..8] != [0, 1] || version > 1 {
        return Err(Error::Unsupported(
            "ALAC sample entry version/data reference unsupported",
        ));
    }
    let header = if version == 0 { 28 } else { 44 };
    if entry.data.len() < header {
        return Err(Error::Invalid("short ALAC sample entry extension"));
    }
    let configs = atoms(
        &entry.data[header..],
        entry.offset + header as u64,
        &mut count,
        control,
    )?;
    if configs.iter().any(|a| &a.kind == b"sinf") {
        return Err(Error::Unsupported("encrypted MP4 is unsupported"));
    }
    for extra in &configs {
        match &extra.kind {
            b"alac" => {}
            b"btrt" if extra.data.len() == 12 => {}
            b"chan" => {
                full(extra.data)?;
                if extra.data.len() != 16
                    || ![0x640001, 0x650002].contains(&be32(extra.data, 4)?)
                    || be64(extra.data, 8)? != 0
                {
                    return Err(Error::Unsupported(
                        "ALAC explicit channel layout outside mono/stereo scope",
                    ));
                }
            }
            _ => {
                return Err(Error::Unsupported(
                    "ALAC extra configuration outside F01 scope",
                ));
            }
        }
    }
    let cookie = one(&configs, b"alac")?;
    full(cookie.data)?;
    if cookie.data.len() != 28 && cookie.data.len() != 52 {
        return Err(Error::Invalid("ALAC cookie length"));
    }
    let c = &cookie.data[4..];
    let block = be32(c, 0)?;
    let bits = u32::from(c[5]);
    let channels = u32::from(c[9]);
    let rate = be32(c, 20)?;
    if c[4] != 0
        || ![16, 24].contains(&bits)
        || !(1..=2).contains(&channels)
        || !(8000..=384000).contains(&rate)
    {
        return Err(Error::Unsupported(
            "ALAC requires version 0, 16/24-bit mono/stereo at 8-384 kHz",
        ));
    }
    if u32::from(u16::from_be_bytes(entry.data[16..18].try_into().unwrap())) != channels
        || u32::from(u16::from_be_bytes(entry.data[18..20].try_into().unwrap())) != bits
    {
        return Err(Error::Invalid(
            "ALAC sample entry and cookie precision/channels disagree",
        ));
    }
    for layout in configs.iter().filter(|a| a.kind == *b"chan") {
        if be32(layout.data, 4)? != (if channels == 1 { 0x640001 } else { 0x650002 }) {
            return Err(Error::Invalid("ALAC cookie/channel layout mismatch"));
        }
    }
    if block == 0 || block > 65536 || be32(c, 12)? > MAX_PACKET {
        return Err(Error::Unsupported(
            "ALAC block/packet exceeds bounded decode limits",
        ));
    }
    if c.len() == 48
        && (be32(c, 24)? != 24
            || &c[28..32] != b"chan"
            || be32(c, 32)? != 0
            || be32(c, 36)? != (if channels == 1 { 0x640001 } else { 0x650002 })
            || be64(c, 40)? != 0)
    {
        return Err(Error::Unsupported(
            "ALAC channel layout outside mono/stereo scope",
        ));
    }
    if timescale != rate {
        return Err(Error::Unsupported(
            "ALAC media timescale must equal native sample rate",
        ));
    }
    if frames == 0 {
        return Err(Error::Invalid("ALAC declares zero frames"));
    }
    validate_tables(&stbl, block, frames, &media, control)?;
    let mut config = [0; 48];
    config[..c.len()].copy_from_slice(c);
    let info = AlacInfo {
        rate,
        channels,
        bits,
        frames,
        cookie: config,
        cookie_len: c.len(),
        source_len: end,
    };
    if let Some(s) = sink.as_deref_mut() {
        s.report.technical = Some(TechnicalMetadata {
            container: "m4a".into(),
            codec: "alac".into(),
            selected_track_id: 0,
            declared_sample_rate_hz: Some(rate),
            declared_channels: Some(channels),
            declared_precision_bits: Some(bits),
            sample_encoding: Some("native_integer_pcm".into()),
            declared_frames: Some(frames),
            declared_duration_seconds: Some(frames as f64 / f64::from(rate)),
            declared_bit_rate_bps: (be32(c, 16)? != 0).then_some(u64::from(be32(c, 16)?)),
            derived_pcm_bit_rate_bps: Some(u64::from(rate) * u64::from(channels) * u64::from(bits)),
            compression_mode: Some("lossless".into()),
            unavailable_fields: vec![
                "storage_bits_per_sample: compressed ALAC has no fixed PCM storage width".into(),
                "format_profile: no native ALAC profile field".into(),
            ],
            ..Default::default()
        });
    }
    metadata(&movie, &mut count, control, &mut sink, 0)?;
    source.seek(SeekFrom::Start(0))?;
    Ok(info)
}

fn validate_tables(
    stbl: &[Atom<'_>],
    block: u32,
    frames: u64,
    media: &[(u64, u64)],
    control: &mut impl FnMut() -> bool,
) -> Result<(), Error> {
    let sizes = one(stbl, b"stsz")?;
    full(sizes.data)?;
    let fixed = be32(sizes.data, 4)?;
    let samples = be32(sizes.data, 8)?;
    if samples == 0 || samples > MAX_SAMPLES {
        return Err(Error::Unsupported(
            "ALAC sample count outside bounded limit",
        ));
    }
    if sizes.data.len() != 12 + (if fixed == 0 { samples as usize * 4 } else { 0 }) {
        return Err(Error::Invalid("MP4 sample sizes count/length mismatch"));
    }
    let size = |i: u32| -> Result<u32, Error> {
        let n = if fixed == 0 {
            be32(sizes.data, 12 + i as usize * 4)?
        } else {
            fixed
        };
        if n == 0 || n > MAX_PACKET {
            return Err(Error::Unsupported("ALAC packet size outside 1 MiB limit"));
        }
        Ok(n)
    };
    let times = one(stbl, b"stts")?;
    let n = table(times.data, 8)?;
    let mut count_samples = 0u64;
    let mut duration = 0u64;
    for i in 0..n as usize {
        check(control)?;
        let count = be32(times.data, 8 + i * 8)?;
        let delta = be32(times.data, 12 + i * 8)?;
        if count == 0 || delta == 0 || delta > block {
            return Err(Error::Invalid("ALAC invalid packet timing"));
        }
        count_samples += u64::from(count);
        duration += u64::from(count) * u64::from(delta);
        if count_samples > u64::from(MAX_SAMPLES) {
            return Err(Error::Unsupported("ALAC timing sample count exceeds limit"));
        }
    }
    if count_samples != u64::from(samples) || duration != frames {
        return Err(Error::Invalid("ALAC timing/frame/sample count mismatch"));
    }
    if stbl.iter().any(|a| a.kind == *b"stss") {
        let sync = one(stbl, b"stss")?;
        let n = table(sync.data, 4)?;
        let mut prior = 0;
        for i in 0..n as usize {
            check(control)?;
            let sample = be32(sync.data, 8 + i * 4)?;
            if sample <= prior || sample > samples {
                return Err(Error::Invalid("invalid MP4 sync sample table"));
            }
            prior = sample;
        }
    }
    if stbl.iter().any(|a| a.kind == *b"ctts") {
        let offsets = one(stbl, b"ctts")?;
        let n = table(offsets.data, 8)?;
        let mut count = 0u64;
        for i in 0..n as usize {
            check(control)?;
            let entries = be32(offsets.data, 8 + i * 8)?;
            if entries == 0 || be32(offsets.data, 12 + i * 8)? != 0 {
                return Err(Error::Unsupported(
                    "ALAC nonzero composition offsets unsupported",
                ));
            }
            count += u64::from(entries);
        }
        if count != u64::from(samples) {
            return Err(Error::Invalid("MP4 composition sample count mismatch"));
        }
    }
    let offsets = match (
        stbl.iter().any(|a| a.kind == *b"stco"),
        stbl.iter().any(|a| a.kind == *b"co64"),
    ) {
        (true, false) => one(stbl, b"stco")?,
        (false, true) => one(stbl, b"co64")?,
        _ => {
            return Err(Error::Invalid(
                "MP4 requires exactly one chunk offset table",
            ));
        }
    };
    let stride = if offsets.kind == *b"co64" { 8 } else { 4 };
    let chunks = table(offsets.data, stride)?;
    let mapping = one(stbl, b"stsc")?;
    let maps = table(mapping.data, 12)?;
    let mut sample = 0u32;
    let mut map_index = 0usize;
    let mut prior_end = 0;
    for i in 0..maps as usize {
        check(control)?;
        let first = be32(mapping.data, 8 + i * 12)?;
        if first == 0
            || first > chunks
            || (i == 0 && first != 1)
            || (i > 0 && first <= be32(mapping.data, 8 + (i - 1) * 12)?)
            || be32(mapping.data, 12 + i * 12)? == 0
            || be32(mapping.data, 16 + i * 12)? != 1
        {
            return Err(Error::Invalid("invalid MP4 sample-to-chunk mapping"));
        }
    }
    for chunk in 1..=chunks {
        check(control)?;
        if map_index + 1 < maps as usize && be32(mapping.data, 8 + (map_index + 1) * 12)? == chunk {
            map_index += 1;
        }
        let per = be32(mapping.data, 12 + map_index * 12)?;
        let next = sample
            .checked_add(per)
            .filter(|&n| n <= samples)
            .ok_or(Error::Invalid("MP4 chunk sample count overflow/mismatch"))?;
        let mut bytes = 0u64;
        for i in sample..next {
            check(control)?;
            bytes += u64::from(size(i)?);
        }
        let at = if stride == 8 {
            be64(offsets.data, 8 + (chunk as usize - 1) * 8)?
        } else {
            u64::from(be32(offsets.data, 8 + (chunk as usize - 1) * 4)?)
        };
        let end = at
            .checked_add(bytes)
            .ok_or(Error::Invalid("MP4 chunk offset overflow"))?;
        if at < prior_end
            || !media
                .iter()
                .any(|&(start, limit)| at >= start && end <= limit)
        {
            return Err(Error::Invalid("MP4 chunk overlaps or exceeds mdat"));
        }
        prior_end = end;
        sample = next;
    }
    if sample != samples {
        return Err(Error::Invalid("MP4 chunk/sample count mismatch"));
    }
    Ok(())
}

fn metadata(
    list: &[Atom<'_>],
    count: &mut usize,
    control: &mut impl FnMut() -> bool,
    sink: &mut Option<&mut Collector<'_>>,
    depth: usize,
) -> Result<(), Error> {
    if depth > 8 {
        return Err(Error::Unsupported("MP4 metadata nesting exceeds 8"));
    }
    for a in list {
        match &a.kind {
            b"trak" | b"mdia" | b"minf" => {
                let children = atoms(a.data, a.offset, count, control)?;
                metadata(&children, count, control, sink, depth + 1)?;
            }
            b"udta" => {
                let children = atoms(a.data, a.offset, count, control)?;
                metadata(&children, count, control, sink, depth + 1)?;
            }
            b"meta" => {
                full(a.data)?;
                let children = atoms(&a.data[4..], a.offset + 4, count, control)?;
                for ilst in children.iter().filter(|a| a.kind == *b"ilst") {
                    let tags = atoms(ilst.data, ilst.offset, count, control)?;
                    for (order, tag) in tags.iter().enumerate() {
                        tag_metadata(*tag, order, count, control, sink)?;
                    }
                }
                for unknown in children
                    .iter()
                    .filter(|a| a.kind != *b"ilst" && a.kind != *b"hdlr")
                {
                    if let Some(s) = sink.as_deref_mut() {
                        s.opaque(
                            "m4a_meta",
                            unknown.offset,
                            unknown.data.len() as u64,
                            "unparsed MP4 metadata box",
                        );
                    }
                }
            }
            b"mvhd" | b"tkhd" | b"edts" | b"mdhd" | b"hdlr" | b"smhd" | b"dinf" | b"stbl" => {}
            _ => {
                if let Some(s) = sink.as_deref_mut() {
                    s.opaque(
                        "m4a_ancillary",
                        a.offset,
                        a.data.len() as u64,
                        "unparsed MP4 ancillary box",
                    );
                }
            }
        }
    }
    Ok(())
}

fn tag_metadata(
    tag: Atom<'_>,
    order: usize,
    count: &mut usize,
    control: &mut impl FnMut() -> bool,
    sink: &mut Option<&mut Collector<'_>>,
) -> Result<(), Error> {
    let fields = atoms(tag.data, tag.offset, count, control)?;
    let mut key: Vec<u8> = tag
        .kind
        .iter()
        .map(|&b| char::from(b))
        .collect::<String>()
        .into_bytes();
    if tag.kind == *b"----" {
        let mean = one(&fields, b"mean")?;
        let name = one(&fields, b"name")?;
        full(mean.data)?;
        full(name.data)?;
        if mean.data.len() > MAX_PACKET as usize || name.data.len() > MAX_PACKET as usize {
            return Err(Error::Unsupported("MP4 freeform key exceeds 1 MiB"));
        }
        key = mean.data[4..].to_vec();
        key.push(b':');
        key.extend(&name.data[4..]);
    }
    for data in &fields {
        check(control)?;
        if data.kind == *b"mean" || data.kind == *b"name" {
            continue;
        }
        if data.kind != *b"data" {
            if let Some(s) = sink.as_deref_mut() {
                s.opaque(
                    "m4a_ilst",
                    data.offset,
                    data.data.len() as u64,
                    "unparsed tag child",
                );
            }
            continue;
        }
        let dtype = be32(data.data, 0)?;
        be32(data.data, 4)?; // Locale remains in the original encoded box, no guessing.
        let value = &data.data[8..];
        let original_lengths = (
            if tag.kind == *b"----" {
                (key.len() - 1) as u64
            } else {
                4
            },
            value.len() as u64,
        );
        let cap = if tag.kind == *b"covr" && [13, 14].contains(&dtype) {
            8 * 1024 * 1024
        } else {
            MAX_PACKET as usize
        };
        if value.len() > cap {
            return Err(Error::Unsupported(
                "MP4 metadata value exceeds bounded text/artwork limit",
            ));
        }
        if let Some(s) = sink.as_deref_mut() {
            if tag.kind == *b"covr" && [13, 14].contains(&dtype) {
                let index = s.report.artwork.len();
                s.report.artwork.push(ArtworkDescriptor {
                    source: "m4a_ilst".into(),
                    source_offset: data.offset,
                    picture_type: None,
                    width: None,
                    height: None,
                    depth_bits: None,
                    byte_length: value.len() as u64,
                });
                s.tag_encoded(
                    ("m4a_ilst", data.offset, order),
                    &key,
                    value,
                    Some(index),
                    original_lengths,
                );
            } else if dtype == 1 || dtype == 4 {
                s.tag_encoded(
                    ("m4a_ilst", data.offset, order),
                    &key,
                    value,
                    None,
                    original_lengths,
                );
            } else if [21, 22].contains(&dtype) && [1, 2, 4, 8].contains(&value.len()) {
                let mut number = [0u8; 8];
                number[8 - value.len()..].copy_from_slice(value);
                if dtype == 21 && value[0] & 128 != 0 {
                    number[..8 - value.len()].fill(255);
                }
                let text = if dtype == 21 {
                    i64::from_be_bytes(number).to_string()
                } else {
                    u64::from_be_bytes(number).to_string()
                };
                s.tag_encoded(
                    ("m4a_ilst", data.offset, order),
                    &key,
                    text.as_bytes(),
                    None,
                    original_lengths,
                );
            } else {
                s.opaque(
                    "m4a_ilst",
                    data.offset,
                    value.len() as u64,
                    "binary/UTF16/unknown tag type has no text adapter",
                );
            }
        }
    }
    Ok(())
}
