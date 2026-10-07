# P02 metadata and ReplayGain — 2026-10-05

Engine **0.23.0**, metadata/audit version **1**, release contract **1**. This
implements PYTHON_PARITY G01/G02 and the tag-only part of G23. Measurement
schema `0.18.0`, policy `observations-only-v18`, ancestry `INCONCLUSIVE` and
null evidence index are preserved. It does not implement the product verdict,
P07 CLI/info workflow, ALAC/DSD decoding or MQA confirmation.

## API and container coverage

`metadata::read_metadata_path` and `read_metadata_source` return a separate
`MetadataReport`. They use the existing one-worker permit, guarded source,
cooperative deadline and cancellation. Metadata-only extraction constructs no
decoder, performs no DSP, and seeks over WAV PCM or stops before FLAC frames.
`AnalysisOptions.track_id` must be absent/0 for these single-stream containers;
`max_seconds` does not restrict metadata. Metadata cannot establish header/PCM
agreement or full-file checksum integrity. Blocking I/O remains cooperative.

| Container fields | Exposed / unavailable |
| --- | --- |
| Native FLAC | STREAMINFO rate, channels, precision and declared total; zero unknown total stays null; Vorbis vendor and all comments, original case/keys/order/duplicates; native and nested base64 picture descriptors |
| RIFF/WAVE | Validated PCM/extensible format, storage and valid precision, rate/channels, data-frame declaration, derived PCM bitrate; INFO entries before/after data; raw iXML/axml/XMP text; BEXT description/originator/reference/date/time/coding-history text |
| Artwork | Picture type, header dimensions/depth when nonzero and byte length; no image or base64 payload. MIME/description/binary image contents are not copied into tag text |
| Unsupported ancillary structures | Explicit source/offset/length descriptors. WAV ID3, non-INFO LIST and unrecognized chunks have no text adapter; FLAC application/unknown blocks stay opaque. Seek/cue tables retain preflight checks, without parsed public navigation fields |
| Technical absence | No FLAC declared bitrate. WAVE byte rate is exposed as **derived PCM bitrate**, not a header claim of provenance. Format profile is unavailable in these headers; optional tags remain separate. Writing library is a named reference to vendor/encoder/ISFT text, not proof of which tool encoded PCM |
| Future F01/F02 | Reuse `TechnicalMetadata`, `Collector::tag`, descriptor and limit semantics; explicitly bind selected stream/native codec. Their parsers remain separate required packets |

These are finite supported text adapters, not a claim to decode every possible
ancillary/binary metadata structure. Opaque structures make text searches
incomplete; their descriptors prevent silently presenting a complete negative.
F01 extends version 1 to bounded M4A ilst/freeform metadata; see
[ALAC validation](ALAC_VALIDATION.md) for current support and limits. DSF/DFF
remain F02. The P02 checks below are historical WAV/FLAC checks.
Actual native codec comes from the signature/format header, regardless of name.

`named_tags` refers to raw entry indices for title, album, date, album artist,
artist, BPM, comment quality/comments, both ReplayGain gains, writing library
and optional format-profile text. First retained entry wins; duplicates remain
ordered and independently accessible. This explicitly differs from MediaInfo's
merged/projected dictionary. Vendor, BEXT and XML keys label their source fields;
Vorbis/INFO keys retain the actual original key. RIFF INFO removes one terminal
NUL; embedded text characters and empty values remain. Invalid UTF-8 is replaced
and marked, never described as byte-exact decoded text. Its original byte lengths
remain known. Missing fields are absent/null, not fabricated zeroes.

## Resource and text contract

At most **1024 entries**, **256 UTF-8 bytes/key**, **16 KiB/value**, **1 MiB
retained UTF-8 tag text**. Named fields reference entries instead of duplicating
text. Truncation stops at character boundaries and marks the affected entry;
aggregate counters record original encountered entries, omitted entries,
truncated/invalid/malformed entries, decoded UTF-8 bytes retained and omitted.
All counts include skipped text encountered after the output cap. Artwork
base64 is excluded from text counts and represented by an indexed descriptor.
`complete` refers to recognized text, while `text_scan_complete` also checks
opaque structures. Failed/interrupted parsing preserves partial fields with
incomplete status and no successful metadata observations.

The prior preflight limits still apply: 16 MiB encoded ancillary bytes, 1024
blocks/chunks, 4096 records, 1 MiB individual fields and 8 MiB native pictures.
Metadata-only WAV traversal applies them across the container, including trailing
chunks, and rejects multiple data chunks as ambiguous. The existing measurement
preflight still stops at its first data chunk; it has not gained a whole-file
metadata pass. Metadata scratch holds one bounded FLAC block/recognized WAV
field plus bounded output; it does not scale with PCM length. Text searches can
temporarily hold a lowercase copy of at most the retained text (Unicode lowercase
can expand it), plus at most 1024 source spans. No duration-sized buffers exist.

## Encoder/MQA observations and ReplayGain

