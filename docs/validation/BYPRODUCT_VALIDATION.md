# P03 reference byproducts and native level display — 2026-10-05

P03 complete. Engine 0.24.0; separate byproduct version 1.
Measurement schema 0.18.0, observations-only policy, INCONCLUSIVE ancestry/null
evidence index remain unchanged. No ancestry score or calibrated verdict.

## Frozen definitions and differences

The pinned c6ecce2296256b516709d87088896d1be913908c pure byproduct functions
are the oracle. Samples convert to f32 before separate add/subtract and division
by two; reconstructed L/R add/subtract in f32 before f64 block statistics.
Mono uses channel 0 with absent side; its phase is inapplicable, never fabricated
as 1. All byproducts cover the shared analyzed interval, without the 180 s
detector cap. Silent mid with native signal has an explicit cancellation marker.

- Phase: complete floor(sr/10)-sample blocks, discard tail, centered signed
  Pearson; denominator strictly >1e-12. Equal mean of valid blocks, no native
  whole-stream numerical gates substituted. Constant/silent pairs unavailable.
  Welford f64 accumulation replaces dense centering with 1e-10 absolute numeric
  tolerance. Pinned three-place text and threshold verdicts are audit fields;
  ordinary summary describes correlation, with processing history unknown.
- Ceiling: count reconstructed channel samples with |x| >= 1-1/32768 at every
  source precision, including finite out-of-range floats. Exact 0/<10/>=10 audit
  wording is retained. A below-ceiling plateau is not counted. Neither a plateau
  detector nor audibility/recording diagnosis. Native precision full-scale counts
  remain unchanged and separate.
- Silence: strict f32 comparison against the reference's converted -60 dBFS
  amplitude; continuous runs of >=floor(sr/2) samples, including EOF/prefix runs.
  Exact sample intervals and all-run totals; percentage denominator is analyzed
  frames (D02). Audit text retains Python's near-end (two frames) EOF marker;
  ordinary text identifies analyzed end when the stream EOF was not reached.
- Noise fallback: at least one analyzed second, complete floor(sr/10) blocks,
  RMS after widening mid to f64; remove exactly zero blocks, require >=5 positives,
  NumPy linear p5 then 20 log10 amplitude. Discard trailing block. This is a
  fallback candidate, separate from astats; P03a/P07 decide use when astats is
  unavailable. Numeric bounds: RMS 1e-12 relative, dB 1e-9; two-place text exact
  on frozen fixtures. It does not measure isolated recording noise.
- Native level mapping: per-channel sample peak/RMS dBFS, uncentered DC amplitude,
  separate linear/dB sample crest, existing FIR estimated dBTP; native programme
  LUFS/LRA/momentary/short-term with original window scope. Fixed -16/-14 deltas
  subtract unrounded native integrated LUFS, signed one-place dB. They are method
  targets, not current service policies. D04 retains native meter differences;
  astats/SoX overall aggregates and DR are P03a. No peak-RMS/LRA alias for DR.

## Resource and host contract

The new path/source APIs return a ByproductAnalysis containing the original
measurement and a separate ByproductReport, collected inside the same two-pass
worker/source/progress/deadline/hash contract. Ordinary APIs incur no collector
allocation. No runtime subprocess, extra decode/FFT, retained PCM or dependency.
Failed/unsupported/cancelled/timed-out calls suppress all byproduct observations
and levels; partial measurements are never mapped to successful product fields.

Phase, ceiling and quiet-run state is fixed size. RMS retains at most 864000 f64
positive scalars (6,912,000 payload bytes), in explicit increments capped at the
limit, independent of sample rate. This covers 24 hours at an exact 100 ms grid;
floor(sr/10) at odd rates can reach the cap slightly earlier. Overflow retains
exact all-block counts and marks only fallback ResourceLimit/null, never a prefix
percentile. Exact order selection avoids a full sort and checks controls around
each selection; one selection is cooperatively indivisible. Silence retains
1024 intervals (16,384 payload bytes), exact all-run totals/omission count and
bounded generated text, even beyond the cap. Final text reduction checks controls
per section. Packet-level work and blocking I/O retain existing cooperative limits.

## Actual checks

- Rust 1.85.0 optimized/no-CLI collector checks: **4 tests passed**. One exercises
  all **31 immutable pinned vectors**; other checks cover exact branch/rounding
  boundaries, cancellation/zero frames and section/RMS cap behavior. RMS overflow
  is exercised with a reduced private test cap; no 24-hour audio job was run.
