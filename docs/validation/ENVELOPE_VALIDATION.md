# Version 0.10 cross-band envelope correlation — 2026-10-01

Schema `0.10.0`, policy `observations-only-v10`. Reports retain ancestry
`INCONCLUSIVE` and evidence index null. A relationship between spectral bands
does not establish authenticity, injected noise or codec history.

## Reference and scope

The pinned Python baseline remains `c6ecce2296256b516709d87088896d1be913908c`.
Its `_ultrasonic_envelope_correlation` computes Pearson correlation between the
per-frame RMS FFT magnitudes in nominal 1–8 and 16–22 kHz bands. It returns one
for insufficient frames or unavailable bands, zero for constant envelopes, and
uses the result in source-history interpretations. Those fallback measurements
and interpretations are not ported. The upper band includes audible frequencies;
the Rust feature is named band-envelope correlation rather than ultrasonic proof.

Rust preserves paired, signed Pearson correlation with bounded online statistics.
It requires complete physical bands, useful mean energy and temporal variation.
Actual bins, coverage, counts and applicability are reported per native channel.
The v0.7 circular correlation of samples within individual band-filtered windows
is a different statistic and remains separately named and reported.

## Numerical contract

The existing second-pass 4096-sample symmetric-Hann STFT supplies f32 magnitudes.
Frame hop remains 2048, with strict end-before-EOF and the native channel's global
-60 dB activity mask. Every active frame contributes to both envelopes, including
frames where one band is quiet. No per-band frame deletion, audio concatenation
across gaps or extra transform is introduced. The interval encloses all STFT
windows; total and active counts describe the selection. Prefix limits apply.

Select actual bin centers inclusively inside [1000, 8000] and [16000, 22000] Hz.
Each complete nominal band must lie strictly below Nyquist and have at least two
bins. A missing band gives unsupported/null coefficient. In particular, 44,000 Hz
is unsupported while 44,001 Hz has complete geometry. Low-rate generated carrier
controls contain aliased frequencies and are used only to verify unsupported
band geometry, not as examples of physical high-band content.

Per frame, band power is `2 * sum(magnitude[k]^2) / (4096 * sum(Hann^2))`.
Hann coefficients are the same rounded f32 coefficients as the STFT; powers and
subsequent reductions use f64. Broadband power uses the same normalization with
weight one for DC/Nyquist and two elsewhere. Each envelope sample is the square
root of its band power, in normalized RMS-amplitude units. This differs from
Python's unnormalized mean-bin power, but positive constant scaling of each
envelope does not change Pearson correlation when geometry and frame selection
are identical. Geometry and applicability intentionally differ from Python.

Welford updates retain paired means, centered sums of squares and covariance.
Population standard deviations are `sqrt(M2 / active_frames)`. The coefficient
is `covariance / sqrt(M2_mid * M2_high)`, clamped to [-1, 1] for roundoff only.
The coefficient is not a probability or a source classification.

At least ten active frames are needed, and both bands must pass:

- Mean band power strictly greater than `1e-12` (-120 dBFS power) and `1e-8` of
  mean broadband power (-80 dB relative power).
- Envelope population standard deviation strictly greater than `1e-8` amplitude
  and `1e-3 * mean_envelope` (a 0.1% coefficient-of-variation floor).

These are numerical applicability guards, not calibrated accuracy thresholds.
They are applied to aggregate active-frame statistics. A usable mean can include
individual quiet frames; gaps and common modulation can affect correlation.
Unavailable or insufficient/quiet/constant envelopes give null coefficients,
not fabricated zero or one. Measured zero represents usable uncorrelated
envelopes. Supported bands with active samples still expose observed power,
mean and SD even when correlation eligibility fails; no-data values remain null.

Reports add `envelope` records and a `band_envelope_correlation` observation in
the spectral-measurements family. Band statuses, actual bins, mean powers, means,
SDs and separate energy/variation flags explain eligibility. The engine retains
only constant-size running state, with no frame history, new FFT or decode pass.
Existing packet cancellation/deadline and cross-pass PCM integrity checks apply.
Storage is independent of duration; dependency allocations are not a hard process
memory guarantee.

## Independent controls

Four local Rust integration tests cover signed correlation, native antiphase/gain,
silent channels, constant/quiet/missing bands, inactive gaps, full-band geometry
from 8–384 kHz and the exact nine-versus-ten-active-frame boundary under prefixes.
The initial JSON roundtrip assertion differed by one least-significant f64 bit
(-0.9999985916698473 versus -0.9999985916698472) with default JSON parsing. It now
checks the coefficient within 1e-14 and retains exact frame counts. No production
arithmetic, gates or historical oracle was changed in response.

`scripts/check_envelope.py` independently computes NumPy spectra of f32-windowed
samples, inverse-transforms each masked band, and measures **time-domain** power
and envelope statistics. `numpy.corrcoef` compares the stored paired envelopes;
it does not reproduce Rust's online covariance update. Expected values are saved
before running Rust; FFmpeg independently checks exact s32le PCM hashes.

Declared tolerances: 0.001 absolute coefficient; `1e-14 + 1e-5*abs(expected)`
power; `1e-8 + 1e-5*abs(expected)` envelope mean/SD; 1e-9 Hz bin geometry. Counts,
statuses, intervals and energy/variation flags must agree exactly. NumPy version
2.3.5; generated audio is stereo 24-bit WAV, using FFmpeg 7.1.1 for comparison.

All **18 generated files / 36 native channels** passed numerical checks and exact
PCM hashes. Maximum coefficient discrepancy was 1.249e-8.

| Control | Result |
| --- | --- |
| Shared modulation at 44,001/44,100/48,000/96,000/384,000 Hz | Coefficient approximately +1; native antiphase/half-gain agrees |
| 8,000/32,000/44,000 Hz geometry | Unsupported; no coefficient |
| Opposing modulation | Approximately -0.99999859 |
| Different 1 Hz / 3 Hz modulation | Approximately -0.00094943 |
| Constant envelopes, one missing band, tiny signal, silence | Inconclusive; no coefficient |
| One silent native channel | Active channel measured; silent channel inconclusive |
| Inactive middle second | Approximately +1, with fewer active than total frames |
| Two generated independent noise channels | Approximately -0.17277 / -0.19490 |
| Exact 4096-sample input | No STFT frame; inconclusive |

Finite independent noise need not produce exactly zero correlation. None of
these generated cases establishes source accuracy or justifies a confidence score.

All **73 release tests** passed: 21 unit, 4 AAC, 13 core/CLI, 5 segment/MQA,
4 envelope, 7 noise, 2 parity/integrity, 4 roll-off, 3 structure, 5 transform and
5 transient. Formatting, all-target Clippy with warnings denied, Android ARM64
library compilation without CLI and production Clippy in a publication-only
copy (`target/source-only-v10`) passed. The `0.10.0` CLI passed a short-prefix
text smoke check. Full debug tests and remote CI completion were not verified;
Android compilation does not establish linking or phone behavior.

The unchanged 10/120/600-second generated duration controls matched exact PCM and
used 20.05/23.64/25.60 MiB peak Windows working set, in 1.80/7.10/22.62 seconds.
These runs overlapped compilation and other checks; they are not controlled speed
comparisons. No private audio, encoder/resampler experiment matrix, full private
collection, MQA confirmation or independent grouped source evaluation was run.

Local evidence: `corpus/local/generated/envelope-v10/`,
`corpus/local/results/envelope-v10/`, `corpus/local/results/resources-v10/`,
`tests/envelope.rs` and `scripts/check_envelope.py`. These remain excluded from
publication and are not backed up by a source-only Git push.
