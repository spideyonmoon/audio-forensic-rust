# Version 0.8 high-pass envelope peaks — 2026-10-01

Schema `0.8.0`, policy `observations-only-v8`. Native-channel descriptive
measurements only; ancestry stays `INCONCLUSIVE`, evidence index null. Peak
counts are not physical click counts and do not confirm vinyl or cassette.

## Reference assessment and intentional differences

The pinned Python remains clean at `c6ecce2296256b516709d87088896d1be913908c`.
Its `_silence_and_vinyl` click stage runs after a source-profile gate, high-passes
the capped audio at 1 kHz, smooths the rectified result over about 0.5 ms, and
calls SciPy `find_peaks` with three times the exact median and 10 ms spacing.
It then treats certain rates as confirmation of vinyl. That interpretation,
source gate and score adjustment are not imported.

Rust retains the fourth-order Butterworth filter and rectified-envelope scale.
It adds explicit startup/edge exclusions, minimum coverage, an absolute threshold
floor and bounded median estimation. It selects peaks chronologically rather
than globally preferring taller peaks. Rates use eligible peak-center coverage,
not the entire input duration. Arithmetic is f64 and channels stay separate.
Consequently, neither complete peak-count parity nor source accuracy is claimed.

## Numerical and coverage contract

The existing first decode pass feeds each native channel to two causal, zero-state
Butterworth high-pass sections at 1 kHz. Their Q values are
`1/(2*cos(pi/8))` and `1/(2*cos(3*pi/8))`; transposed direct form II uses f64.
Filter state below absolute `1e-30` is flushed to zero to avoid subnormal work
after decay. This state floor is far below the measurement threshold.

The envelope is `pi/2` times the mean absolute filtered amplitude in an odd
window of `(sample_rate / 2000) | 1` frames, with integer division. A ring buffer
produces centered values only when the full window is available. The sum is
rebased once per ring rotation to bound accumulated roundoff. No edge padding is
used. Centers before `ceil(sample_rate / 50)` (20 ms) are excluded. The finite
startup exclusion does not establish that arbitrary filter effects are absent.

Input is limited to the first `min(analyzed_frames, 180 * sample_rate)` samples
in each channel. Report `interval` describes that input prefix. With N input
frames, half-window h and warmup w, the baseline uses envelope centers
`[w, N-h)` when available. Local maxima need both neighbors, so
`eligible_peak_interval` is `[w+1, N-h-1)` or null when empty. A measured result
requires at least `ceil(sample_rate / 2)` eligible centers. Insufficient coverage
returns inconclusive with null count/rate/threshold, not a successful zero.
Silence with sufficient coverage legitimately yields measured zero peaks.

The first pass retains an 883-counter histogram: exact zero, positive underflow
through `1e-9`, 880 amplitude bins from -180 to +40 dB in 0.25 dB steps, and
overflow from 100. Bounds enclose the two middle order statistics and are averaged
for an even sample count. Positive underflow has bounds `[0, 1e-9]`; overflow
has no finite upper bound and makes the peak result inconclusive. Reports expose
both median bounds, not a fabricated exact median. For ordinary bins the upper
bound is at most approximately 2.92% above the exact median.

The second existing decode pass repeats the filter/envelope with fresh state.
A local maximum must be **strictly above**
`max(1e-4, 3 * baseline_median_upper)`. The floor is -80 dBFS envelope amplitude;
it is an engineering guard, not a calibrated audibility threshold. Exact flat
maxima use their midpoint rounded down. The first qualifying maximum is accepted;
later maxima require at least `max(1, sample_rate / 100)` frames since the last
accepted peak. This chronological choice makes streaming storage constant.
SciPy's distance selection can retain a later, taller peak instead.

All accepted peaks are counted; only the first 128 `(frame, envelope_peak)` pairs
are listed, with `events_truncated` when necessary. Rate is
`count * 60 * sample_rate / eligible_center_count`. Frame indices are envelope
peak centers, influenced by the causal filter and smoothing; they are not impulse
onsets. Peak amplitudes use full-scale-normalized linear envelope units, without
clamping at one. All interval ends are exclusive.

The first-pass histogram uses 7,064 bytes/channel and the envelope ring at most
1,544 bytes/channel at 384 kHz. The histogram is dropped before second-pass
counting; event payload is capped at 2,048 bytes/channel. Filter state, temporary
conversion overlap and report bookkeeping add small fixed amounts. There is no
audio-prefix allocation, extra FFT or extra decode pass. Cancellation/deadlines
remain checked by the existing packet scan, and cross-pass PCM hashes still
reject changing input. This is not a process-wide allocation cap.

