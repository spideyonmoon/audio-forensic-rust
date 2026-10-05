# FLAC continuity and source rewind — 2026-10-03

Engine **0.18.3** in progress; schema `0.18.0`, policy `observations-only-v18`.
Generated controls only. Native channels, exact integer PCM, prefix scope,
bounded memory and cooperative cancellation remain the acceptance contracts.
Ancestry stays `INCONCLUSIVE`, evidence index null.

## Initial characterization

`scripts/check_flac_integrity.py` independently packs five fixed-block verbatim
FLAC frames (four 256-sample blocks plus a 73-sample tail), with header CRC8,
frame CRC16, interleaved source MD5 and expected MSB-aligned s32le PCM. Mono and
stereo, known/unknown total samples and correct/absent/wrong MD5 are crossed with
frame deletion, header/CRC corruption, duplicate frames, inserted/trailing bytes,
partial tails, complete shorter streams and concatenated streams.

The initial copied 0.18.2 binary retained **204 cases / 408 schema-valid reports**:
full 200 failed / 4 unsupported; prefix 180 failed / 24 unsupported. All ten
intended valid full-file controls match independent FFmpeg PCM exactly. A
separate strict `-err_detect crccheck+explode` FFmpeg decode also passed for the
baseline, and ffprobe confirmed every frame timestamp, size and duration.

Initial ordinary Rust controls passed two of five tests and failed three. Valid
app-provided seven-byte reads with unknown byte length reach the FLAC reader's
second-pass seek, which requires a known byte length despite the source being
seekable. A packet diagnostic additionally isolates valid known-length input:
pass one emits five packets (timestamps 0/256/512/768/1024, durations
256/256/256/256/73); pass two emits one 2,244-byte packet, timestamp 0, duration
256. The locked reader retained its last frame sequence while rewinding, merged
earlier frames, and the decoder decoded only its first frame. The resulting
declared-length/source-change failures are real core defects, not bad fixtures.
Python and ordinary Rust packing were byte-identical. The pinned reference,
existing PCM oracles and original recordings are preserved.

Evidence is ignored under `corpus/local/results/flac-integrity-v18-3-20261003/`.
`before-state.json` fingerprints earlier files, the staged index and pinned
reference. `initial/` preserves all inputs, expectations, helper source, binary,
reports and mismatches. Focused initial test and packet/scan diagnostic logs are
retained; temporary scan logging is removed from the implementation.

## Implementation being verified

The container preflight retains a 34-byte STREAMINFO snapshot and audio-start
offset. After each FLAC scan, the logical buffered source position must equal
that offset plus the sum of accepted packet bytes. This detects fragments the
locked parser silently discards, including partial/corrupt tails at EOF when no
total or MD5 is available. Underlying read-ahead positions are not used.

FLAC packet timestamps must be contiguous from sample zero, and packet duration
must equal the decoded frame count. Missing first/interior frames cannot become
a shorter successful observation merely because the header lacks a total/MD5.
After pass one, the source is rewound directly, metadata bounds are rechecked,
the STREAMINFO/audio offset must match, and a fresh reader is probed. Pass two
also gets a fresh decoder, preserving independent MD5 state.

Both PCM scans remain counted as two analysis passes; the second metadata probe
adds bounded reads and one existing-size stream buffer (64 KiB with the locked
default). No full-audio temporary buffer or additional PCM/DSP pass is added.
WAV retains its existing reader seek. Caller I/O errors remain structured, and
all new reads/seeks use the same source guard and cooperative deadline.

Unknown total samples and absent MD5 are availability conditions, not evidence
of corruption. A complete shorter stream with neither field cannot reveal a
removed whole final frame; its actual EOF coverage is legitimate, with declared
total, header-length comparison and decoder verification null. A prefix does
not verify the whole-stream MD5 or validate later frames. Decoder packet framing
can require bytes beyond the last requested sample; this is not a promise of
independent sample-level corruption validation.

## Packet framing follow-up

The initial rewind/continuity fix passed all 204 cases. Further zero-byte
insertion and hidden valid frames exposed **48 false successes** in the expanded
240-case characterization (`hidden-framing/`). Concatenating valid frame CRCs
or appending zero bytes can preserve the aggregate CRC; the locked decoder
ignores bytes after its first decoded frame. Even a matching PCM MD5 therefore
does not establish complete packet framing. These retained outcomes prompted
the additional fix rather than changing expected outputs.

