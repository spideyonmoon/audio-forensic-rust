# Version 0.9 spectral roll-off — 2026-10-01

Schema `0.9.0`, policy `observations-only-v9`. This is a native-channel spectral
measurement, not an analog-source classifier. Ancestry stays `INCONCLUSIVE` and
evidence index stays null. Equalization, filters and musical spectra can produce
the same endpoint contrast as a purported source profile.

## Reference assessment

The unchanged Python baseline is `c6ecce2296256b516709d87088896d1be913908c`.
`_cassette_source` computes 20 nominal 500 Hz bands spanning 12–18 kHz, but its
slope uses only the first and last. It averages log magnitudes from the mean
active spectrum and divides the endpoint difference by six kHz. Unsupported
bands receive -120 dB placeholders, and selected slopes change a cassette score.
Cutoff variation elsewhere in that profile is described as wow/flutter without
independent validation.

Rust retains the useful two-band contrast and makes its interpretation explicit.
It omits unused intermediate bands, synthetic out-of-range levels, score changes
and medium labels. Existing cutoff variation is not relabeled tape speed error.
This is an endpoint difference, not a regression slope, derivative, monotonicity
test or proof of natural roll-off throughout the intervening frequencies.

## Measurement contract

The second decode pass feeds the existing 4096-sample, 2048-hop, f32 symmetric-Hann
STFT magnitudes into one f64 per-bin sum. Each native channel uses its existing
global -60 dB activity mask. Every active frame contributes; inactive gaps are
not concatenated into synthetic continuous audio. At least four active frames
are required. The interval is the support envelope of all STFT windows, with
separate total/active counts; it is not a claim that every covered sample was
used. The shared strict frame-end rule and requested prefix apply unchanged.

For each bin, define `A[k] = 2 * mean_active_magnitude[k] / sum(Hann)` using the
same rounded f32 Hann coefficients as the STFT. This is a coherent-gain amplitude
convention for a non-DC/non-Nyquist sinusoidal bin. It is **not** integrated band
RMS, power spectral density or a rate-independent broadband-noise level.
The reference amplitude is `max(A[k])` over the active mean spectrum, including
DC/Nyquist as a common normalization reference. Endpoint bins exclude those bins.

Endpoint bands are [11,750, 12,250] Hz and [17,750, 18,250] Hz, including bins whose
actual centers fall on the edges. Each complete nominal band must lie strictly
below Nyquist and include at least two bins. Otherwise that band is unsupported,
with null actual geometry/level; the aggregate slope is unsupported if either
endpoint is unavailable. Thus a 36,500 Hz sample rate is unsupported, while
36,501 Hz has valid geometry. The complete requested band is never silently
clipped or filled with a made-up floor.

Every selected bin must strictly exceed both `1e-6` absolute coherent amplitude
(-120 dBFS in that convention) and `1e-4 * reference_peak` (-80 dB relative
amplitude). Bands expose their bin counts, eligible counts and minimum amplitude
for inspection. A supported band with too few active frames or any ineligible
bin is inconclusive, and its mean level is null. These are numerical applicability
gates; they do not calibrate accuracy, reject all leakage or establish bandwidth.

For a measured band, average `20*log10(A[k]/reference_peak)` equally across its
selected bins. The final result is `(upper_level - lower_level) * 1000 /
(upper_mean_bin_hz - lower_mean_bin_hz)`. Using actual mean bin frequencies
avoids pretending all sample rates place bins exactly at 12 and 18 kHz. It
deliberately differs from Python's floored/exclusive band edges and fixed 6 kHz
denominator. Global gain cancels from the slope while energy gates remain passed.

Reports expose channel, aggregate/band statuses, exact geometry, counts, levels,
reference amplitude and slope in dB/kHz. Empty/silent data yield null slope;
measured zero means usable endpoint bands with equal log levels. The new
`spectral_rolloff` detector belongs to the existing spectral-measurements family
and has no hit status or source threshold. It shares information with other
spectral features and must not be counted as independent evidence automatically.

Storage is one 2049-element f64 sum (16,392 bytes/channel), plus fixed bookkeeping
and two band records. It does not grow with duration. No additional transform,
sample capture, decode pass or dependency is introduced. Existing packet-level
cancellation, deadline checks and cross-pass exact PCM integrity checks apply.

## Controls and validation

Local integration tests exercise generated EQ, antiphase/gain invariance, geometry
from 8 to 384 kHz, exact minimum-frame/prefix boundaries, inactive gaps, silence,
quiet audio, sparse spectra and JSON roundtrip.

Initial test development produced three failures: the EQ generator exceeded the
core's documented absolute float limit of 16, and two tests intended to have flat
spectra placed two equal impulses in each window, creating a spectral comb. The
first was corrected by normalizing the generated signal; the flat controls now
have one impulse per window. The original gapped comb is retained as an explicit
abstention control. No production thresholds or historical reference outputs were
changed to make these tests pass. Clippy also requested simplifying `end >=
start + 1` to the equivalent `end > start`.

The independent local `scripts/check_rolloff.py` computes NumPy FFTs of rounded
f32 windowed decoded samples, applies the activity mask, and calculates actual
band geometry and levels. It writes expected values before invoking Rust and
compares exact s32le PCM hashes with FFmpeg. Tolerances declared before comparison:
0.005 dB for endpoint levels, 0.002 dB/kHz for slope, `1e-9 + 1e-4 * abs(expected)`
for coherent amplitudes and 1e-9 Hz for geometry. Statuses, counts and intervals
must agree exactly. These checks concern arithmetic/applicability, not forensic
accuracy or equivalence to the complete Python classifier.

All 18 generated stereo 24-bit controls / 36 native channels passed this oracle.
The largest slope discrepancy was 2.518e-8 dB/kHz.
Flat single-impulse spectra measured zero at supported rates. The 36,500/36,501 Hz
boundary correctly changed unsupported to measured. White noise measured about
0.03186 dB/kHz; adding known -4/+4 dB/kHz EQ changed it to about -3.96811/+4.03184.
Antiphase and half-amplitude derivatives retained the slope within numerical
tolerance. The gapped comb, silence, very quiet quantized noise, tones, short
input, strong low-pass/brickwall filters and a missing endpoint band abstained.
An inactive-gap control remained measured with explicit active/window counts.

All 69 release tests passed: 21 unit, 4 AAC, 13 core/CLI, 5 segment/MQA, 7 noise,
2 parity/integrity, 4 roll-off, 3 structure, 5 transform and 5 transient.
Formatting, all-target Clippy with warnings denied, no-CLI Android ARM64
compilation and production Clippy in `target/source-only-v9` passed. The source
copy contained no tests/scripts/fixtures. Android compilation does not establish
linking or phone behavior. Full debug tests and remote CI were not verified.
The `0.9.0` release CLI also passed a generated EQ short-prefix text smoke check.

The unchanged generated 48 kHz stereo 10/120/600-second duration controls matched
exact PCM hashes and used 20.16/23.79/25.71 MiB peak Windows working set, in
0.55/2.40/8.43 s. These overlapped compilation and are not controlled timing
comparisons. The new accumulator is fixed-size; the existing temporal spectra
still account for growth through 180 seconds. No new encoder/resampler matrix,
private-corpus run or independent source-accuracy evaluation was performed.

Local evidence is under `corpus/local/generated/rolloff-v9/` and
`corpus/local/results/rolloff-v9/`; tests/script are `tests/rolloff.rs` and
`scripts/check_rolloff.py`. These are excluded from publication and are not part
of a source-only Git backup. No private recording was used or modified.
