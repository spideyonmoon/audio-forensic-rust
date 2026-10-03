# Version 0.16 native PCM relationships — 2026-10-02

Schema `0.16.0`, policy `observations-only-v16`. Sample crest factor and signed
native stereo correlation are descriptive; ancestry remains `INCONCLUSIVE` and
evidence index null. No compression, fake-stereo, DR or mastering-quality label.

## Reference and contract

The pinned Python extracts FFmpeg astats' crest factor and formats it as dB,
without converting its linear peak/RMS ratio. Rust exposes both a linear ratio
and `20*log10(ratio)` explicitly. Each native channel uses all PCM inside the
shared prefix, including DC and zero samples, without filtering or a downmix.
Silence has a null ratio and inconclusive status; a nonzero constant legitimately
has linear crest 1 / 0 dB. This is sample crest, not true-peak crest or a DR score.

RMS accumulation now scales by the running sample peak. On a larger peak, rescale
the normalized sum of squares; otherwise add `(sample/peak)^2`. Compute RMS as
`peak * sqrt(scaled_sum/count)` and crest directly as `sqrt(count/scaled_sum)`.
This avoids underflow from squaring very small float PCM and remains bounded.
It changes the RMS arithmetic, not decoding or integer precision. Extremely
small contributions relative to the peak still face ordinary f64 precision;
RMS below the smallest representable f64 can round to zero while the independently
computed dimensionless crest is representable.

For stereo, use all simultaneous PCM pairs in the first decode pass. Maintain
running means, centered sums of squares and covariance using Welford updates,
with each lane normalized by its own running peak. Rescale means/variances and
the cross-product when either scale increases. No full audio, extra pass,
FFT or duration-dependent buffer is added. Storage is ten f64/u64 scalars plus
the fixed report (approximately 80 bytes). Signed Pearson correlation uses the
centered covariance and population standard deviations; its sign is preserved.
Numerical rounding is clamped to [-1,1]. No lag search, alignment, activity mask,
stereo downmix or spectral selection is applied. Silent gaps retain their indices.

At least two pairs and both lane standard deviations strictly above 1e-8
full-scale amplitude and 1e-6 of that lane's sample peak are required. These
are numerical applicability gates, not empirical stereo-quality thresholds.
Mono is unsupported for correlation, with no fabricated channel pair. Constant,
weak or DC-dominated insufficient variation is inconclusive, with null coefficient.
Pair counts, native indices, means, standard deviations, eligibility and interval
are explicit. Correlation is invariant to added DC and nonzero gain only while
the input remains eligible; phase, delays, editing and content can change it.

## Validation protocol

Unit tests compare changing-scale centered statistics against direct means and
variances and verify tiny nonzero variation. Integration controls cover sine/
square/DC/impulse crest, gain/antiphase/DC/phase stereo, mono/constant/quiet/DC
gates, float squaring underflow, minimum pairs, prefixes, silent gaps, late larger
peaks and JSON. Tests use generated audio only.

Independent generated controls use direct dense NumPy PCM arithmetic with
scaled squares, explicit centered arrays and dot products. Immutable expected
JSON must be saved before Rust runs. Exact FFmpeg s32le hashes for integer inputs
and f64le hashes for float inputs must agree. Predeclared tolerances: RMS/peak/DC/
mean/std `1e-14 + 1e-11*abs(expected)`, crest ratio relative 1e-11, crest dB 1e-9,
correlation 1e-10. Counts, intervals, applicability and nullability match exactly.
Tiny values are compared by relative error to avoid an absolute tolerance masking
underflow. No music is used as source-history ground truth.

## Execution record

All 28 unit tests and six new integration tests passed with the local LLVM
linker. All-target Clippy and Python syntax passed. The full release regression
passed under v0.18 after preceding-energy and acceptance additions: **130 tests,
zero failed/ignored**. v0.15's 106-test run was the preceding complete regression.
Existing staged work, private recordings and
the pinned Python clone remain untouched. No commit, push or external backup.
The first focused command stopped at compilation: a test compared an optional
interval with `assert_eq!` although the interval type has no `PartialEq`.
The assertion now checks `is_none()`; production arithmetic was unchanged.
The next run passed all 28 unit tests and five of six integration tests. The
remaining two-pair assertion required exact -1 although centered arithmetic
returned -0.9999999999999999. It now uses 1e-14 tolerance with exact pair counts;
production code was unchanged. The focused suite then passed.

All **26 generated cases / 51 native channels** passed immutable dense NumPy
expectations and exact FFmpeg s32le/f64le hashes. Max errors: 2.137180e-14
amplitude, zero tiny-amplitude relative error, 4.884982e-15 relative crest,
4.218848e-14 dB crest and 2.686740e-14 correlation. Counts, intervals,
nullability and applicability agreed exactly. Generated FFmpeg astats independently
reported crest **1.414214** for the sine control, confirming that the reference's
raw extraction is a linear ratio. Rust reports approximately 3.0103 dB after
conversion. Evidence: `corpus/local/generated/pcm-relationships-v16/`,
`corpus/local/results/pcm-relationships-v16/`, `scripts/check_pcm_relationships.py`
and `tests/pcm_relationships.rs` remain local and excluded from publication.

The separate v0.18 independent regression passed all 26 cases / 51 native
channels with unchanged maximum errors and exact hashes. Its expectations were
copied from the preserved v0.16 directory before running, and existing audio was
verified unchanged. Results: `corpus/local/results/pcm-relationships-regression-v18/`.
