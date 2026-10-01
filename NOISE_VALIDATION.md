# Version 0.6 quiet passages and band power — 2026-10-01

Schema `0.6.0`, policy `observations-only-v6`. These are descriptive measurements:
overall ancestry remains `INCONCLUSIVE` and evidence index remains null. No vinyl,
cassette, dither, codec-noise, upscale or authenticity classification is added.

## Reference assessment

The unchanged Python baseline is `c6ecce2296256b516709d87088896d1be913908c`.
`_silence_and_vinyl` finds consecutive samples below -40 dBFS peak for at least
0.5 seconds. It concatenates qualifying runs and compares their high-band power
with a music excerpt, then applies source claims and scores. `_fft_band_extract`
uses whole-prefix FFT masking (up to 180 seconds); `_cassette_source` also uses
filtered noise, correlation, spectral slope and cutoff variation to infer a
source medium. Autocorrelation and temporal-variance helpers return zero even
when data are insufficient. Those classifications, score bypasses, full-prefix
buffers and fabricated successful-zero cases were not imported.

Rust preserves the useful sample threshold, but measures native channels and
keeps quiet windows in their original positions. The normalization and sampling
are deliberately different from Python's whole-excerpt/concatenated Hann FFT.
Numerical parity with the original filtered signal is not claimed. Independent
NumPy computations and analytic tone controls test the declared Rust statistic.

## Measurement contract

`src/detectors/noise.rs` consumes decoded f64 samples and the existing second-pass
complex f32 STFT. Quiet is `abs(sample) < 0.01` for at least `ceil(rate / 2)`
consecutive samples. Exact +/-0.01 ends a run. This means low peak amplitude,
not perceptual silence. It includes DC and weak tones and depends on gain.

Runs begin/end at the analyzed interval boundaries, including a requested prefix.
An EOF/prefix-clipped run qualifies when its observed portion is long enough;
no claim is made about samples outside coverage. The report includes exact sample
counts, qualifying-run count, longest run, and first sixteen intervals with an
explicit truncation flag. Aggregates include every qualifying run, even when its
interval is omitted from the bounded listing. The sample interval includes tails
that do not enter an STFT window.

All complete strict-boundary STFT frames contribute band power, including frames
excluded by the spectral activity gate. Quiet power uses only windows entirely
inside a qualifying run. A pending per-bin accumulator holds a run's windows
until its length is known; too-short runs are discarded. The STFT callback runs
before consuming its lookahead sample, so a loud lookahead cannot discard the
preceding quiet window. Gaps are never concatenated, bridged or cross-faded.

For each band and contributing window, power is:

`2 * sum(re(X[k])^2 + im(X[k])^2) / (N * sum(w[n]^2))`, with `N=4096`.

The symmetric Hann coefficients match the rounded f32 coefficients used by the
STFT; sums and powers use f64. Both bands exclude DC and Nyquist, so every selected
bin has the one-sided factor two. Average the linear powers over contributing
windows, then report `10 * log10(mean_square)` as band RMS dBFS. This is a Hann
window-weighted power estimate, not unweighted full-track RMS or whole-track
brickwall filtering. Overlapping frames are not independent observations.

The bands are:

| Field | Requested lower bound | Requested upper bound |
| --- | --- | --- |
| `high_band` | 16000 Hz | min(22000 Hz, Nyquist - 100 Hz) |
| `above_cutoff_band` | channel cutoff p95 + 1000 Hz | Nyquist - 100 Hz |

Actual inclusive FFT-bin frequencies must lie within the requested band. The
report provides requested and actual edges and bin count. At least two bins
must exist, otherwise the band is unsupported. A missing/nonpositive channel
cutoff also makes the second band unsupported. The cutoff itself retains the
existing active-frame policy; all/quiet power does not use that activity mask.

Each estimate requires four contributing windows. Fewer is inconclusive with
null power/dBFS. All-frame and quiet estimates have separate statuses and counts.
Measured exact-zero power is `0.0`; its logarithm is null. It is not silently
replaced by an arbitrary floor. Positive values below numerical precision can
reflect FFT/window roundoff and leakage. The values must not be interpreted as
a precise analog noise floor, noise origin, source bit depth or source medium.