- Rust 1.85.0 optimized/no-CLI decoded API/native checks: **23 tests passed**:
  5 byproducts, 6 PCM relationships, 6 loudness, 5 listening levels, 1 report
  contract. All **27 focused tests** passed with zero failures/ignored. The API
  comparison verifies exact equality of every native-report field with collector
  enabled/disabled on the same antiphase source; decoded f64 hash matches direct
  bytes. Prefix, finite/null units, signed deltas/ties, mono/short/odd-rate meters,
  failure/cancel/deadline/missing path and Finished-event behavior are checked.
- `scripts/check_byproducts.py` passed **31 decoded generated WAV cases** with
  exact independent FFmpeg s32le/f64le SHA-256 and frame counts. All pinned
  phase/ceiling/fallback/quiet audit strings/counts/intervals match. Every file
  passes unchanged measurement and new byproduct schemas; four structural
  mutations are rejected. The 24-bit ceiling control has 16000 reference samples
  above the fixed threshold while both native full-scale counts stay zero.
  Inputs, frozen executable and expected fixture were verified unchanged.
- Read-only `--check-saved` additionally verifies raw silence percentages,
  native sample peak/RMS/DC/crest/true-peak mapping, loudness/range intervals,
  fixed targets, raw RMS p5 and dB conversion. Maximum errors against the dense
  pinned-domain oracle: phase **1.110224e-15** absolute, RMS p5
  **1.065814e-14** relative, floor **9.947598e-14 dB**. Exact text/counts/nulls
  and all declared numeric tolerances pass. No tolerance was widened or reference
  regenerated after comparison.
- MSRV all-target Clippy with warnings denied, optimized no-CLI example build,
  stable formatting and Python helper syntax passed. Byproduct schema drift
  passed (**16 definitions**); measurement (**47**) and metadata/audit (**9/3**)
  artifacts remain unchanged. Pinned-vector `--check` and parity inventory checks
  pass; final task/link/whitespace checks and preservation are recorded in P03.md.

Tools: NumPy **2.3.5**, FFmpeg **7.1.1**. The existing local schema-validation
environment supplies jsonschema via PYTHONPATH to system Python; no package was
installed. The generator executes only four AST-extracted pure functions from
the clean pinned reference, never its CLI or user recordings.

Case expectations: silence/DC give no valid phase blocks; mono phase is
inapplicable; dual mono/antiphase give +1/-1, with antiphase mid 100% quiet and
explicit native-signal cancellation. Differently scaled +1/-1 blocks average to
zero independently of energy, exposing the difference from native correlation.
0/9/10/1000 ceiling cases exercise exact audit branches; the 0.9 plateau is zero.
3999/4000 quiet frames at 8 kHz give zero/one qualifying half-second run.
Threshold equality is excluded; one f32 step below is quiet. A four-stage fade
staircase qualifies its final 4000-frame run. The one-second prefix containing
4000 quiet frames reports 50%, not the full two-second duration's 25%; a wholly
quiet half-second prefix marks analyzed end. Four positive blocks stay unavailable;
the ten-block RMS vector discards zero and interpolates positives exactly.
Odd-rate 11025 Hz uses 1102-frame blocks and a 5512-frame half-second floor.
All short/tail/DC/near-variance-gate/reconstruction rounding values are retained
in the compact public generated-only fixture.

The initial format attempt found Rust shorthand float literals and applied no
formatting; these were corrected. Initial test compilation was denied at the
installed linker inside the sandbox; the authorized retry passed the four unit
tests, while its global test-name filter ran zero integration tests. The subsequent
unfiltered focused command passed all 23 integrations. An initial system-Python
probe lacked jsonschema; the existing environment was reused, with no installation.
Some documentation patches applied nothing due to CRLF context; exact normalized
text edits repaired them. These were tooling attempts, not oracle disagreements.
Logs/exit receipts, generated files, copied expectations/reports and binary hashes
are ignored local files in **target/p03-byproducts/**, not Git history/backup.

No full regression, private recording analysis, codec/accuracy campaign,
Android compile/link/device run, commit/push/upload or external backup.
No jobs remain running. P03a is next; astats/SoX/DR and later product workflows
retain their own contracts. Native measurements and uncalibrated audit wording
do not establish source authenticity or audibility.
