# Version 0.11 below-cutoff sparsity — 2026-10-01

Schema `0.11.0`, policy `observations-only-v11`. Ancestry remains `INCONCLUSIVE`
and evidence index null. Very quiet spectral bins are a measurement, not evidence
that a codec discarded them. Sparse tones, filtering and spectral gaps can produce
the same observation without lossy encoding.

## Reference and scope

The pinned Python baseline is `c6ecce2296256b516709d87088896d1be913908c`.
Its `_spectral_sparsity` counts bins below -95 dB relative to each frame's peak,
using a final global cutoff. It includes DC, selects a floored cutoff slice,
adds epsilon terms inside logarithms and returns zero for missing coverage.
Rust retains the relative-magnitude count while making the actual geometry,
eligible-frame selection and unavailable measurements explicit. Python's codec
bin-zeroing interpretation and score contribution are not ported.

## Numerical contract

Use the existing per-native-channel 4096-sample symmetric-Hann STFT, hop 2048,
strict frame end before EOF and global -60 dB activity mask. No extra FFT or
decode pass is added. An active frame is eligible only when
`2 * max(magnitudes) / sum(Hann) > 1e-6`. The sum uses the actual rounded f32
window coefficients, with f64 reduction. This is a coherent spectral-amplitude
guard, not broadband RMS, a noise-floor estimate or source precision.

For each eligible frame and FFT bin, count the strict comparison
`magnitude < frame_peak * 10^(-95/20)` in f64 using the existing f32 magnitudes.
No logarithm or epsilon is needed. The engine stores only 2049 u64 per-bin
counts (16,392 bytes/channel) and constant-size bookkeeping, independent of
duration. It does not retain a spectrogram.

After streaming ends, use the channel's existing active-frame p95 cutoff.
That cutoff includes **all active frames**, including active frames rejected by
the new absolute-amplitude gate. It is not re-estimated from eligible frames or
selected independently for each frame. The percentile interpolates linearly
between ranked cutoff bin indices. Select actual bin centers strictly below
that frequency, also excluding DC and Nyquist. At a fractional cutoff, this may
include one more bin than Python's floored slice; exclusion of DC is also an
intentional difference. Bins above a narrow frame's own cutoff can contribute
if they are below the final global cutoff.

At least four eligible frames and ten selected bins are required. Otherwise
status is inconclusive and the sparse count, denominator and fraction are null.
A measured fraction is `sum(selected per-bin counts) / (eligible_frames * bins)`.
Measured zero is valid for sufficiently flat spectra. Insufficient input is
never a fabricated zero or a successful-clear codec result.

The `sparsity` report exposes total, active, eligible and below-floor frame
counts; actual selected bin range/count; global cutoff; numerator, denominator
and fraction. Its interval encloses all STFT windows. Counts describe selection
within that interval; it does not mean every enclosed sample was eligible.
Prefix limits and existing packet cancellation/deadline and two-pass PCM
integrity checks apply. A `below_cutoff_sparsity` observation belongs to the
spectral-measurements family and remains measured/inconclusive, without a codec
hit, probability or source label.

## Executed validation

Five local Rust integration tests cover flat single-impulse spectra over
8–384 kHz, sparse natural tones, native antiphase/half-gain, silence, quiet
signals, inadequate bandwidth, zero/three/four-frame prefix boundaries,
inactive gaps, a silent channel and changing bandwidth with the final cutoff.

`scripts/check_sparsity.py` independently computes NumPy FFTs after f32 window
multiplication and retains a dense frame/bin matrix for its oracle. It selects
the global cutoff independently and calculates counts directly from that
matrix, rather than reproducing Rust's accumulated-bin implementation. Expected
values are saved before running Rust and cannot be silently overwritten on a
rerun. Exact FFmpeg s32le hashes check integer decoding for every generated file.

Declared comparisons: counts, geometry, eligibility, intervals and statuses must
match exactly, with 1e-9 Hz tolerance for frequency values. For the sparse count,
f32 FFT rounding can cross a strict threshold: the predeclared allowance is the
number of independently computed ratios within `2e-7` of `10^(-95/20)`. Count
differences must be no larger than that number; if no ratio is ambiguous the
count must agree exactly. Fraction tolerance is that count allowance divided by
the exact denominator, plus 1e-14 for serialization/reduction. This is a numerical
comparison bound, not a claim that threshold decisions generalize to real audio.

All **19 generated stereo 24-bit files / 38 native channels** passed numerical
comparisons and exact PCM hashes (NumPy 2.3.5, FFmpeg 7.1.1). The notch control
differed by one sparse observation out of 141,243: 21,062 in Rust versus 21,063
in NumPy, a fraction difference of 7.080e-6. Its oracle had 28 ratios inside the
predeclared ambiguity band, so this passed without changing tolerances or saved
expected values. Other sparse counts agreed exactly. This does not establish
bit-for-bit FFT equivalence or robustness of individual threshold decisions.

| Generated control | Result |
| --- | --- |
| One impulse per window, 8/32/44.1/48/96/384 kHz | Zero sparse fraction; 2047 non-DC/non-Nyquist bins |
| White noise, antiphase half-gain | Zero sparse fraction for this finite control |
| 6–10 kHz notch versus 8 kHz low-pass | Fractions about 0.14912 / 0; selected region matters |
| 6 kHz tone, antiphase half-gain | Fraction 0.9921875 for both channels |
| Silence, tiny tone, DC and 10 Hz tone | Inconclusive/null through frame or bin gates |
| Inactive gap with a silent native channel | Active channel measured, silent channel inconclusive |
| Low-pass segment followed by broadband noise | About 0.20948 using a single global cutoff |
| Exact 4096-sample input; three/four STFT frames | No-data/insufficient/measured respectively |
| Tones selecting nine/ten bins | Inconclusive/measured at the exact bin-count boundary |
| Quiet broadband after a stronger tone | 69 active, 24 eligible, 45 below-floor frames; global cutoff 24 kHz |

All **78 release tests** passed: 21 unit, 4 AAC, 13 core/CLI, 5 segment/MQA,
4 envelope, 7 noise, 2 parity/integrity, 4 roll-off, 5 sparsity, 3 structure,
5 transform and 5 transient tests. Formatting, all-target Clippy with warnings
denied, Android ARM64 library compilation without CLI and production Clippy in
the publication-only copy `target/source-only-v11` passed. Release CLI version
`0.11.0` and a generated 0.8-second-prefix text smoke check passed. The final
source differs from the tested build only by expanded explanatory caveat text;
the release CLI and publication-only checks include that text.

The unchanged generated 10/120/600-second duration inputs matched exact PCM and
used 20.07/23.70/25.57 MiB peak Windows working set in 0.70/2.48/9.20 seconds.
These host observations support bounded-duration storage; they are not a hard
process memory limit or an Android benchmark. Full debug tests, remote CI
completion, NDK linking, phone behavior and source classification accuracy were
not checked. No new encoder/resampler matrix or private collection pass was run.
Historical processing experiments remain described in earlier validation files.

Local evidence: `tests/sparsity.rs`, `scripts/check_sparsity.py`,
`corpus/local/generated/sparsity-v11/`, `corpus/local/results/sparsity-v11/` and
`corpus/local/results/resources-v11/`. These remain excluded from source-only
Git history and are not backed up by a source push. No historical expected
output or private recording was modified.