The new fields are separate from `spectral.noise_above_cutoff_db`, an older
unnormalized magnitude statistic. No silence/music ratio is reported: the Python
denominator, 2-second gate and concatenated 30-second reference are not reproduced.

## Storage and cancellation

Per channel: three 2049-element f64 power buffers (49176 bytes) and at most sixteen
16-byte intervals (256 bytes), plus small counters/object/allocation overhead.
No retained audio, extra FFT or decoding pass; storage is independent of duration
and sample rate. Final band selection uses the finished channel cutoff without a
third pass. Packet-level cooperative cancellation/deadlines still apply; these
checks do not interrupt blocking I/O or an individual decoder call.

## Validation

Four local Rust integration tests cover analytic bin-centered tone power at
44.1/48/96/192/384 kHz, native anti-phase stereo, silent-channel isolation,
exact quiet-threshold crossings, one-sample lookahead, rejected short runs,
EOF flushing, odd-rate minimum duration, prefixes, strict STFT boundaries,
unsupported bands, exact-zero versus null estimates, bounded interval listings
with complete aggregates, and JSON serialization. All four passed in release.

`scripts/check_noise.py` generates nine stereo 24-bit WAV controls under ignored
`corpus/local/generated/noise-v6`. Before running Rust it computes quiet runs by
independent vectorized run boundaries, uses NumPy's f64 FFT on the declared f32
windowed inputs, and computes the active-frame cutoff separately. Expected
numbers and reports are saved in `corpus/local/results/noise-v6`.

Acceptance criteria are fixed before comparison: PCM SHA-256 exact against
FFmpeg s32le; run/window counts, intervals, cutoff and band bins exact for these
controls; power absolute error <= `1e-14 + 3e-5 * expected_power`; dBFS error
<0.002 dB when expected power >1e-12. Exact-zero dBFS must be null. Below that
level the absolute power criterion handles f32 numerical-floor differences.
This tolerance is numerical, not a forensic confidence or detection threshold.

All nine independent stereo controls passed (eighteen native channels), with exact
FFmpeg PCM and all numerical criteria met. The largest absolute power difference
was 3.84e-10 on the loud high-frequency tone (power about 0.045); the faint-hiss
control differed by at most 8.43e-16. Controls include quiet gaps at 44.1/48/96/384
kHz, continuous quiet hiss, a low tone with faint hiss and a silent channel, high
tones at two gains, exact zeros, and unavailable high-band coverage at 16 kHz.
No oracle was regenerated in response to a failed comparison.

The full release suite passed all 57 tests, with no failures/ignored tests.
The four new noise integration tests also passed in debug (including the STFT
lookahead assertion); the full debug suite was not rerun.
Formatting, all-target Clippy with warnings denied, and no-CLI Android ARM64
library compilation passed. The release CLI identifies itself as 0.6.0. A separate
copy containing only publication files passed `clippy --lib --bins -D warnings`.

Existing generated stereo 16-bit/48-kHz sine controls at 10/120/600 seconds passed
exact PCM checks with peak working sets 19.70/19.89/19.91 MiB, respectively.
Elapsed times were 2.0887/5.7580/18.8756 seconds. Compilation/tests overlapped:
these are bounded-storage observations, not controlled speed benchmarks or
Android resource measurements. Raw results: `corpus/local/results/resources-v6`.

The full debug suite, remote CI completion, Android NDK linking/device behavior
and grouped accuracy evaluation were not verified. The private recording
collection was not rerun; its latest full-file validation remains v0.3.

## Remaining work

Historical v0.6 next step below: v0.7 now implements explicitly circular band
correlation and bounded block variation; see `NOISE_DYNAMICS_VALIDATION.md` for
the completed scope and its differences from continuously filtered signals.

Design bounded band-limited correlation/temporal-variation measurements with
explicit intervals, leakage handling and insufficient-data outcomes before
adding click or analog profiles. Correlation of raw broadband samples must not
be presented as correlation of a filtered noise band. Low noise, stable hiss,
clicks or high silence-band energy alone cannot establish source history.

Source-only publication deliberately omits tests, scripts and generated/private
evidence. Public CI checks production builds/lints and Android target compilation;
full local tests need the local fixtures (including an internal AAC oracle).
