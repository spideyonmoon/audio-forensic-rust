# Version 0.15 high-band spectral lags — 2026-10-02

Schema `0.15.0`, policy `observations-only-v15`. Signed spectral measurements
only; ancestry stays `INCONCLUSIVE`, evidence index stays null.

## Pinned reference assessment

The unchanged baseline `c6ecce2296256b516709d87088896d1be913908c` computes a
centered log of the mean active magnitude spectrum inside `_psychoacoustic_artifacts`
and compares its lag products at multiples of rate/64 and neighbouring lags.
It converts selected peaks into an MP3 claim and score, behind an outer codec/
cutoff condition. Rust retains the numerical measurement with explicit scope
and applicability. No outer codec/cutoff gate, peak classification, MP3 verdict
or score is imported. This operation is neither frequency mirroring nor the
reference's separate pre-echo heuristic.

## Numerical and coverage contract

Reuse `SpectralStats`' existing f64 magnitude sums from all active native-channel
4096-sample symmetric-Hann f32 FFT windows, with 2048-sample hops and the global
relative activity gate. No additional running accumulator, FFT or decoding pass
is added. The strict frame boundary, shared prefix and packet-level cancellation
remain unchanged. The report interval encloses all evaluated STFT windows;
total/active counts describe contributing windows without concatenating gaps.

Select actual bin centers in **[16000, 20000) Hz**, using integer rational
membership. The full nominal upper edge must lie strictly below Nyquist;
rates <=40000 Hz are unsupported, rather than substituting a clipped band.
At least 81 selected bins are required. Excessively high rates with too few bins
are explicitly unsupported even though their Nyquist is adequate. Actual lower/
upper bin centers and counts remain available for a complete but under-resolved band.

Define coherent-gain amplitudes `A[k] = 2 * mean_active_magnitude[k] / sum(Hann)`
and reference amplitude `max(A)` over the entire mean spectrum, including DC/
Nyquist. This is not integrated band RMS or power spectral density. At least four
active windows are required, and **every selected bin** must exceed both `1e-6`
amplitude and `1e-4` of the reference. These conservative numerical applicability
floors abstain on missing/weak bins; they are not calibrated forensic thresholds.
The minimum amplitude and eligible count are exposed even when the gate fails.

For eligible bands, compute `d[k] = 20*log10(A[k]/reference)`, center by its
full-band mean, and let `D = sum(d_centered^2)`. Require population standard
deviation strictly above 0.1 dB. No additive log/denominator epsilon or fabricated
zero coefficient is used. Flat spectra are inconclusive. The standard deviation
is reported only after the frame/geometry/every-bin energy gates pass.

For lag L, report `sum(d[k]*d[k+L], k=0..B-L) / D` with the same full-band
centering and denominator for each lag. This is **not overlap-centered Pearson
correlation**; finite overlap attenuates even a perfectly periodic spectrum.
Keep its sign. Three targets use 64, 128 and 192 bins (exactly rate/64 multiples
for the fixed FFT), with signed neighbours at L-3/L+3. Requested/actual Hz,
pair count, status and nullability are explicit for every target and neighbour.
At least sixteen pairs are required; individual long lags can be unsupported
while a shorter lag is measured. Aggregate status is measured if the band gates
pass (81 bins guarantee at least seventeen pairs at the first target).

Finalization uses at most 410 centered f64 values, about 3.21 KiB of temporary
payload, bounded by fixed geometry. Persistent storage is the existing spectral
sum plus the fixed report. No duration-dependent waveform or spectrum history
is retained. EQ, periodic filters, tones and noise affect these measurements;
they do not identify a codec, filterbank residue, aliasing or authenticity.

## Validation protocol

Unit controls check exact centered cosine products, signed neighbours, flat/
missing-bin/short-spectrum abstention. Generated float-WAV integration controls
exercise periodic and smooth EQ spectra, native gain/antiphase and silent channels,
rate/resolution and per-lag geometry, weak/flat/tone spectra, strict prefixes,
minimum frames, inactive gaps and JSON round trips.