## Actual validation

Five new Rust integration tests passed in release, including f64 WAV input:

- Exact minimum-coverage boundaries at 8,000/8,001/44,100/48,000/96,000/384,000 Hz,
  short input, measured silence and JSON roundtrip.
- Isolated impulses, native antiphase stereo and a silent channel.
- 5 ms versus 12.5 ms spacing, startup/tail exclusion, and 200 accepted events
  with only 128 listed.
- Smooth musical attacks and a continuous clipped tone, with no source label.
- A requested 0.8-second prefix and an actual 181-second input that excludes an
  event beyond the 180-second transient cap while preserving whole-file coverage.

`scripts/check_transients.py` independently designs SciPy SOS coefficients,
filters decoded samples, computes the envelope by direct NumPy convolution,
obtains exact median order statistics and uses SciPy local maxima followed by
chronological spacing. Expected results are written **before** Rust runs. It also
records height-priority selection on the same eligible envelope for comparison;
this is not a full reproduction of the gated Python source classifier.

All **14 generated stereo 24-bit WAV controls / 28 native channels** passed exact
FFmpeg s32le PCM hashes and the numerical checks. Counts, retained positions,
intervals, applicability and truncation flags matched exactly. Floating values
use a predeclared tolerance of `1e-10 + 1e-8 * abs(expected)`. The largest retained
peak-amplitude difference was `1.562e-15`. Median bins match except that sub-1e-9
decaying tails permit either exact-zero or positive-underflow bounds due to the
state flush; exact-median containment is checked with the absolute tolerance.
Versions: NumPy 2.3.5, SciPy 1.17.1, FFmpeg 7.1.1.

| Generated control | Observed counts (left / right) |
| --- | --- |
| Four antiphase impulses at 8/44.1/48/96/384 kHz, five files | 4 / 4 in every file |
| Silence / very low-level quantized noise | 0 / 0 |
| Stationary broadband noise / its inverse | 0 / 0 |
| Three unequal 5 ms impulse pairs, amplitudes swapped by channel | 3 / 3; earliest peak retained |
| 200 impulses / silence | 200 / 0; first 128 listed |
| Three smooth 5 kHz tone bursts / continuous clipped 12 kHz tone | **60 / 0** |
| Two amplitude steps / constant DC | 2 / 0 |
| First/last-sample impulses | 0 / 0 |
| 100-sample short input | null / null |
| 181 s input with impulses at 179 and 180.5 s | 1 / 1 |

The three musical bursts producing 60 envelope peaks is a material limitation,
not a claim of 60 physical clicks. Ripple and decay can produce multiple maxima
per attack. The paired-impulse experiment also demonstrates the deliberate
earlier-versus-taller selection difference. Do not tune away these controls or
use the resulting rate to confirm a medium. Event clustering and validated click
classification would be separate work.

The complete release suite passed **65 tests**: 21 unit, 4 AAC, 13 core/CLI,
5 segment/MQA, 7 noise, 2 parity/integrity, 3 structure, 5 transform, 5 transient.
Formatting, all-target Clippy with warnings denied, and no-CLI Android ARM64
compilation passed. These are compilation checks, not Android linking or phone
behavior. The full debug suite was not rerun.
The release executable reports `0.8.0` and passed a short-prefix text smoke check.
A publication-only copy in `target/source-only-v8` passed production Clippy
(`--lib --bins -D warnings`) with tests, scripts and fixtures absent.

Unchanged generated 48 kHz stereo duration controls passed exact PCM hashes:

| Duration | Peak Windows working set | Wall time |
| --- | --- | --- |
| 10 s | 20.11 MiB | 0.78 s |
| 120 s | 23.76 MiB | 2.59 s |
| 600 s | 25.64 MiB | 8.89 s |

These runs overlapped oracle/Android checks and are not controlled performance
comparisons. Existing temporal spectra grow to 180 seconds; transient storage is
fixed. No new private recordings, encoder/resampler matrix, full private-corpus
run, or source-accuracy evaluation was performed for v0.8. Existing AAC trim misses
and source ambiguities remain; no threshold was retuned to hide them.

Local-only evidence: `corpus/local/generated/transients-v8/`,
`corpus/local/results/transients-v8/`, `corpus/local/results/resources-v8/`,
`tests/transients.rs` and `scripts/check_transients.py`. These are excluded from
source publication and are not backed up by a source-only Git push.
