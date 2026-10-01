# Version 0.5 spectral structure — 2026-10-01

This milestone ports the pinned Python `_aucdtect_features` measurements into
the offline Rust core. It does **not** implement or validate the auCDtect product,
confirm a source medium, or classify recording history. Schema is `0.5.0`;
policy is `observations-only-v5`. Ancestry stays `INCONCLUSIVE`, evidence index
null, and the new feature outcomes are measured/inconclusive/unsupported.

## Reference assessment and scope

The reference remains clean at
`c6ecce2296256b516709d87088896d1be913908c`. `_aucdtect_features` computes a
scatter bound and phase entropy, then its caller turns low bounds and high
entropy into codec penalties. Those interpretations were not ported. Noise,
filters and codec processing can share these statistics; spectral scatter also
shares underlying evidence with existing wall/cutoff measurements.

The neighboring `_silence_and_vinyl` and `_cassette_source` paths combine noise
filtering, autocorrelation, temporal variation and click counting with source
labels and score bypasses. Those paths remain unimplemented. Their underlying
measurements require separate bounded designs and validation, and synthetic
hiss/clicks alone cannot confirm vinyl or cassette provenance.

## Implementation and deliberate differences

`src/detectors/structure.rs` consumes each native channel's second-pass STFT.
`StreamingStft::push_spectrum` borrows the existing positive-frequency complex
bins; the public magnitude-only `push` API is preserved. There is no additional
FFT, input capture or decoding pass. Activity uses the existing per-channel
global peak and strict frame-end convention. Packet-level cancellation and
deadlines cover this streaming work, alongside the existing transform controls.

Scatter follows the reference geometry: magnitudes normalized to frame peak,
relative dB clamped at -110, natural-log power, five-bin population standard
deviation, then five-bin average with nearest-edge padding. The bound is the
last bin at or above `min(0.6, 0.25 * maximum_scatter)`.

Intentional changes:

- Rust uses centered f64 differences for variance instead of subtracting f32
  first/second moments. A maximum scatter <=1e-6 is inapplicable. The Python
  zero-threshold comparison can otherwise place an entirely flat spectrum's
  bound at Nyquist. At least four usable bound frames are required.
- Every active frame contributes. Python subsamples bound frames once there
  are more than 2500. A fixed 2049-counter histogram supplies mean and exact
  modal bound; the lowest bin wins a tie. Python's mode is instead the center
  of a 20-bin histogram whose edges depend on observed extrema. The Rust mode
  does not imply a probability or represent Python's histogram-center statistic.
- Native channels are independent. Python's top-level path generally uses mid;
  no downmix or source-profile veto is introduced here.
- Phase uses f64 angles of the existing complex f32 coefficients, wrapped into
  [-pi, pi], accumulated into 36 equal bins. Bins start at the first actual
  frequency >=10 kHz; Nyquist's real-valued bin is excluded. Python starts from
  a floored index (possibly below 10 kHz) and includes Nyquist.
- Both adjacent frames must be active, and a bin's magnitude must exceed both
  `1e-4 * frame_peak` (-80 dB relative magnitude) and `1e-8` unnormalized FFT
  units in **both** frames. A frame pair needs two shared bins; at least three
  usable pairs are required for entropy. Python counts all high-band phases,
  including nearly empty bins, and compares compacted active rows across gaps.
  These new gates establish applicability, not calibrated detection confidence.
- No usable phase band gives unsupported; insufficient energy or sample count
  gives inconclusive with null measurements. Quiet phase is not fabricated as
  zero entropy. A measured zero means a supported, single-bin phase histogram.

The `spectral_structure` report includes separate scatter/phase statuses, STFT
and active frame counts, usable/flat scatter counts, usable phase frame/bin-pair
counts, actual high-band start frequency and the evaluated STFT interval. This
interval is an envelope; gates can exclude observations inside it, and trailing
samples not in a complete strict-boundary STFT frame are excluded. No unbounded
list of per-frame intervals or full spectrogram is stored.

Per channel the new persistent arrays contain four 2049-element u64/f64 vectors,
2049 booleans and 36 u64 histogram counters: 67905 payload bytes, about 66.4 KiB,
plus small object/allocation overhead. Buffer sizes do not depend on duration or
metadata. This is not a hard cap on the whole process or dependency allocations.

## Frozen numerical comparison

`scripts/generate_structure_reference.py` reads six existing generated WAV/FLAC
fixtures and writes only `tests/fixtures/structure_reference.json`. It records
both the unchanged Python outputs and an independent NumPy implementation of
the declared differences (sliding-window standard deviations and phase
histograms). It uses NumPy 2.3.5, SciPy 1.17.1 and FFmpeg 7.1.1. Existing audio
and historical oracles were not changed. Ordinary Cargo tests need no Python,
FFmpeg or private recordings.

The three new release integration tests passed. On nine native channels from
noise, lowpass, activity, 96 kHz, wall and AAC controls:

- Decoded PCM SHA-256 matches independent FFmpeg output exactly.
- STFT, activity, bound, flat, phase-frame-pair and phase-bin-pair counts match
  the NumPy oracle exactly.
- Mean and modal bound errors are allowed at most one FFT bin (`sample_rate /
  4096` Hz), reflecting hard threshold decisions on f32 FFT results. Mean bounds
  also meet that tolerance against the unchanged Python implementation.
