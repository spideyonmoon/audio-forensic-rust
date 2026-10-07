# Current extension — 2026-10-05

P04a/P04b are complete in engine 0.27.0, extending the separate API to inputs
version 2. See `REFERENCE_PROFILES_VALIDATION.md` and `schemas/reference-inputs-2.schema.json`.
P04c numerical methods/native measurements remain unchanged. The historical v1
schema and frozen controls below are preserved; adaptive-wall dependencies now
consume explicit reference resampling/filtered void inputs.

# P04c reference inputs — 2026-10-05

Pinned source: `c6ecce2296256b516709d87088896d1be913908c`, Python file
SHA256 `b60959955ed5ed8317cd9c43f7619ca1917c81a41e0d93fa9b2b4185367c1e30`.
New public generated oracle `tests/fixtures/reference_inputs.json` was frozen
before Rust implementation with NumPy 2.3.5 / SciPy 1.17.1. The helper refuses
to overwrite it. Existing PCM and per-method oracles remain immutable.

## Methods and dependencies

Separate method `python-c6ecce2-p04c-f32-v1`, inputs version 1. The native
report/schema/policy are unchanged. This packet owns G08/G11/G13/G14/G15;
P04a/P04b own subsequent inputs and P05 owns all scoring and interpretation.

Native samples round individually to f32; stereo mid/side arithmetic is f32.
Native signal and exactly cancelled mid are explicit. Base STFT is symmetric
f32 Hann 4096 / hop 2048, with strict window end < analyzed frames, no padding.
Activity is > (global magnitude peak + 1e-12) * 1e-3. At least four reference
frames and four active rows are required for numerical base inputs (D03).
Cutoff last-bin > -65 dB, p95 linear interpolation and population variance use
fixed histograms. Mean spectrum accumulates in f32 row order; sharpness/cliff
retain f32 arithmetic. Magnitude HF and f32-squared noise reductions use bounded
f64 totals rather than duration-sized NumPy reductions; differences must pass
the frozen RELEASE_CONTRACT tolerances. Entropy uses f32 normalized magnitudes
with f64 reduction. STFT noise is raw FFT magnitude dB, not a time-domain void.

auCDtect source scatter selects active indices 0, stride, 2*stride, ... where
stride = A//2500 + 1 if A>2500 else 1, known after the second pass. f32 running
moments / five-bin nearest-edge smoothing and observed-range twenty-bin
histogram retain source semantics (lowest histogram tie). Equal observations
expand the range ±0.5 Hz. Flat rows are excluded from product values using the
existing centered variance gate (D03); source flat-floor values are audit-only.
Compacted source phase uses f32 angles widened to f64, floor(10k/bin), Nyquist
included, and all active rows (not scatter sampling), including gaps. Its
36-bin histogram/entropy are legacy audit only. Physical product phase uses
adjacent actual active rows, >=10 kHz, excludes Nyquist, absolute/relative
energy floors, at least two shared bins/pair and three pairs. Gaps stay gaps.

Segments request floor(duration/15) clamped 9–36, and require 2 seconds per
requested probe, Nyquist > 16.5 kHz. CPython MT19937 seed-array initialization
and getrandbits rejection reproduce Random(42).randint; duplicate offsets stay.
A two-second f32 ring and reusable f64 clip produce exact planned intervals,
including overlap and duplicate/end probes. Existing ClipAnalyzer transforms
the widened shared basis using f64 Hann/FFT. Inactive/empty numerical bands
remain explicit; product vote needs three broad-band eligible clips. The
source silent-only count and even-half/odd-majority rule are separate audit.
Nearest wall searches the ordered source table, preserving first equal-distance
tie; it is an empirical candidate, not proof of codec history. Adaptive wall
accepts reference cutoff/cliff, P04a resampling-wall and P04b verified time-domain
void inputs explicitly. Pending dependencies stay null; false is never assumed.
Classic wall remains available, and per-probe data supplies P05 splice rules.

AAC uses f32 M/S widened to f64 with the existing bounded energy/anchor/KBD/MDCT
implementation and stronger four-probe/sixteen-band applicability (D03).
Energy sums are local hop reductions rather than source cumulative subtraction;
equal-energy anchors retain the documented earliest native deterministic tie.
Winner is highest available score, first M then S on ties; null never wins.
The pinned `_vorbis_grid` actually widens f32 M/S **before reconstructing L/R**,
then searches L followed by R (mono M), with first equal-score winner. The old
card/map's “mid/side winner” wording was inaccurate. Support/tested counts are
the winning basis's actual counts; absent/unsupported inputs have null winners.
No E01/E02 accuracy changes or threshold tuning are included.

## Source/control and resource contract

Opt-in source/path API holds one worker and opens one guarded source. It uses
three independent decoder passes on the same requested prefix, verifies exact
integer/f64 PCM hash, frame count and PCM kind on each pass, checks full-EOF
decoder checksums and FLAC frame/byte accounting, and re-probes FLAC from zero.
Native report coverage remains two passes; the separate record carries all three
hashes and its processing-pass count. Progress exposes pass 3 and one Finished.
Failed/cancelled/timed-out records suppress all reference inputs. Ordinary APIs
allocate no reference collector and preserve their two-pass contract.

