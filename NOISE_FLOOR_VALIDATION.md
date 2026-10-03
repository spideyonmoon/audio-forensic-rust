# Version 0.12 quiet-block profile — 2026-10-02

Schema `0.12.0`, policy `observations-only-v12`. Ancestry stays `INCONCLUSIVE`
and evidence index null. The report measures signal levels and spectral color;
it does not isolate physical noise, identify dither or infer source precision.

## Reference assessment and deliberate differences

The pinned Python baseline remains `c6ecce2296256b516709d87088896d1be913908c`.
`_noise_floor_profile` averages stereo and removes zero RMS entries before
sorting, then applies those compacted positions to the original block array.
The regression with alternating zero/nonzero blocks and decreasing nonzero
amplitudes selects original blocks 45, 47 and 49 in Rust. Python's compacted
positions point to different, sometimes silent blocks. Antiphase stereo is
measured separately in Rust instead of cancelling before measurement.

Python's independent 30-second (often midpoint) decode ignores the common
prefix limit. Rust uses the first 300 complete blocks within the shared prefix.
No `_bit_depth_verdict`, flatness label, effective-depth conversion, authenticity
claim, source label or score is ported. Exact exercised integer bits remain a
separate full-coverage measurement. Numerical agreement does not validate the
reference's forensic claims.

## Numerical contract

Each native channel uses nonoverlapping `floor(sample_rate / 10)`-sample blocks
from frame zero. At supported rates this is 800–38400 samples. At rates not
divisible by ten, blocks are slightly shorter than 100 ms and 300 blocks span
slightly less than 30 seconds. Endpoints are half-open. Exact complete EOF blocks
qualify without STFT lookahead; a partial final block does not contribute.

The first decoding pass stores at most 300 RMS values, using a scaled f64 sum
of squares to avoid squaring underflow on tiny float PCM. Count exact all-zero
blocks separately. Nonzero blocks whose RMS still rounds to zero are reported
as `rms_underflow_blocks`, excluded from percentiles and selection. Thus
`complete_blocks = zero_blocks + rms_underflow_blocks + nonzero_blocks`.
`nonzero_blocks` means positive representable RMS, not an activity threshold.

Require at least 20 positive-RMS blocks. Otherwise level status is inconclusive
and percentiles/selection are unavailable. Compute linear-interpolated 1.5th
and 99th percentiles of nonzero RMS, then `20*log10(RMS)` without epsilon.
Select `max(3, floor(nonzero_blocks / 10))` quietest blocks (at most 30), breaking
exact RMS ties by original index. Report selected original indices and RMS in
chronological order. No selected gaps are concatenated. Silence exclusion is
explicit: these are conditional nonzero statistics, not whole-prefix percentiles.
Floating-point reduction differences can reorder nearly tied unequal blocks;
reported original indices make the actual selection inspectable.

The second pass computes an f64 symmetric-Hann FFT of each selected block.
For non-DC/non-Nyquist bins the power is `2*|X[k]|^2/(N*sum(window^2))`.
Average linear power over the selected blocks and each band's bins. This is
mean power **per bin**, not integrated band power or power spectral density.
The bands have strict frequency edges: `(150, 2000)` Hz and
`(0.33*rate, 0.45*rate)` Hz. Requested and actual geometry and bin counts are
reported. Bin membership is evaluated with exact integer ratios: low-band
`150*N < k*rate < 2000*N`, high-band `33*N < 100*k < 45*N`.
This excludes exact boundary bins independently of rounded Hz presentation.
At least two bins must be present. No detrending is performed.

Color is `10*log10(high_mean_bin_power / low_mean_bin_power)`. It is measured
only if both means exceed `max(1e-24, 1e-12*selected_mean_total_power)`.
Total power includes DC and Nyquist with their correct one-sided weights.
These are numerical leakage guards, not calibrated noise/dither thresholds.
Available band powers remain reported when the ratio abstains; unavailable
power is null. Level status and color status are separate. Music, DC, fades,
gain, filters and quantization affect the measurements. Tiny numerical band
powers must not be interpreted as resolved physical noise.

`interval` covers complete surveyed blocks. `inspected_frames` also includes a
discarded partial tail; `trailing_frames` gives its length. `coverage_capped`
means shared analyzed coverage extended past the detector's 300-block limit.
The shared report independently exposes any requested prefix restriction.
Selected block `i` covers `[i*block_frames, (i+1)*block_frames)`.

## Resource and cancellation contract

No additional decode pass, full-prefix waveform or full spectrogram is stored.
The survey retains 300 f64 values/channel. Spectral collection uses one
block-sized complex f64 buffer, a block-sized f64 window, FFT plan/scratch and
at most thirty selected descriptors. At 384 kHz buffer plus window occupy
921600 bytes/channel before plan/scratch/allocator overhead. Unusual FFT sizes
may need larger plan/scratch allocations; all sizes are bounded by sample rate,
never input duration. After selection ends, sample callbacks do no further FFTs.
Packet-level cancellation/deadline and exact two-pass PCM checks still apply;
an individual FFT/planner call or blocking decoder/I/O call is not interruptible.

## Validation protocol and execution