The pinned encoder signatures, lowercase search, sorting/deduplication and
`mp3tag` exclusion are retained on mapped comments/quality/library/unknown tag
text. Matching entry indices are exposed. The reference extension gate affects
only the raw `legacy_encoder_trace` wording; parsed codec remains independent.
The normal summary says editable encoder text and unknown audio history. MQA
metadata uses the pinned ASCII token boundaries and optional encoder/studio
suffix. It exposes claim/entry references, never a detected/studio/provenance
claim or score override. Failed parsing cannot emit a stale negative/positive.

`audit_replaygain` freezes the pinned pure comparison: first stored token,
decimal-digit/dot/minus filtering, **-18 + stored gain**, absolute difference,
strict **delta <1 / <3 / >=3 dB** branches and exact legacy formatted tuple.
Positive/negative signs, unusual finite tokens and Unicode decimal digits follow
the pinned arithmetic. Nonfinite loudness/gain/arithmetic is explicitly invalid
rather than serialized as a fabricated number or unqualified Python NaN warning.
The old re-encoding wording lives only under `legacy_outputs`; the normal
summary attributes a reference comparison and leaves the cause unknown (D11).

`audit_metadata_replaygain` uses existing native LUFS without inventing full-file
or FFmpeg LUFS. It records method/schema/policy, actual measured frame count,
PCM hash, rate and complete loudness-window interval. Missing loudness/tags and
invalid/truncated selected gains remain explicit; failed reports, differing
source names/track geometry and unknown contracts cannot produce a comparison.
The caller must retain the same source snapshot for both reports: metadata
does not read/hash audio, and matching names/headers alone cannot prove file
identity. A saved audit retains its supplied numeric inputs and measurement
binding; it does not reopen a file. P07 owns integrated source/product workflows.

## Independent expectations and known projection differences

`tests/fixtures/replaygain_reference.json` contains **21** frozen tuples from
the pinned pure Python function, extracted through its AST without executing
audio/subprocess/module initialization. `generate_replaygain_reference.py
--check` compares without overwriting; generation refuses existing files.

`check_metadata.py` creates three generated controls using the unchanged public
noise FLAC fixture and procedural silent WAV. It records actual MediaInfo
24.01 JSON, pinned extractor/encoder output, source hashes and expected values
**before** executing a frozen Rust example. It checks equivalent named fields,
raw duplicates/unknown values, selected codec, rate/channels/precision, duration,
source preservation and an actual 0.5-second native ReplayGain audit. Duration
comparison allows 0.001 s for MediaInfo's millisecond printing; parsed header
frames/duration stay exact. It uses offline JSON Schema validation and rejects
missing fields, unknown version and excessive-entry mutations.

The first comparison found actual extractor projection losses, retained in
`target/p02-metadata/differential-initial/`:

- MediaInfo exposes track gain as Audio `ReplayGain_Gain` and album gain as
  General `Album_ReplayGain_Gain`; the pinned extractor searches differently
  named General/extra fields and returns empty gains. Rust preserves the actual
  raw +/- dB strings. The checker retains the empty Python projection and checks
  both source strings against independently normalized MediaInfo numeric values.
- MediaInfo maps RIFF `IART` to `Director`; the pinned extractor's artist is
  empty. Rust retains IART as artist and raw text. The checker compares against
  Python's retained `other[Director]` plus the generated source value.
- Duplicate keys stay separate rather than being merged into MediaInfo display
  text; unknown keys stay original rather than prettified/truncated at 200 chars.

These are D12 native metadata projection differences, not a changed scoring
rule, updated audio or regenerated oracle. Remaining equivalent fields and the
encoder legacy string are compared exactly. No private recording was analyzed.

## Execution record

Final Rust 1.85 optimized focused regression passed **38 tests**: 10 metadata,
7 container limits, 10 FLAC integrity, 10 source contract and 1 report
serialization. Initial 10-test metadata debug run passed. The broader debug run
passed metadata/container/FLAC/serialization suites but was deliberately stopped
during source tests; it is **incomplete**, not a successful full run. Optimized
checks then completed the same finite set. Final all-target MSRV Clippy with
warnings denied and the no-CLI metadata example build passed. Formatting,
measurement (47 definitions) and metadata/audit (9/3 definitions) schema drift,
21 frozen Python vectors, parity inventory, helper syntax and whitespace passed.

Final `differential-current/summary.json` records **3 generated files**, **1 native
prefix audit**, **4 rejected schema mutations**, both documented MediaInfo
projection differences, exact equivalent fields/encoder text and preserved
source/binary hashes. No reference regeneration or tolerance widening occurred.
Preservation checked **232** prior files; **220** unrelated files unchanged,
12 intentional packet edits, no unexpected changes, all **48** user-recording
files/owner checklist/Git index intact and Python baseline clean. Detailed exact
commands/attempts are in `task-results/P02.md`.

No full regression, private recording analysis, accuracy evaluation, Android
compile/link/device check, commit, push, upload or external backup occurred.
No task jobs remain running. Logs/tool receipts are under ignored
`target/p02-metadata/`; these are local files, not Git history or a backup.