`scripts/check_spectral_lags.py` independently stores dense NumPy FFT windows,
means their active magnitudes, applies an integer dense band mask, and computes
centered full-array products using NumPy dot products. Expected JSON is saved
immutably before Rust runs. Full/prefix FFmpeg s32le PCM hashes, counts, statuses,
geometry, intervals and nullability must agree exactly. Predeclared tolerances:
amplitudes `1e-9 + 1e-4*abs(expected)`, standard deviation 0.005 dB, coefficients
0.0002, other floating geometry 1e-9. Generated ripple periods, smooth EQ, white
noise, notch/lowpass, tones, silence, flat impulses, gain, gaps, rate limits and
short/prefix controls are covered. These are arithmetic controls, not source
ground truth or codec accuracy estimates. After the initial precision finding
below, separate immutable `*.fft_precision.expected.json` files supplement the
unchanged f64 expectations with an independent SciPy f32 FFT. Only diagnostic
minimum amplitudes below the absolute 1e-6 applicability floor use that f32
comparison, with the original amplitude tolerance. Coefficients, standard
deviations and eligibility continue to use the dense f64 oracle.

## Execution record

All **27 generated stereo cases / 54 native-channel analyses** passed independent
arithmetic and exact FFmpeg s32le PCM hashes, including prefixes. Maximum errors:
1.726097e-8 coherent amplitude, 2.877911e-7 dB standard deviation and
1.288664e-7 coefficient. Status, eligibility, geometry, pair/frame counts,
intervals and nullability agreed exactly.

The initial run failed its amplitude tolerance on the minimum rejected 18 kHz
tone bin: Rust 1.321874701e-9 versus NumPy f64 FFT 3.628422602e-9. This is below
the 1e-6 applicability floor; both abstain with identical eligibility/counts.
An independent SciPy f32 FFT gives 1.321874590e-9, agreeing with Rust within
1.11e-16. Its full mean-spectrum difference from the f64 FFT is up to 1.774455e-8.
This isolates FFT rounding in deep leakage rather than changing a measured lag.
The original expected JSON is preserved. Separate precision expectations were
saved before rerunning Rust; neither production arithmetic nor the coefficient/
standard-deviation tolerances were altered. The initial failure is retained here.

| Generated control, first native channel | Three target coefficients |
| --- | --- |
| 64-bin ripple at 64 kHz | +0.750000, +0.500000, +0.250000 |
| 93-bin ripple | -0.277791, -0.361118, +0.213117 |
| Smooth -24 dB spectral EQ | +0.281241, -0.250010, -0.406261 |
| White noise | +0.018670, +0.079678, -0.038718 |
| Flat impulses, tones, silence, weak band | Inconclusive, null coefficients |
| 192 kHz ripple | First target measured; longer targets unsupported |
| 384 kHz ripple | Entire band under-resolved, unsupported |

The full release suite passed with LLVM linking: **106 tests**, zero failed or
ignored (99 preceding tests plus two new units and five new integration tests).
Final formatting, all-target
Clippy with warnings denied, Android ARM64 no-CLI compilation, Python syntax,
CLI version/0.8-second generated-prefix text and final-scope JSON smokes, and
Git whitespace passed.
The final ordinary release CLI build also passed with the same LLVM flags after
the full suite; its regular dependency feature set was rebuilt successfully.

Generated 10/120/600-second resource controls matched exact PCM and used
**20.79/24.48/26.42 MiB** peak Windows working set in
**0.8299/3.2695/15.3972 s**. Compilation overlapped; these are not controlled
timing comparisons or Android benchmarks.

Initial Windows release unit/integration/CLI links failed with GNU ld exit 204,
including a single-job retry; no tests ran on those attempts. A simple C program
linked successfully with the same GCC. The bundled LLVM linker, selected through
GCC's `-fuse-ld=lld` and `-B` flags, successfully linked the CLI/tests. The cause
of GNU ld's failure remains undetermined. An initial alternative-linker command
failed due to PowerShell argument forwarding; corrected array arguments passed.
The project release optimization/LTO profile was unchanged. `HANDOFF.md` records
the process-local flags for reproducing the working build.

No full debug suite, publication-only build, codec/resampler matrix, private
collection pass, grouped source evaluation, remote CI or Android linking/device
test was run. No certified forensic accuracy or meter conformance is established.

No private recording, pinned Python file, staged index or publication ref was
changed. No commit, push or external backup was made. Evidence paths:
`corpus/local/generated/spectral-lags-v15/`,
`corpus/local/results/spectral-lags-v15/`, `tests/spectral_lags.rs` and
`scripts/check_spectral_lags.py`; duration evidence is in
`corpus/local/results/resources-v15/`. Tests/scripts/evidence are excluded from
source-only publication; the new source is currently an untracked local file.