No runtime Python, subprocess, new dependency, full spectrogram or duration-sized
PCM. Shared-base histograms/accumulators/STFT are fixed. Transform captures cap
180 seconds, sixteen AAC anchors/basis and twelve Vorbis anchors/channel.
Segments retain one 2-second ring and clip, regardless of file duration or probe
count. FFT planning/scratch is bounded by supported rates (8–384 kHz). At rate r with n=2r, segment arrays require 4n ring + 8n widened clip +
8n Hann + 16n FFT buffer + 16*(n/2+1) magnitude/dB + 16*S(r) scratch bytes,
where S(r) is the locked planner's in-place scratch length. At 384 kHz this is
**46,080,016 payload bytes**, including 12,288,000 FFT scratch bytes. FFT plans/tables, allocator overhead,
scalar reports and existing native buffers are separate; no total RSS/device cap
is claimed. Scratch varies for unusual/prime rate lengths. The two-second PCM
buffers are always exactly 9,216,000 payload bytes at 384 kHz; no 36-clip allocation.
AAC M/S captures max 784,384 bytes, Vorbis reconstructed L/R max 589,632 bytes
at supported 44.1/48 kHz; those reference captures are reduced/dropped before
reference segment-ring allocation. Each new method checks control at scan/phase
batches; a single two-second FFT is one bounded cooperative work unit.

## Checks

- Optimized Rust 1.85/no-CLI compatibility run passed **79 tests**: lib 46,
  AAC 4, detectors 5, parity 2, reference inputs 4, source contract 10,
  structure 3 and transforms 5. This run used the final DSP code at engine
  0.25.0 before the version bump and u64-before-usize ring-index correction.
  Existing immutable PCM/DSP oracles were not changed. Recorded receipt:
  ignored `target/p04c-reference-inputs/compatibility-summary.json`.
- Final engine 0.26.0 no-CLI optimized `read_reference_inputs` build passed.
  Three complete generated streaming controls passed all exact PCM hashes,
  prefix/EOF, STFT activity, plan/clip intervals, frozen base/scatter/phase/clip
  values and native schema checks. Unequal stereo, an 18-second wall/noise/
  silence splice and a 55-second active-gap control cover distinct bases.
  The long control has **2,532 active frames / stride 2 / 1,266 sampled rows**.
  Legacy compacted phase has 2,531 pairs; physical phase has 2,530, preserving
  the gap. The splice has 263 active rows and six eligible clips.
- Max errors over those streaming controls: cutoff/variance/scatter mean/mode/
  segment cutoff/segment peak **0**; sharpness **1.43052e-6 dB**, HF ratio
  **7.77835e-8**, entropy **6.06641e-7 bits**, noise **2.55162e-7 dB**, source
  phase entropy **1.99713e-5 bits**, clip cliff **2.41298e-9 dB**, clip high-band
  **3.05534e-10 dB**. All are inside the unchanged RELEASE_CONTRACT bounds.
  Each control rejects six schema mutations (18 rejection checks): native
  method alias, unknown version, wrong pass count, missing null reason, failed
  result with successful inputs and missing pass hash. Saved JSON/summary/errors
  are in ignored `target/p04c-reference-inputs/streaming/`.
- Five adapter units cover f32 rounding/widened reconstruction, Python RNG for
  adaptive 9–36 plans including >32-bit ranges, 5,003-row stride 3 / twenty-bin
  scatter, flat-spectrum suppression, gap/bin applicability, ring overlap /
  duplicate offsets / fixed storage and adaptive dependencies/vote boundaries.
  Eight frozen AAC/Vorbis layouts cover mono, dual mono, antiphase, right-only,
  M/S winner/equal scores and reconstructed L/R winner/support/tested counts.
  API tests cover unchanged native prefix reports, pass-3 cancellation, source
  mutation, structured deadlines/errors and success-payload suppression.
- All-target Rust 1.85 Clippy passed with and without CLI features, warnings
  denied. Formatting/whitespace, frozen oracle reproduction, helper syntax,
  57-definition new schema and unchanged measurement/byproduct/statistics/
  metadata/audit schemas (47/16/13/9/3) passed. Parity inventory passed.
- Final 0.26.0 optimized no-CLI run passed **50 tests** (46 lib + 4 API);
  storage receipt confirms **46,080,016 segment-array payload bytes at 384 kHz**.
  Log: ignored `target/p04c-reference-inputs/final-tests.log`. Two early
  test issues were repaired: missing leading zeros in Rust test float literals,
  and a Vorbis-null expectation on a 0.4-second prefix that actually met its
  minimum length (changed to 0.3 s). No source oracle/tolerance was edited to
  repair Rust. An initial documentation script used the wrong text encoding;
  a nonexistent audit-helper filename and a row-split assertion were corrected
  and the actual inventory checker passed.

No full core/CLI integration regression, Android compile/link/device work,
private recording analysis, calibration, new format work, endgame encoder
research, commit/push/upload or external backup. Local ignored receipts are
not Git history or a source backup. Existing unrelated dirty/untracked work,
owner checklist, private recordings and pinned reference must stay intact.

## Follow-up review — 2026-10-05

Owner-requested review of all completed P04 work found no production-code defect
requiring correction. P04c is complete; P04a/P04b remain separate pending
packets. See `task-results/P04-review.md` for inspected scope and actual checks.

The existing third-pass mutation test corrupted a WAV header, so it did not
exercise the PCM hash mismatch after a successful decode. The expanded test
also changes one finite f32 mantissa bit at frame 40000 only on pass 3, preserving
the header, decoded count and PCM kind. It requires the exact diagnostic
`Source changed on reference input pass` and null reference inputs. The original
malformed-input and deadline cases remain. Optimized Rust 1.85/no-CLI lib/API
run passed **50 tests (46 + 4)**, including this additional case; no oracle,
tolerance, production method or engine version changed. The separate schema
reproduction and parity inventory checks passed. The full regression, streaming
corpus and Android/device checks were not rerun for this test-only correction.
Focused Rust 1.85 no-CLI lib/API Clippy with warnings denied and formatting
verification also passed. Initial local toolchain/linker environment failures
were resolved without production changes; details are in the review record.
