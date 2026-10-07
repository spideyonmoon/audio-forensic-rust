# Version 0.3 resampling and Vorbis validation — 2026-09-30

This is the historical v0.3 record. The subsequent AAC implementation and its
checks are recorded separately in [AAC_VALIDATION.md](AAC_VALIDATION.md).

This milestone implements two additional observation families. Resampling
candidates describe spectral patterns near a lower sample rate's Nyquist.
Vorbis candidates describe a persistent transform-coefficient grid. Neither
establishes a verified source history; ancestry stays `INCONCLUSIVE` and the
evidence index stays null. The earlier validation documents preserve their own
version-specific results.

## Implementation and resource contracts

Resampling consumes the existing active STFT frames and stores only per-bin
sums and running mirror correlations. It checks 44.1/48/88.2/96 kHz hypotheses
when at least 1.4 kHz of room exists above the candidate Nyquist. All matching
rates and modes are retained, unlike Python's first-match return. At least eight
active frames and sufficient energy near the candidate edge are required.
Mirror correlation is unavailable when either comparison band is too quiet.

The notch/wall/mirror gates otherwise follow the pinned reference. They can be
mimicked by filters and spectral gaps. A low-pass at 22.05 kHz can trigger a
44.1 kHz hypothesis without any rate conversion; this observation does not prove
resampling, counterfeit mastering or lossy encoding. It shares underlying wall
information with spectral features and must not be scored as independent evidence.

Vorbis uses the reference's sine-of-sine-squared window and a newly implemented
orthonormal MDCT. The kernel folds the windowed signal and evaluates a DCT-IV
with a complex FFT. Direct cosine-definition tests check signs, phase and scale.

The collector stores at most twelve 3071-sample spans per channel, selected from
the first min(analyzed duration, 180 seconds). Native channels are preserved.
It searches all 1024 sample offsets per span, pooling equivalent phases modulo
128, and compares numerical zeros with median off-grid occupancy. The sampled
frame intervals are explicit in JSON. Fewer than four active probes abstains.
The search is limited to 44.1/48 kHz and the reference's 2048/256 block geometry.
Cancellation/deadline checks run every 64 phase transforms as well as between
decoder packets. No whole-track samples or batch coefficient matrix are stored.

## Tests and numerical comparison

The local test set contains 39 passing tests: 14 unit tests, 13 core/CLI tests,
5 segment/MQA tests, 2 original parity/integrity tests and 5 transform integration
tests. Formatting and Clippy checks pass with warnings denied. New checks cover:

- MDCT agreement with the direct cosine definition.
- Bounded, ordered anchor positions and the 180-second cap.
- Cancellation inside the phase search.
- Flat spectra and finite zero mirror correlation; simultaneous rate hypotheses.
- Python resampling measurements and original hit modes on six fixtures.
- Python Vorbis scores/support on clean and encoded 44.1/48 kHz fixtures.
- Trim/gain, silent-channel isolation and anti-phase stereo.
- Tone, harmonic, quiet, silent, short and unsupported-rate controls.

The optimized Windows executable built successfully. The Android ARM64 library
also passed `cargo check --lib --no-default-features --target aarch64-linux-android
--locked`; this checks target compilation, not native linking or device execution.

`scripts/generate_transform_reference.py` requires the clean pinned Python commit
`c6ecce2296256b516709d87088896d1be913908c`, NumPy, SciPy and FFmpeg/libvorbis.
The checked-in FLACs contain generated signals only. Ordinary Rust tests consume
them and the saved oracle JSON without those external tools. PCM SHA-256 checks
verify the exact decoded samples independently of floating-point features.

For resampling, dB-band comparisons allow 0.05 dB absolute error, increasing to
0.2 dB below -120 dB relative to the spectral peak. Usable correlations allow
0.003 absolute error. The wider deep-floor tolerance reflects f32 FFT roundoff,
not statistical confidence. Investigation found a 0.0804 dB discrepancy in a
band near -135 dB; correlation of those nearly empty bins differed by about 0.024.
Such correlation was outside the original method's energy applicability gates
and is now explicitly unavailable rather than published as a meaningful number.
The reference oracle applies those same gates. No signal or gate was retuned to
force a hit, and measured low-level dB differences remain testable.

The constructed 48 kHz fixtures agree on a 44.1 kHz notch, wall and mirror
hypothesis respectively. Full-band 48/96 kHz controls and the separate 20.46 kHz
codec-wall control do not produce a resampling hit. These synthetic constructions
validate arithmetic and policy behavior, not provenance claims.

| Vorbis fixture | Python excess-zero score | Supporting / active probes |
| --- | ---: | ---: |
| Clean generated pink noise, 44.1 kHz | 0 | 0 / 12 |
| Same source through libvorbis q6, 44.1 kHz | 0.4478668 | 12 / 12 |
| Clean generated pink noise, 48 kHz | 0 | 0 / 12 |
| Same source through libvorbis q10, 48 kHz | 0.0592448 | 12 / 12 |

