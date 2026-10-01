# Version 0.7 band correlation and temporal variation — 2026-10-01

Schema `0.7.0`, policy `observations-only-v7`. Native-channel descriptive
measurements only: ancestry stays `INCONCLUSIVE`, evidence index null. These
features do not confirm vinyl, cassette, codec history or source authenticity.
Version 0.6 band-power definitions remain in `NOISE_VALIDATION.md`.

## Design and reference differences

The pinned Python remains clean at `c6ecce2296256b516709d87088896d1be913908c`.
Its source profiles mask a whole-prefix FFT, inverse-transform the selected band,
measure absolute Pearson correlation at a lag, and compute population standard
deviation of one-second RMS levels. Insufficient/quiet data can return zero. The
profiles then infer analog sources and alter scores. Those interpretations and
whole-prefix buffers are not ported.

Rust v0.7 measures the periodic band-limited signal defined by each existing
4096-sample symmetric-Hann STFT window. This gives bounded arithmetic without a
third decode pass or another FFT. It is deliberately **circular autocorrelation**,
not a substitute implementation of full-stream Pearson correlation. The exact
statistic is tested against an independent inverse-FFT/time-domain calculation.

The existing `high_band` and `above_cutoff_band` gain two correlation records and
a temporal-variation record. Requested/actual band edges, bin count and original
all/quiet powers are unchanged. New correlation/variation always uses all eligible
windows, not just quiet passages; quiet power remains separately reported.

## Correlation contract

For each selected non-DC/non-Nyquist bin k, let P[k] be the sum of squared complex
FFT magnitudes over **all** analyzed STFT windows. At lag L, report the signed
coefficient `sum(P[k] * cos(2*pi*k*L/4096)) / sum(P[k])`, clamped to [-1, 1] only
for floating-point overshoot. This equals summed circular covariance divided by
summed variance of the band-masked windowed signals. The filtered signal has no
DC component. The statistic weights windows by their band energy; it is not the
unweighted mean of per-window normalized correlations.

Two lags follow the reference's nominal scales:

- `max(25, round(50 * sample_rate / 44100))` samples;
- `max(50, round(100 * sample_rate / 44100))` samples.

Rust rounds positive half-integers upward; Python's round-to-even can differ at
some unusual integer sample rates. Reports contain actual lag samples and seconds.
The minimum lags mean the physical time offset changes at low sample rates.

A band must contain at least two actual bins and have at least four STFT windows.
Its normalized mean-square energy must be **strictly greater** than both 1e-12
(-120 dBFS) and 1e-8 of the all-band window-normalized power (-80 dB relative
power). Broadband power includes DC/Nyquist with their proper one-sided weights.
Insufficient energy/windows gives inconclusive/null; unavailable geometry gives
unsupported/null. Exact-zero correlation is only reported when a usable band
actually produces it. The energy gates establish numerical applicability, not
statistical confidence or guaranteed rejection of spectral leakage.

Correlation intervals are the existing all-window support envelope, excluding
unframed tail samples. Within each window the filtered samples are periodic:
lag comparisons wrap around. Window taper, finite resolution, band edges and
band-limiting white noise all affect correlation. No continuity across distinct
STFT frames is assumed. A signed result retains information that Python's
absolute value discards; no correlation threshold classifies a source.

## Temporal-variation contract

Retain per-bin sums for the first 180 one-second blocks of the analyzed interval.
Each block accepts only STFT windows entirely inside that second. Boundary-crossing
windows are excluded; sample intervals in the report are the exact support
envelopes of included windows. Blocks use the original global 2048-sample hop grid;
the grid does not restart at each second. The shared strict EOF boundary remains.

Only completed seconds appear in the final record; a trailing partial second is
discarded even if it contains complete windows. The block list is bounded to 180.
Each listed block includes its second index, support interval, window count,
mean-square band power, RMS dBFS and energy eligibility. The declared search limit
is 180 * sample_rate; actual intervals show how much was examined.

Power normalization is unchanged from v0.6. Every block must individually exceed
the same absolute/relative energy floors, using its own broadband reference.
At least two completed blocks are required. When all are eligible, report the
population standard deviation of their `10*log10(mean_square)` levels in dB.
Otherwise return inconclusive/null, preserving all block measurements and the
eligible count. Zero/quiet blocks are never silently dropped to create an
apparently stable noise floor. An unavailable band is unsupported.

This is variation of window-averaged band-power estimates within seconds; it
does not equal RMS variation of a continuously filtered stream. Filters, tones,
music, quantization and leakage may all contribute. No arbitrary log floor, silence
ratio, analog bypass, source label or score is added.