Local Rust integration tests cover original indices after silence, independent
antiphase/gain, analytic two-tone band powers, 19/20-block applicability,
silence, exact ties, prefix/tail/cap coverage, 8–384 kHz, odd sample rates,
tiny float PCM, JSON and unchanged conservative policy.

`scripts/check_noise_floor.py` uses dense NumPy block arrays and batched f64
FFTs, with time-domain Hann-weighted total power for the energy gate. It saves
expected values before running Rust and refuses to overwrite changed oracles.
Generated 24-bit stereo controls include silence gaps, a high odd FFT length,
tones, gain/antiphase, white/colored signals, silence, missing bands, cap/tie
and shared-prefix boundaries. Exact FFmpeg s32le SHA-256 checks decoding.

Predeclared numerical tolerances: RMS/geometry absolute `1e-14 + 1e-10*abs(x)`;
normalized per-bin power absolute `1e-25 + 1e-8*abs(x)`; dB absolute `1e-6`.
Counts, indices, statuses, eligibility and intervals must match exactly.
These tolerances address arithmetic only, not source-classification accuracy.

All **17 generated analysis cases / 34 channel analyses** (14 distinct 24-bit
stereo files, including three additional prefix runs) passed the corrected
independent oracle and exact FFmpeg PCM hashes. NumPy version: 2.3.5. Maximum
absolute errors were 2.950e-17 RMS, 1.655e-23 normalized per-bin power and
4.264e-14 dB. Counts, indices, intervals, statuses and energy gates matched.
Geometry differed only by at most 2.274e-13 Hz, within the declared tolerance.

| Control | Observation |
| --- | --- |
| Silence-gap native antiphase, 8�384 kHz including 383990 Hz | Correct original indices, independent native powers and exact PCM |
| Alternating zero/descending DC blocks | Selected 45, 47, 49; 25 zero blocks; color abstains |
| All-zero / 19 positive-RMS blocks | Inconclusive levels and empty selection |
| Constant amplitude / late quieter content | Exact ties select earliest blocks; 300-block cap excludes later content |
| Prefixes 0.01, 1.99, 2.000125 seconds at 8 kHz | Complete-block count, discarded tail and 20-block applicability match |
| Two musical tones | Color about -10.969 dB, without a noise-origin claim |
| Low tone at two gains | Loud channel's high band abstains; quantized weak channel measures about -27.222 dB |
| Filtered versus white signal | About -61.958 versus +0.093 dB; filtering changes color without identifying source depth |

Formatting, all-target Clippy with warnings denied, final release CLI build,
version/text smoke checks, and no-CLI Android ARM64 library compilation passed
on the final rational-boundary implementation. Python script syntax and Git
whitespace checks passed. The final full release suite passed **all 85 tests**:
21 unit, 4 AAC, 13 core/CLI, 5 segment/MQA, 4 envelope, 7 noise, 7 quiet-block,
2 parity/integrity, 4 roll-off, 5 sparsity, 3 structure, 5 transform and 5 transient.
There were no failures or ignored tests. No validation jobs remain running.

Final generated 10/120/600-second resource inputs matched PCM and used
**20.60/23.84/26.09 MiB** peak Windows working set in **1.166/3.6962/14.177 s**.
These overlapped compilation and are bounded-duration observations, not
controlled timing comparisons or Android performance measurements. Results:
`corpus/local/results/resources-v12-final/`. No full debug suite, separate
publication-only build, new encoder/resampler matrix, private collection run,
remote CI, Android linking/device tests or grouped accuracy evaluation was run.
No v0.12 commit or push was made; these results describe the local working tree.

The pre-boundary-correction full release run passed all 84 tests. The final
85-test run includes the additional exact-boundary regression. An earlier build was invalidated by a concurrent model-field
addition and failed to compile its tests; the subsequent 84-test run used a
consistent source tree. Initial focused run: four passed, one failed
because a test's one-sample duration rounded below one sample at an odd rate.
The test now requests 1.5 samples beyond each boundary and asserts the decoder's
documented floor convention. No production prefix logic or historical oracle
was changed to hide the failure. Final tiny-float checks were added afterward.

The first independent comparison passed at 8 kHz, then exposed a boundary
mismatch at 8001 Hz: NumPy's reciprocal-step frequency calculation produced
2640.3300000000004 Hz for bin 264, just above its rounded 33% edge. Mathematically
`264/800 = 33/100`; that bin must be excluded. Rust's floating comparisons could
also vary at exact edges for other rates. Both now use exact rational membership,
with explicit Rust regressions at 8001, 44101 and 383990 Hz. Original
`*.expected.json` files are preserved unchanged; corrected expectations are
separately named `*.expected-rational-bounds.json`. No tolerances were widened.
The initial duration run is preserved under `resources-v12`; final resource
checks after the arithmetic correction use `resources-v12-final`.

Direct execution of the pinned reference function on generated antiphase PCM
returned null. Its compacted sorting positions were `[24, 23, 22, 21, 20]` on
the silence-gap regression. The assessment is saved in `reference-assessment.json`;
the pinned checkout was not edited.

Local-only evidence remains in `corpus/local/generated/noise-floor-v12/`,
`corpus/local/results/noise-floor-v12/`, `tests/noise_floor.rs` and
`scripts/check_noise_floor.py`. These are excluded from source-only publication.