`src/flac_frame.rs` now checks each packet's complete encoded layout before
decode. It walks constant/verbatim/fixed/LPC subframes, wasted bits, Rice methods,
partitions and escape widths, then requires zero alignment padding and exactly
the final CRC bytes. It reconstructs no samples and allocates no audio buffer.
The layout follows [RFC 9639 sections 9.2–9.3](https://www.rfc-editor.org/rfc/rfc9639.html#section-9.2).
The existing reader/decoder still checks frame headers, CRCs and PCM MD5.
Channels and explicit precision must match the selected stream. Packet length
is limited to 16 MiB and block duration to 65,535 samples; control checks occur
per subframe/partition, every 1,024 residuals and every 4,096 unary bits.
An adversarial unterminated Rice quotient unit verifies deadline propagation.
These cooperative checks do not make caller-blocked I/O interruptible.

## Checks after implementation

- All seven ordinary FLAC integration tests passed on Rust 1.85, including
  unknown-byte-length seven-byte reads, variable blocks/sample numbers,
  damaged/deleted/duplicated frames, extra bytes, prefix scope, metadata identity,
  injected rewind I/O errors, cancellation and worker recovery. The new
  long-unary deadline unit passed separately. Tests require no external tools.
- `all-layout-final/`: **270 generated cases / 540 schema-valid reports** passed;
  full 34 analyzed / 236 failed, prefix 148 analyzed / 122 failed. All 34 intended
  valid full controls match independent exact FFmpeg PCM. Includes hand-packed
  constant, wasted-bit verbatim, fixed Rice4/Rice5, zero-width escape residuals,
  LPC residuals, variable-block headers and nonzero alignment-padding rejection.
- `compression-controls/`: **96 cases / 192 reports** passed, all analyzed,
  with 96 independent strict FFmpeg PCM comparisons. Source signals use 8/16/24
  bits, mono/stereo, 8/48 kHz and encoder levels 0/8; silence, ramps, noise and
  tones exercise common compression. FFmpeg promotes 8-bit source controls to
  16-bit FLAC; tested encoded precision is 16/24, not native 8-bit FLAC.
- MSRV all-target Clippy passed after a test-expression precedence repair;
  the failed first invocation is retained. MSRV no-CLI check and formatting
  passed. A premature regression was interrupted and is not a complete pass.
  Final source/test/helper/schema freeze covers 121 files; full Rust 1.85 debug
  regression passed 161 tests with zero failures/ignored. Python syntax/schema drift and scoped whitespace
  checks passed. Cargo manifest/lock comparison confirms only the root patch
  version changed; dependency/features remain identical.
- The repeat mutation sweep passed all **788 reports**, both baseline PCM hashes
  exact: 56 analyzed / 142 unsupported / 590 failed. All generated input hashes
  match the 0.18.2 batch. The only changed status is FLAC file byte 4, bit 7:
  falsely marking STREAMINFO as the final metadata block previously allowed
  skipped metadata bytes; now it fails accounted logical-byte continuity.
  See `container-mutations/` and `mutation-comparison.json`.
- `source-package-v18-3-20261003/` contains a local 35-file production snapshot.
  Offline MSRV build, no-CLI library check, production Clippy, its own
  **732 schema-valid reports / 130 independent exact PCM controls**, copied-file
  hashes and ZIP integrity passed. Archive size 108,336 bytes; SHA-256
  `60b804f7eb7902b102c0e94f2daed59aae8cce8c5adb624994b831a11ad697b4`.
  This is a local source artifact, excluding tests/fixtures, private evidence,
  toolchains and Git history; it is not an external backup or publication.

## Generated duration/resource controls

The frozen release CLI built offline with Rust 1.98.1 and the installed LLVM
linker. Four generated stereo 16-bit sine recordings were independently packed,
verified against the prior WAV generator, FLAC-encoded locally and analyzed.
Every report passed schema, generated exact PCM SHA-256 and independent FFmpeg
PCM. Receipts include binary/input hashes, raw reports and sampled process peak
working set in `resource-summary.json` and `resource-reports/`.

| Rate | Duration | Runtime | Sampled peak working set |
| --- | --- | --- | --- |
| 48 kHz | 10 seconds | 1.62 s | 24.72 MiB |
| 48 kHz | 120 seconds | 5.31 s | 26.14 MiB |
| 48 kHz | 600 seconds | 19.90 s | 28.01 MiB |
| 384 kHz | 10 seconds | 4.18 s | 78.04 MiB |

Runtime includes detector analysis, not just the frame layout walk. Duration
controls show bounded memory behavior for this desktop batch; rate-dependent
detector workspaces remain larger. Measurements are sampled, include host load
and do not establish a hard RSS cap, Android linking or phone behavior.

No report schema, decoder dependency, detector thresholds or ancestry policy
changed. The release test suite was not rerun for 0.18.3; Android/device/remote
CI checks remain deferred. No private recordings were analyzed.

## Header equivalence and contradiction audit — 2026-10-03

Engine remains 0.18.3. Inspection of the locked decoder showed that its output
buffer uses STREAMINFO's rate, so the output-buffer rate check alone would not
establish frame-header agreement. Generated controls confirm that the existing
reader/core path rejects contradictory frame rates, including when every frame
agrees with every other frame. No production change was necessary.

Three new tests in `tests/flac_integrity.rs` cover:

- 48 successful full/prefix reports: mono/stereo, explicit/inherited 16-bit
  precision, all five encodings of 8 kHz (inherited, fixed code, explicit kHz,
  Hz and tens of Hz), plus a stream mixing those equivalent encodings. Every
  decoded PCM SHA-256 matches the independently generated integer samples.
  Optional encoded-frame byte sizes are cleared because extended headers
  change those sizes; frame/header CRCs are recalculated, not disabled.
- 136 rejected full/prefix reports: mono/stereo, known/unknown sample totals,
  ten other fixed rate codes, three explicit contradictory rates, three zero
  explicit rates and the reserved rate code. Rejected reports have no coverage,
  channel measurements or detectors. An additional two-report regression
  specifically covers uniform 16 kHz frames contradicting 8 kHz STREAMINFO.
- Existing seven FLAC integration tests, including rewind/cancellation and
  hidden-frame/padding controls, still pass.

All ten integration tests passed with Rust 1.85, zero failures/ignored. The
focused Rust 1.85 Clippy check with warnings denied and stable formatting check
also passed. Production files are unchanged in the working-tree diff. The
first focused attempt was blocked by sandbox linker `Permission denied`, before
test execution; an authorized offline retry passed. Logs remain separately in
ignored `corpus/local/results/flac-header-v18-4-20261003/`; the directory's v18-4
label does not indicate a released or implemented engine version. This audit
does not re-run the external FFmpeg matrix, JSON Schema export/validation, full
Rust regression, release tests, resource measurements or Android checks.
The test signals derive from the earlier independently checked packing; these
new header variants were not independently decoded by FFmpeg in this audit.
No production code, schema, dependencies or policy changed. The pinned reference
is clean at `c6ecce2296256b516709d87088896d1be913908c`. No private recordings
were analyzed and no files were published.