Rust matches these scores within 0.0001 and the support counts exactly. These are
feature values, not probabilities. Python input uses each original mono channel;
stereo Rust reports remain per native channel rather than selecting Python's
single best channel. This is a deliberate report-policy difference.

## Broader generated processing checks

`python scripts/check_transforms.py` generated ten stereo cases and checked every
decoded PCM hash against FFmpeg. All ten comparisons passed. The release search
produced these observations on both channels:

| Processing history | Vorbis grid | Resampling hypotheses |
| --- | --- | --- |
| Generated pink noise | No hit, score 0 | Unsupported at native 44.1 kHz |
| Generated transient noise | No hit, score 0 | Unsupported at native 44.1 kHz |
| libvorbis q4 / q6 / q8 / q10, then 24-bit FLAC | Hit at all four tested levels | Unsupported at 44.1 kHz |
| q10, trim 137 samples, gain 0.37, quantize to 24-bit FLAC | Hit | Unsupported at 44.1 kHz |
| Transient noise through libvorbis q8 | Hit | Unsupported at 44.1 kHz |
| Pink noise through FFmpeg SWR, 44.1 to 48 kHz | No hit | 44.1 kHz mirror |
| Pink noise through SoXR precision 28, 44.1 to 48 kHz | No hit | 44.1 kHz wall |

The six Vorbis-encoded cases have 12/12 supporting probes in each channel. The
q10 scores are about 0.0552/0.0578 and remain about 0.0547/0.0572 after the tested
trim/gain/quantization chain. The attacks exercise encoder block switching. This
does not establish invariance to arbitrary editing, dithering or resampling.
The SWR/SoXR controls use no lossy codec and retain an inconclusive ancestry verdict.

The eight version 0.2 controls were also rerun with version 0.3. All decoded PCM
hashes match FFmpeg. MP3 128/320, AAC 128, Opus 128, unencoded full-band noise,
mastering low-pass and a sine tone all have Vorbis score zero; silence abstains.
These narrow engineering controls are not a representative specificity estimate.

Exact commands, versions and processing histories are saved in
`corpus/local/results/synthetic-transforms-v3/manifest.json`. Per-file reports and
observations share that directory. Cross-codec controls are in
`corpus/local/results/other-codecs-v3`. No private songs are included in the
generated fixtures or public tests.

## Local music collection and resource checks

All 34 supplied FLACs were rerun without modification; their SHA-256 file hashes
are unchanged from version 0.2. The same 31 readable files completed full analysis
and matched FFmpeg's decoded PCM exactly. Thirty had successful embedded FLAC
checksum verification; `7 - Bend the Clock - Dream Theater.flac` had no available
verification result and still matched FFmpeg. Header lengths matched for all 31.
The same three previously documented corrupt inputs returned errors, including
independent FFmpeg errors, and the batch continued. The validator's exit code 1
reflects those known input failures.

Across the 62 successfully analyzed native channels:

- Vorbis: 40 applicable searches returned no hit; 22 were unsupported at their
  sample rates. Searches cover selected spans within the first 180 seconds only.
- Resampling: 24 applicable searches returned no hit; 38 were unsupported because
  no tested lower-rate Nyquist had enough space in the analyzed spectrum.
- All 31 overall ancestry verdicts remain inconclusive. These files have no
  independently established labels, so this is not a false-positive estimate or
  confirmation of authenticity.

Per successful file, observed process time was 1.50–13.06 seconds and peak working
set 20.02–43.27 MiB on this Windows host. Raw records are in
`corpus/local/results/transforms-v3/summary.json` and neighboring JSON reports.

The generated stereo 16-bit/48 kHz duration controls also retained exact FFmpeg
PCM agreement:

| Duration | Peak working set | Analysis process wall time |
| --- | ---: | ---: |
| 10 seconds | 19.87 MiB | 0.43 seconds |
| 120 seconds | 20.00 MiB | 1.43 seconds |
| 600 seconds | 20.01 MiB | 5.27 seconds |

The approximately flat footprint is consistent with bounded DSP buffers and the
fixed number of captured transform spans. Measurements sample the Windows
process's reported peak working set every 10 ms. These are neither Android
benchmarks nor controlled speed comparisons against previous versions; caching
and other host activity affect timings. Decoder/container allocations are not
subject to a hard process-wide cap. Raw data is under
`corpus/local/results/resources-v3`.

## Remaining work

AAC quantization-lattice analysis, auCDtect features, analog profiling, noise-floor
bit-depth analysis, loudness, calibrated aggregation and MQA confirmation remain.
Vorbis and resampling hits must be evaluated on independent source groups before
using them in a classification policy. Phone linking, on-device resource checks,
bindings and Android UI follow core validation.