## Resource and cancellation bounds

`src/detectors/noise_dynamics.rs` retains at most 180 * 2049 f64 values per channel:
2,950,560 bytes (about 2.82 MiB), plus counters and bounded vectors. Allocation grows
with observed seconds only until the cap, independently of sample rate. The
existing v0.6 fixed accumulators remain about 48.3 KiB/channel. Two bands reuse
the same temporal spectra and need no retained audio or inverse FFT workspace.
Report block lists are also bounded to 180 per band. This is not a process-wide
allocation limit for decoders or other components.

Global correlation and v0.6 all/quiet powers continue across the full analyzed
stream after the temporal cap. Cutoff-dependent bands are selected only after
the channel cutoff is finalized. Packet-level cancellation still covers streaming
work; final reductions also check cancellation/deadlines between native channels.
These checks cannot interrupt blocking I/O or an individual decoder call.

## Numerical validation

Seven local noise integration tests pass in release: the four v0.6 regressions
plus analytic tone/known-level-step checks, silence/insufficient/partial-block
checks, and a 182-second cap boundary. Analytic checks cover signed circular tone
correlation, expected dB standard deviation of three known amplitude levels,
native anti-phase preservation, actual window support and null outcomes. The
cap test verifies all-stream coverage continues after temporal history stops.

`scripts/check_noise_dynamics.py` passed **16 generated stereo cases / 32 native
channels**, all with exact FFmpeg-decoded PCM SHA-256. Seven new cases supplement
the unchanged nine v0.6 WAVs: changing levels, stationary hiss, a silent middle
second, low/high tones, sub-floor energy, impulses and a 182-second 8-kHz case.
Together they cover 8/16/44.1/48/96/384 kHz, unavailable bands and exact zeros.

Expected values are computed and saved **before** invoking Rust. NumPy 2.3.5
performs an independent f64 forward/inverse FFT on the declared f32 windowed
input. It masks each frequency band, reconstructs samples, computes correlation
with a circular sample shift and calculates power directly from squared filtered
samples. It independently finds cutoff, second boundaries, window selection,
energy eligibility and standard deviation. It does not copy Rust coefficients
or use Rust's cosine-weighted spectral reduction as the oracle.

Acceptance criteria, chosen before comparison:

- PCM hashes, interval/window/block counts, eligibility, statuses and nulls: exact.
- Cutoff and band geometry for these controls: 1e-9 Hz.
- Band/block mean-square power: absolute error <= `1e-14 + 3e-5 * expected`.
- Correlation coefficient: absolute error <= 0.001.
- Temporal level SD and block dBFS above -110 dBFS: absolute error <= 0.005 dB.
  Lower block levels use the linear-power criterion because of the f32 floor.

No numerical comparison failed and no oracle was regenerated to make a failure
pass. Local expectations/reports/summary are under
`corpus/local/results/noise-dynamics-v7`; the seven new generated WAVs are under
`corpus/local/generated/noise-dynamics-v7`. Source scripts/tests and raw local
evidence remain excluded from source-only publication.

## Resource observations and limits

Existing generated stereo 16-bit/48-kHz sine controls at 10/120/600 seconds matched
FFmpeg PCM exactly. Peak working sets were 19.98/23.61/25.53 MiB; elapsed times
2.87/4.77/18.37 seconds. The increased memory reflects bounded per-second history,
which stops accumulating at 180 seconds. Compilation/tests overlapped, so timings
are not controlled speed comparisons or Android measurements. Raw results:
`corpus/local/results/resources-v7/summary.json`.

The full release suite passed all 60 tests with no failures/ignored tests.
Formatting, all-target Clippy with warnings denied and Android ARM64 no-CLI
library compilation passed. The release CLI reports version 0.7.0. Production
Clippy also passed on a source-only copy with no tests/scripts/fixtures, and the
short-prefix text CLI smoke check passed. The debug suite, private music
collection and broad encoding matrix were not rerun for this
measurement milestone. Android linking/device behavior and independent source
accuracy remain unverified; no overall scoring policy is enabled.

## Next milestone

Assess bounded click/transient measurements and the reference's analog-profile
rules. Establish ordinary musical attacks, clipping, silence and generated clicks
as controls before implementing any transient count. Noise correlation or stable
hiss must never act as a source-confirmation shortcut. Continuous filtered-stream
Pearson correlation would be a different future statistic requiring its own
filter/edge/cost validation, not an implied capability of this circular measure.
