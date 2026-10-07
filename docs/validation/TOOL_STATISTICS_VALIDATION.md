# P03a tool statistics and DR — 2026-10-05

Status: implementation complete; final acceptance receipts are recorded below. Pinned development tools: FFmpeg **7.1.1**,
SoX **14.4.2**. Runtime is offline Rust with no subprocess. Existing native
measurements/byproducts, schema 0.18.0 and ancestry INCONCLUSIVE/null stay intact.

## Algorithms and input domains frozen before implementation

Primary source algorithms:
[FFmpeg astats](https://github.com/FFmpeg/FFmpeg/blob/n7.1.1/libavfilter/af_astats.c),
[drmeter](https://github.com/FFmpeg/FFmpeg/blob/n7.1.1/libavfilter/af_drmeter.c),
[SoX stat](https://github.com/chirlu/sox/blob/sox-14.4.2/src/stat.c) and
[sample conversion](https://github.com/chirlu/sox/blob/sox-14.4.2/src/sox.h).
Downloaded original files/versions/hashes remain ignored under
target/p03a-tool-stats/oracle, preserving copyright notices. Rust independently
implements the numerical definitions; upstream code is not bundled as runtime.

- Astats uses separate native lanes. Integer normalization follows tool signed
  maximum (32767 for <=16 bits, 2147483647 above); float retains its actual f32/f64
  input domain. Preserve global peak/RMS/DC aggregation, extreme sample/run
  counts (runs at EOF excluded), normalized 8192-bin absolute-amplitude entropy,
  last-nonzero sign changes (initial sign is negative), and minimum nonzero
  amplitude range. That range is not DR. RMS peak/trough use the pinned 50 ms
  exponential recurrence after the exact startup condition; <window inputs use
  whole RMS. Exactly one window leaves nonphysical upstream extrema sentinels:
  retain legacy text but mark numeric extrema inapplicable. Noise floor is the
  minimum output of the source's circular amplitude queue, including its cursor-
  equality expiration edge case and equal-value collapse. This can yield zero
  on a periodic odd-rate input although a conventional rolling maximum is
  positive. It is not the P03 RMS-p5 fallback or isolated recording noise.
- Astats overall fields are explicitly separate from per-channel crest/range/
  zero crossings. Legacy LoudnessProfile text follows Python's last printed
  occurrence, including omitted RMS trough=1, six-decimal FFmpeg rounding before
  Python two-place formatting, and raw linear crest mislabeled crest_factor_db.
  Correct product crest has separate linear and 20 log10 dB fields (D04).
- SoX stat uses channel-interleaved signed32 samples divided by 2147483647.
  Preserve exact integer words; f32 conversion truncates scaled samples, f64
  rounds away from zero; both saturate, counting conversion clips, while native
  decoding/hash stays unchanged. First delta is zero, minimum delta is the tool's
  initial zero, mean/RMS delta use samples-1; rough frequency truncates the tool
  estimate to an integer. All 15 canonical CamelCase keys are present with units,
  numeric/null reasons, source labels and pinned text precision. Zero power and
  too-short delta/frequency/volume results are inapplicable, never sentinel ints.
  Tool encoding guesses are diagnostics, not a sixteenth audio measurement.
- DR uses native-channel f32 input, exact sequential f32 squares/additions, three
  second blocks, RMS=sqrt(2*sum/count), nearest-even amplitude bins 0..32768.
  It selects the second peak across blocks and whole RMS histogram bins until
  the nearest-even 20% count is reached, using the pinned f32 arithmetic and
  its overshoot behavior. Tail is included only on finalization; the 20% count
  is computed from completed blocks before tail. Exact multiples have no tail
  and return upstream no-data; short zero-selection/silent/nonfinite ratios
  stay unavailable. Channel DR and finite overall mean remain distinct from
  astats amplitude range, crest and LRA. Product overall integer label is
  explicitly separate from the legacy first nonnegative matching channel label.
  Legacy %g precision/positive-only regex quirks stay audit-visible.

All algorithms observe the same selected stream/shared prefix. No implicit
mid/side/downmix or resampling. No duration-sized PCM or block list. Source,
precision, scope/hash, methods, exact counters and unavailable domains are typed.

## Worst-case scratch and cancellation

At 384 kHz, astats window is 19200 frames: each native lane has a fixed f64
sample ring and a fixed 19200 f64 circular amplitude array (307200 payload bytes), plus
8192 u64 entropy bins (65536). Two lanes: **745472 bytes**. DR has two 32769 u64
histograms per lane: **1048608 bytes** for stereo. Combined payload ceiling is
**1794080 bytes**, plus bounded scalar/container/report overhead. SoX has scalar
state only. This excludes existing native DSP/decoder and separately requested
P03 byproduct buffers. No array scales with duration or complete-block count.
Collectors are opt-in. Histogram reductions check cancellation/deadline in
bounded batches and between channels; source packet/blocking-I/O limits remain
cooperative. Failed/unsupported/cancelled/timed-out results suppress product data.

## Acceptance plan

Freeze generated silence/constants/sine/impulse/plateaus/noise/unequal stereo,
float conversion/overrange, one-sample/50 ms equal/short/tail/prefix, long varying
three-second levels, exact multiples and odd/high-rate controls before Rust
comparison. Actual tool format negotiation, stdout/stderr, versions/hashes and
PCM receipts are retained. Bounds: linear 1e-5 absolute, levels 0.05 dB, exact
integer counts, tool printed precision; DR 0.1 dB and integer labels exact away
from boundaries. Do not overwrite or widen an oracle after disagreement.
Detailed execution results will be appended when measured.

## Oracle frozen before Rust implementation

28 finite generated recipes and actual outputs were frozen in
`tests/fixtures/tool_statistics_reference.json` before `src/tool_statistics.rs`.
The fixture pins all four source hashes and tool version strings. Ignored audio,
selected-prefix audio and verbose tool logs are under
`target/p03a-tool-stats/frozen/`; the final differential does not call or rewrite
the oracle. Formats: signed16/24/32 and float32/64 WAV, rates 8/11.025/384 kHz,
short/equal/long 50 ms windows, omitted unit-power trough, extrema plateaus, zero
bridges, unequal sine, deterministic noise, saturation/conversion controls,
exact 2/6/9-second DR, zero-target tail, varying-level tail, equal-bin overshoot
and shared prefix. FFmpeg input/auto-format negotiation is in verbose logs.

The first checks found checker-only omissions for the deliberately suppressed
silent crest=1 and Windows SoX `1.#INF00` text. The odd-rate noise-floor mismatch
then exposed a real implementation error: a conventional deque did not model
the source's cursor-equality empty branch. The correction uses two fixed arrays
and source-compatible circular indices; frozen outputs and tolerances stay
unchanged. No runtime external tool or dependency was added.

## Product fields and audit mapping

`analyze_path_with_tool_statistics` / `analyze_source_with_tool_statistics`
return `ToolAnalysis { measurement, byproducts, tool_statistics }`. These are
opt-in synchronous source/path APIs with the existing callback and cancellation
contract. Validate each product separately. Statistics version/contract 1 uses
`ffmpeg-7.1.1-sox-14.4.2-v1`; dispatch by those IDs, coverage PCM hash, stream
precision/input domain and analyzed frame interval, not a generic field name.
`examples/read_tool_statistics.rs` is a no-CLI consumer; P07 owns CLI/report
exposure. Native `--json` and ordinary source APIs keep their existing shape.

Astats channel records expose exact frame/min/max/absolute-peak/zero-crossing/
noise-window u64 counts, 50 ms window frames and named scalar maps. Overall
peak count is an average across lanes and can be fractional; the source omits
overall crest/range/crossing fields. Scalar `value`, unit, display, applicability
and reason are distinct from `legacy_text`. `legacy_loudness_profile` reproduces
the pinned last-occurrence mixing and six-to-two-decimal formatting. Its unanchored
`Peak count:` regex selects the later `Abs Peak count:` value; both actual
extrema count and absolute-peak count remain separately named in the product.
This audit also retains raw linear `crest_factor_db` solely for audit. Correct `crest_linear` and
`crest_db` are separately named/unit-tagged. Trough=1 omission affects only
legacy selection, not the valid numeric zero-dB product result.

SoX has exactly 15 required canonical keys with original labels. Exact
`channel_sample_count` remains u64; scalar `samplesRead` is the legacy numeric
projection. `conversion_clips` is the signed32 conversion count, separate from
P03/native amplitude counters. Undefined one-sample delta and zero-power rough
frequency/volume remain explicit nulls; the Windows binary's nonfinite spelling
is an audit formatting detail, not a finite measurement. `Try` encoding guesses
and tool path/header warnings are retained in local source-tool logs and omitted
from the canonical measurement map (D12).

DR reports complete blocks before tail, tail frames and rounded top-20% target,
per-lane numeric values and overall mean. Whole-bin overshoot, second peak and
sequential f32 sums match the source. Finite negative DR is allowed as a
measurement; no compression/quality grade is assigned. The overall integer
label truncates the numeric mean; the legacy label follows the first nonnegative
`DR:` match after six-significant-digit tool formatting. The varying unequal-
stereo control has legacy `DR6` versus overall `DR0`; these are distinct labeled
outputs, not silently interchangeable. P05/P07 must choose the attributed field.

## Acceptance execution — 2026-10-05

The first optimized reader passed all 28 generated cases and exact independent
FFmpeg PCM hashes/selected frame counts. Three product schemas accepted results;
four mutations (wrong version, missing successful product, unavailable finite
value, missing canonical SoX key) were rejected. Max absolute errors against
printed tool values: level **4.99635e-7 dB**, linear **5.00001e-7**, DR
**4.98215e-6 dB**. Volume adjustment is printed to three decimals upstream;
maximum raw-versus-printed delta **0.000428573**, within its half-print-quantum
0.000501 comparison. All exact per-lane counts/PCM hashes and legacy DR labels
matched. Overall integer labels are also checked against independent tool text
away from integer/print boundaries, not merely against the Rust value. Both
overrange float controls counted the source-compatible **4000 conversion clips**.

Optimized Rust 1.85 no-CLI compatibility run passed **66 tests**: 40 library
units (including 4 new statistics units), 3 new API tests, 5 P03 byproduct tests,
6 PCM relationship, 6 loudness, 5 listening-level and 1 report-serialization test.
The final engine 0.25.0 statistics unit run passed **5 tests**, including a new
one-sample SoX legacy-text/null control. The three API controls also passed on
0.25.0 and were repeated after the pinned extractor exposed the absolute-peak
selection quirk; final receipts are listed in the packet result. Existing report/byproduct equality is
checked on the same prefix/source name; progress has one Finished; cancellation
during decode, timeout, malformed/missing and unsupported inputs suppress all
product records. Exact-multiple and long silent DR never invent values. Fixed
maximum-rate buffers retain capacities across many windows/blocks; entropy/DR
reductions propagate cancellation/deadline in bounded batches.

MSRV Clippy passed all targets with warnings denied, with and without default
CLI features. The initial unquoted PowerShell `--` lost the warning delimiter;
the quoted retry exposed and repaired an unnecessary cast and manual range
check. Formatting, all existing/new schema checks (47/16/9/3/13 definitions),
Python helper syntax, parity inventory and whitespace passed. The initial
missing shell `cargo` was repaired by the existing local wrapper, with no install
or dependency/lockfile update except the root package version. A saved-receipt
helper probe needed explicit scripts path and UTF-8; the repaired read-only
recheck passed without tool/audio reruns. Detailed local outputs remain ignored
under `target/p03a-tool-stats/`, distinct from Git history or external backup.

Final versioned reader/API results and preservation counts are recorded in
`task-results/P03a.md` and `HANDOFF.md`. No full core/CLI integration regression,
Android target/link/device check, private audio analysis, detector calibration,
new formats, commit/push/upload or external backup was performed. P07 owns
workflow integration; P04c owns reference spectral/transform/basis adapters.

A final pinned-extractor audit found an additional reference quirk: unanchored
`Peak count:` also matches the later `Abs Peak count:`. The previous checker
had independently selected the exact label and missed this. It now executes
the actual pinned `extract_loudness` AST on frozen ordered tool text, checking
all 12 astats audit fields plus DR without runtime subprocess/audio execution.
Corrected legacy selection and separately named absolute-peak product fields
leave actual extrema counts/PCM unchanged. Frozen source outputs/tolerances were
not edited. A shared-prefix regression verifies extrema count 12000, absolute
peak 6000 and legacy `peak_count` text `6000.00`. Seven generated combined-asplit
format probes (s16/24/32, f32/64, overrange, varying DR) also confirm the
single-filter oracle agrees with Python's combined astats/48k-ebur128/drmeter
graph for the P03a fields; receipts: `target/p03a-tool-stats/combined-graph/`.

Final optimized 0.25.0 reader re-ran the immutable 28 recipes after the audit fix
and passed exact PCM/counts, pinned AST legacy selections, numeric/null bounds
and all three schemas. Final producer constraints require complete astats lane/
overall, canonical SoX and audit maps; six mutation rejections pass on the saved
final reports. Lane count/order and window/block/tail/target relationships also
pass. Final maxima are unchanged. Preservation passed 253 prior files (242
unchanged, 11 intentional, zero unexpected), all 48 recordings/notes, owner and
Git index, clean pinned reference and unchanged dependencies except root version.
No jobs remain running; complete hashes/commands/limits are in task-results/P03a.md.
