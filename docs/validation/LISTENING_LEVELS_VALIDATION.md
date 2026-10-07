# Version 0.14 true-peak estimates and loudness range — 2026-10-02

Schema `0.14.0`, policy `observations-only-v14`. These are listening-level
measurements. Ancestry remains `INCONCLUSIVE`; evidence index remains null.

## Numerical references

True peak uses the 48 coefficients in
[ITU-R BS.1770-5 Annex 2](https://www.itu.int/dms_pubrec/itu-r/rec/bs/R-REC-BS.1770-5-202311-I!!PDF-E.pdf),
as four FIR phases with twelve taps each. Floating arithmetic needs no preliminary
attenuation. This estimates the maximum reconstructed waveform amplitude;
sample peaks alone can miss peaks between sampling instants.

[EBU Tech 3342](https://tech.ebu.ch/docs/tech/tech3342.pdf) supplies the loudness-range
reference: 3 s K-weighted power windows, at least 10 updates/s, -70 LUFS absolute
and -20 LU relative gates, and the difference of the 95th and 10th percentiles.
The reference MATLAB uses inclusive gates and nearest order statistics. Its
file-meter recommendation appends at least 1.5 s of silence. We deliberately
retain only complete windows inside the shared interval, without that extension,
and expose this difference in the report. Full EBU meter conformance is not claimed.

Generated phase-sensitive tones follow the numerical descriptions in
[EBU Tech 3341](https://tech.ebu.ch/docs/tech/tech3341.pdf). The standard's permitted
peak error for these controls is -0.4/+0.2 dB. This small generated subset does
not substitute for its full test distribution or prove general meter accuracy.

## True-peak implementation

Each native channel maintains twelve f64 input samples. For incoming frame n,
phase p produces `sum(h[4*k+p] * x[n-k], k=0..11)`. The table is used verbatim,
without DC-gain normalization or a pre-filter. The flattened 48-tap impulse
response is symmetric with a delay of 23.5 oversampled samples. Input is treated
as zero before frame zero and after the analyzed prefix. Eleven zero frames at
finish capture the entire FIR tail, including a last-frame impulse. Padding does
not increase reported real-audio coverage. Only visited samples inside the shared
prefix contribute; a decoder may internally read a whole packet at a boundary.

`interpolated_peak` is the maximum of all four full convolution responses.
`estimated_peak` is `max(sample_peak, interpolated_peak)`, preserving the exact
sample-peak lower bound even when the interpolator attenuates a sample-aligned
peak. `estimated_peak_dbtp` is `20*log10(estimated_peak)`, or null for exact zero.
Silence remains measured with zero linear amplitude. No square operation is
used, so very small representable float peaks are not lost through squaring.

The fixed filter is applied at all core-supported rates, including odd rates,
with its frequency response relative to the input Nyquist. Its oversampling
factor and actual output rate are explicit. At rates below 48 kHz, the output
rate is below Annex 2's recommended 192 kHz. This is an explicitly described
estimate, not an assertion of certified conformance. Fourfold sampling and finite
FIR bandwidth can under-read continuous peaks, especially near Nyquist. Abrupt
prefix boundaries/zero extension may create ringing. A measured value over
0 dBTP is not proof of clipping, audible distortion, lossy history or poor mastering.

Cost is 48 multiply-add terms per native sample, in the existing first pass.
Storage is 96 bytes of channel history plus counters/maxima; coefficients are
fixed. No waveform, resampling output buffer or extra decode pass is retained.
Packet-level cancellation remains in force; finalization adds only eleven frames.

## Loudness-range implementation

The existing loudness meter supplies native-channel K-weighted 3 s powers every
100 ms. Pass one computes the absolute-gated power mean. Pass two applies the
fixed relative gate to unquantized powers, using `>=` for both gates. This is
separate from integrated loudness's strict gates and -10 LU relative offset.

The second pass retains 12000 u64 histogram counts over `[-70, 50)` LUFS at
0.01 LU/bin (96000 bytes), independently of input duration. Any overflow is
counted and causes an inconclusive range rather than a clamped successful result.
The first pass retains no histogram. Unsupported rates follow the existing
loudness restriction (sample rate divisible by ten).

For percentile p, use zero-based rank `round((count-1)*p/100)`, half upward,
computed with integer arithmetic. Report the selected bin centers for p10/p95.
Each differs from its unquantized order statistic by at most 0.005 LU, apart
from floating arithmetic. The range error is at most 0.01 LU. Report both the
center estimate and `[max(0, estimate-0.01), estimate+0.01]` bounds. Those bounds
cover histogram quantization only, not window-grid, gate-boundary, acoustic or
subjective uncertainty. Two gated windows are required; one cannot describe a
distribution's spread. A measured zero is valid for constant-level programme.

The range interval is the union of complete 3 s windows, or null when none.
Absolute/relative gated counts and relative threshold are explicit. No tail
silence is synthesized and no partial windows are padded. Prefixes therefore
describe the prefix's level variation, not the whole track. LRA is not a DR score,
crest factor, a compressor detector or a mastering grade.

## Validation protocol

`tests/listening_levels.rs` uses generated float WAVs for phase-sensitive peaks,
native-channel gain/antiphase, exact silence and tiny floats, one/last-sample
impulses, odd/high/low rates, prefix isolation and FIR-tail flushing. Range tests
exercise generated 20-second level steps, minimum windows, prefix boundaries,
silence, unsupported rates and JSON. Unit tests check filter symmetry, a single
impulse, inclusive gate equality and histogram overflow abstention.

`scripts/check_listening_levels.py` independently performs full NumPy convolution
for each FIR phase. Range uses published 48 kHz coefficients, SciPy full-array
filtering, direct slice means, explicit gate selection and sorted unquantized
order statistics. Immutable expected JSON is saved before Rust runs. PCM must
match FFmpeg s32le SHA-256 exactly for full inputs and prefixes. Generated standard
peak tones and long level-step controls are also checked with FFmpeg ebur128.

Predeclared tolerances: FIR amplitude `1e-13 + 1e-11*abs(expected)`, peak dB
`1e-8`, gate level `1e-6 LU`, percentile `0.00500001 LU`, range `0.01000001 LU`.
Dense expected range must lie inside the reported bounds (1e-10 arithmetic
allowance). Counts/status/coverage/nullability match exactly. Standard-described
peak tones use -0.4/+0.2 dB and LRA level steps use +/-1 LU, independently for
Rust and FFmpeg. Different FIRs and LRA window/tail conventions do not generally
give identical arbitrary-signal results; they are not forced into tight parity.

## Execution record

All **33 generated analysis cases / 53 native-channel peak analyses** passed
the independent oracles and exact FFmpeg s32le hashes. The first run covered
31 cases; a targeted follow-up covered two changing-level noise/gain cases to
exercise nontrivial order statistics. Summaries are `summary.json` and
`summary-varying_noise.json`; expected files are immutable. No expected value or
tolerance was altered to obtain a pass.

Maximum differences: **0** in FIR amplitudes for these dyadic 24-bit controls,
**0.001677 LU** in range, **0.001716 LU** in a percentile, and **1.208e-13 LU**
in range gate level. All dense range expectations lay inside the reported bounds.
Counts, intervals, applicability and nullability matched exactly. Five generated
phase-sensitive peak cases and five long level-step/gain cases met the specified
standard-described tolerances in both Rust and FFmpeg.

| Generated control | Rust observation |
| --- | --- |
| fs/4, 45 degrees, 0.5 amplitude | Sample peak 0.3535534; estimated -5.975908 dBTP |
| fs/4, 45 degrees, 1.41 amplitude | Sample peak 0.9970206; estimated +3.029074 dBTP |
| One/last-sample 0.5 impulse | FIR peak 0.486083984375; estimate retains sample peak 0.5 |
| Long 10/5/20/15 LU step controls | 10/5/20/15 LU reported range, with +/-0.01 LU bounds |
| Changing-level noise and half-gain copy | Both 12.13 LU range, bounds [12.12,12.14], 80 gated windows |
| Silence / one gated short-term window | Range inconclusive; silence has zero measured linear peak and null dBTP |
| Prefix ending mid-signal | Exact prefix PCM, complete range windows, own zero-extended FIR tail |

All **27 v0.13 loudness cases** were rerun against v0.14, using
`--output corpus/local/results/loudness-regression-v14` to preserve v0.13 evidence.
They passed with exact PCM and the same maximum dense-oracle error, 9.024e-13 LU;
their 22 applicable FFmpeg loudness comparisons passed as well.

All 24 unit tests and the five new integration tests passed in focused release
runs. Formatting, all-target Clippy with warnings denied, release build/version,
generated-prefix text smoke, Python syntax and Android ARM64 no-CLI compilation
passed. On resume, no preceding jobs remained running; the final full release
regression suite was run to completion: **99 passed**, zero failed/ignored.

The unchanged 10/120/600-second resource inputs matched PCM and used
**20.71/24.04/25.96 MiB** peak Windows working set in
**1.6916/9.3978/34.7751 s**. Compilation and independent checks overlapped, so these
are not controlled speed comparisons or Android performance measurements.

No full debug suite, publication-only copy build, new codec/resampler matrix,
private collection pass, grouped accuracy evaluation, certified meter test-file
suite, remote CI or Android linking/device test was run. No commit, push or
external backup was made; existing staged work and private recordings were
preserved. These results describe local files only.

Generated evidence stays in `corpus/local/generated/listening-levels-v14/`,
`corpus/local/results/listening-levels-v14/`,
`corpus/local/results/loudness-regression-v14/` and
`corpus/local/results/resources-v14/`. Test/script sources are
`tests/listening_levels.rs` and `scripts/check_listening_levels.py`; these and
the evidence are excluded from source-only publication. No private audio is used.