- The actual high-band starting frequency agrees within 1e-9 Hz.
- Phase entropy agrees within 0.001 bits against the gated NumPy calculation.
  The original ungated Python entropy is retained for comparison, not used as
  a parity target for the changed applicability policy.

The tests also check native anti-phase stereo, silent-channel isolation, null
measurements for silence and short data, exact STFT boundary behavior, prefixes,
unsupported low-rate phase bands, JSON serialization, and absence of new source
classification outcomes. Unit checks cover flat spectra, shared energy, Nyquist
exclusion, inactive gaps and accumulation beyond the old 2500-frame boundary.

## Generated processing controls

`scripts/check_structure.py` completed nine generated stereo cases, with exact
decoded PCM SHA-256 agreement against FFmpeg for every file. All ancestry
verdicts stayed inconclusive with null evidence indices. Actual observed values:

| Generated processing | Scatter mean | High-band phase |
| --- | --- | --- |
| White noise, 48 kHz | 23984 / 23991 Hz | 5.1691 / 5.1688 bits |
| Same noise, 15 kHz lowpass | 15144 / 15137 Hz | 5.1679 / 5.1671 bits |
| Same noise, 9 kHz lowpass | 9135 / 9139 Hz | Inconclusive, null; no eligible pairs |
| 12 kHz tone | 12082 Hz both channels | Measured zero; 132 bin pairs per channel |
| Noise with 0.5-second silence gap | 23978 / 23987 Hz | 33 usable adjacent pairs among 35 active frames |
| Native anti-phase noise | Same measurements in both channels | Neither channel cancels |
| Noise left / silence right | Left measured; right null | Left measured; right inconclusive/null |
| Exact silence | Inconclusive/null | Inconclusive/null |
| White noise, 16 kHz | 7996 / 7992 Hz | Unsupported/null |

The lowpass controls show that a reduced scatter bound is possible without lossy
encoding. Both full-band and filtered noise have high phase entropy. These are
related engineering controls, not independent recordings or specificity estimates.
The experiment assertions verify applicability and consistency, not a classifier.
(All nine inputs are also checked for conservative
report outcomes and independent PCM agreement.)

Raw reports, commands/history, versions and observations are local and ignored:

```text
corpus/local/results/structure-v5/manifest.json
corpus/local/results/structure-v5/summary.json
corpus/local/results/structure-v5/observations.json
```

## Build and regression checks

One full **release** suite passed 53 tests, with zero failures/ignored tests:
21 unit, 4 AAC integration, 13 core/CLI, 5 segment/MQA, 2 original parity/integrity,
3 structure and 5 transform tests. The shared FFT change therefore passed the
previous decoder, magnitude, AAC, Vorbis, resampling and MQA regressions as well
as the new structure checks. The full debug suite was not rerun for v0.5.

Clippy passed on all targets with warnings denied; formatting passed. ARM64
Android library compilation passed without the CLI feature. Python scripts
compiled without syntax errors. The release CLI reports `0.5.0`; a 0.1-second
text-output prefix smoke check correctly returns null structure measurements
for insufficient frames. The separate final release build passed.

Commands used on the local Rust/Cargo 1.98.1 GNU toolchain (host linker
`C:\msys64\ucrt64\bin\gcc.exe`):

```powershell
./scripts/cargo.ps1 test --release --locked --offline --test structure
./scripts/cargo.ps1 test --release --locked --offline
./scripts/cargo.ps1 build --release --locked --offline
& ./scripts/cargo.ps1 @('fmt', '--all', '--', '--check')
& ./scripts/cargo.ps1 @('clippy', '--locked', '--offline', '--all-targets', '--', '-D', 'warnings')
./scripts/cargo.ps1 check --lib --no-default-features --target aarch64-linux-android --locked --offline
python -m py_compile scripts/generate_structure_reference.py scripts/check_structure.py
python scripts/generate_structure_reference.py
python scripts/check_structure.py
python scripts/validate_local.py corpus/local/generated/resource-check --output corpus/local/results/resources-v5 --source-history 'Generated sine controls reused unchanged; v0.5 spectral structure'
```

The existing generated stereo 16-bit/48 kHz duration controls were reused without
changing their audio. All three matched FFmpeg PCM exactly:

| Duration | Peak Windows working set | Process wall time |
| --- | ---: | ---: |
| 10 seconds | 19.59 MiB | 0.685 s |
| 120 seconds | 19.79 MiB | 2.505 s |
| 600 seconds | 19.77 MiB | 9.681 s |

Raw data is local and ignored at `corpus/local/results/resources-v5/summary.json`.
These are whole-process Windows observations. A release build ran concurrently;
timing is not a controlled performance comparison. Approximately flat memory is
consistent with the fixed buffers, not a hard allocation guarantee or phone result.

NDK linking, APK creation, phone execution, remote CI and independent grouped
accuracy evaluation were not run. The private music collection was not rerun;
the historical v0.3 collection results and known corrupt inputs remain unchanged.

## Next milestone

Design bounded silence/noise-band measurements before attempting analog-source
profiles. Preserve unknown source history, per-channel applicability and the
known AAC trim miss. The remaining core acceptance gates, grouped corpus
evaluation, NDK linking and Android app work remain unchanged.
