# F01 ALAC/M4A validation — 2026-10-06

## A04 compatibility correction — 2026-10-08, unreleased

Manual Hot 11S / Android-11 selection exposed two real stereo ALAC containers
rejected by the frozen 0.32.0 desktop core too. Both contain all-zero version-0
`sdtp` dependency entries; the 24-bit stream also has a generic 16-bit audio
sample-entry value. Compatibility now accepts only a single exact-length
all-zero table, bounded by the existing sample limit, with cancellation checks.
Nonzero dependencies remain unsupported. A sample-entry precision of 16 is
accepted for an independently validated 24-bit cookie; all other mismatches and
channel mismatches still fail. The actual decoder/report precision remains the
cookie value, with unchanged bytes passed to Symphonia.

Primary format references: Apple's [dependency table layout](https://developer.apple.com/documentation/quicktime-file-format/sample_dependency_flags_atom/sample_dependency_flags_table)
defines one flag byte per sample; its [ALAC cookie definition](https://github.com/macosforge/alac/blob/master/ALACMagicCookieDescription.txt)
defines decoder source precision. The [FFmpeg development discussion](https://ffmpeg.org/pipermail/ffmpeg-devel/2023-February/307027.html)
documents the ISO audio-entry 16-bit template. No external decoder is a product
dependency; local FFmpeg is solely an independent validation oracle.

Eight locked/offline Rust-1.85 ALAC tests passed, including the existing generated
matrix plus unchanged exact PCM hashes after dependency-table insertion and
generic-entry mutations, full/prefix controls, invalid versions/lengths,
nonzero flags and arbitrary precision rejection. Both private originals now
probe successfully; exact one-second native signed-s32le PCM hashes match FFmpeg
at 44,100 and 48,000 frames. Original encoded hashes are unchanged. Private
receipts stay ignored `target/a04/private-alac/`; no recordings or their names/
hashes are in source history. Whole private-track decode was not run. Fresh
Android ABI/APK acceptance and owner replacement-APK retest remain pending.
Historical F01/P09 acceptance below retains its original scope and binary.

Engine **0.28.0**, measurement schema **0.18.0**, unchanged
`observations-only-v18` policy. **DONE 2026-10-06**, including post-fix optimized
tests, serialized acceptance and preservation checks after resuming the
usage-limit checkpoint. See [F01 results](../../task-results/F01.md).

## Input and source contract

Pure Rust Symphonia **0.5.5** `alac`/`isomp4` features extend the existing
path/source APIs. These are the only added dependency crates, both MPL-2.0;
there is no runtime FFmpeg, platform decoder, network or account requirement.
Primary source: [Symphonia 0.5.5 feature inventory](https://docs.rs/crate/symphonia/0.5.5/source/README.md),
and the downloaded versioned ALAC/MP4 source in the local Cargo registry.

`mp4::read` checks a single unencrypted, nonfragmented ALAC audio track,
16/24-bit mono/stereo, inclusive 8–384 kHz. `moov` may precede or follow
`mdat`; sample-entry versions 0/1, 32/64-bit box sizes and chunk offsets are explicit. The selected
public track ID is **0**, Symphonia's track index, rather than the physical
`tkhd` identifier. Actual codec comes from `stsd` plus its ALAC cookie, never
the filename. AAC/Opus/DRM, video/multiple tracks, fragmentation, nonidentity
edits, external data references and unsupported layouts remain structured
unsupported. Identity edits preserve the entire native PCM interval.

The media timescale must equal the native rate, as required by this analysis
domain. Native cookie rate/precision/channels fill fields that the locked MP4
reader otherwise leaves absent or reads from a rate-limited 16.16 sample entry.
The cookie obtained by the decoder probe must equal the preflight cookie before
decoder allocation. Decoder panics from corrupt partial/predictor packet lengths
are contained as structured decode errors under the supported default unwind
build. This guard wraps only third-party ALAC decoding, preserving caller-I/O and
callback unwind behavior. A01 must retain an unwind build or replace this decoder
boundary before adopting panic=abort; this is not an exhaustive hostile-input
security audit. PCM remains exact MSB-aligned signed s32le; no resampling,
gain, downmix or lossy decoder is introduced. Packet timestamps and decoded
durations must agree with native frames. Two passes must retain exact PCM hash,
frame count and representation; an optional reference-input pass uses the same
source/interval. ALAC has **no embedded PCM checksum**: decoder verification is
null, independently of exact cross-pass/hash and container checks.

Sources retain the existing worker, guarded short-read/I/O, cancellation and
cooperative deadline behavior. Preflight discovers the actual extent by seeking
to end, then rewinds. For a stable seekable source with unknown `byte_len`, the
guard exposes this discovered extent to the MP4 reader; metadata still labels
the caller's originally unknown file-size field null. No whole-file staging
occurs inside the library. Prefix requests use the same half-open PCM interval
in every pass; complete container geometry is checked even for a prefix.

## Bounded parsing and metadata

Preflight skips `mdat` and padding through seeks, retaining one **16 MiB maximum
moov**. It caps traversal at **4096 boxes**, metadata nesting at **8**, each
table/total packet count at **1,048,576**, ALAC block length at **65,536 frames**,
and each encoded packet at **1 MiB**. Text/freeform fields are capped at **1 MiB**
and artwork at **8 MiB** before report retention. Counts must fit their actual payload
before the upstream reader allocates; timing, size and chunk tables must agree,
every chunk must fit an `mdat`, and chunks cannot overlap. No buffer is sized
from declared track duration. The upstream reader's bounded sample tables do
scale with packet/chunk count up to these explicit caps; this differs from the
fixed native DSP buffers. These limits are support conditions, not exhaustive
fuzzing or a total-process memory guarantee.

P02 metadata version **1** retains iTunes `ilst` atom identifiers (Latin-1
fourcc rendered as UTF-8), duplicate values/order, UTF-8 text, unknown keys and
freeform **mean:name** namespaces. Named title/album/date/artist/album-artist,
BPM, comments, writing library and iTunes ReplayGain refer to raw entry indices.
Movie/track/media-level metadata containers are traversed with the same limits.
Signed/unsigned variable integers are displayed as decimal text. Original
encoded lengths remain distinct from decoded UTF-8 text budgets: an atom ID is
four wire bytes, a two-byte BPM value remains two encoded bytes, and freeform
key length is the sum of the mean/name bytes, excluding the rendered separator.
Artwork stays
a byte-length descriptor with null dimensions/precision where unparsed; image
payload never becomes report text/base64. UTF16, unknown binary types and
unrecognized ancillary structures retain explicit opaque descriptors, making
negative metadata text searches incomplete. These are finite adapters, not a
claim to parse every binary metadata convention. The existing 1024-entry,
256-byte key, 16 KiB value and 1 MiB retained-text caps and omission accounting
apply. Editable tags cannot affect ancestry or evidence index.

## Frozen generated controls and checks

`scripts/generate_alac_fixtures.py` freezes original signed integer PCM before
encoding. Public fixtures contain generated signals only, including extrema,
zero and low-bit activity, distinct native stereo lanes and a partial last
packet. Nine rates: 8/44.1/48/88.2/96/176.4/192/352.8/384 kHz; both precisions,
both channel counts and both movie layouts give **72 controls**. Every control
contains 5001 frames; prefix hashes independently bind the first 1000 frames.
FFmpeg **7.1.1** encoded each fixture and its independent signed32 decode matched
the original canonical bytes exactly before Rust validation. `ffprobe` also
confirmed actual ALAC identity, rate, precision and channels. Frozen hashes,
encoder commands and tool version are in `tests/fixtures/alac/manifest.json`.

Initial Rust matrix passed exact full/prefix PCM for all 72 controls. Its first
source test exposed the locked MP4 reader's known-length requirement; the
discovered-length adapter addresses that finding. Initial compile omitted the
new context field and was corrected. A sandboxed link was denied permission to
execute the installed LLVM linker; the authorized offline retry compiled and
ran. Later final-source checks passed **96 optimized no-CLI tests**, MSRV
all-target Clippy with/without CLI, CLI/consumer build, ARM64 Android target
compilation, unchanged schema reproduction and formatting/whitespace. Final CLI
serialization initially passed **72 full + 18 prefix reports**, five container
variants and six bounded metadata controls before the actual AAC diagnostic
assertion failed: the ALAC table whitelist preceded codec identity, so AAC's
sgpd/sbgp tables reported a scope error. Moving that unchanged whitelist after
the actual ALAC check corrected identification without extending ALAC scope.
The completion helper passed all those controls plus actual AAC `not ALAC`
rejection/null coverage, a sorted mixed AAC/ALAC M4A directory, the optional
third-pass reference-input schema/exact hash, and a **734003200-byte (700 MiB)**
generated padding/seek control with unchanged PCM hash. Frozen fixture hashes
are unchanged. Receipts: `target/f01-alac/serialized-final/summary.json`,
`reference-inputs.json`, `full.json`, `prefix.json`, `variants.json` and
`metadata.json`. Post-fix MSRV all-target Clippy with/without CLI, Android
target compilation, CLI/consumer build, schema reproduction, format/helper
syntax and parity inventory passed. The build's sandbox linker execution
failure was resolved by an authorized offline retry. The post-fix optimized
Rust 1.85 no-CLI suite passed **96 tests**, zero failed/ignored: 50 library,
7 ALAC, 10 FLAC, 10 metadata, 2 parity, 1 serialization, 10 source and 6 WAV.
Receipt: `target/f01-alac/completion-tests.log`. Final preservation compares
274 prior files: 258 unchanged and 16 intentional edits, zero missing or
unexpected changes; all 48 private recording/note files, owner checklist,
Git index and clean pinned reference remain unchanged. Receipt:
`target/f01-alac/completion-preservation.json`. No jobs remain running.

Local logs/serialized controls are ignored under `target/f01-alac/`; these are
local evidence, not Git history or an external backup. No private recording
analysis, commit, push or upload is required. Android target compilation is
separate from NDK linking, an APK or phone behavior.
