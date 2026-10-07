# Version 0.13 programme loudness — 2026-10-02

Schema `0.13.0`, policy `observations-only-v13`. This milestone measures programme
listening level. Ancestry remains `INCONCLUSIVE` and evidence index remains null.

## Reference selection

[ITU-R BS.1770-5, Annex 1](https://www.itu.int/dms_pubrec/itu-r/rec/bs/R-REC-BS.1770-5-202311-I!!PDF-E.pdf)
defines K-weighted channel powers, unit weights for mono/front stereo channels,
400 ms gating blocks with 75% overlap, an absolute gate of -70 LKFS and a second
gate 10 LU below the absolute-gated mean. Both comparisons are strict. Incomplete
blocks are discarded. The 48 kHz filter tables supply the independent coefficient
test. LUFS and LKFS express the same numerical programme level here.

[EBU Tech 3341](https://tech.ebu.ch/docs/tech/tech3341.pdf) supplies the ungated
400 ms momentary and 3 s short-term window definitions. Our maxima are sampled
every 100 ms, after a complete window. They can miss an off-grid maximum and are
not advertised as certified EBU file-meter maxima or full meter conformance.

The general-rate coefficient parameterization follows the filter design used by
[libebur128](https://github.com/jiixyj/libebur128/blob/master/ebur128/ebur128.c):
shelf frequency 1681.974450955533 Hz, gain 3.999843853973347 dB,
Q 0.7071752369554196, intermediate-gain exponent 0.4996667741545416;
high-pass frequency 38.13547087602444 Hz and Q 0.5003270373238773.
Rust uses two separate f64 biquads with zero initial state.

The pinned Python `extract_loudness` delegates to FFmpeg, resamples loudness to
48 kHz and ignores the shared prefix. It also calls astats/drmeter and derives
presentation labels. Rust measures native-rate PCM within the shared interval.
There is no runtime FFmpeg/Python dependency, resampling, downmix, dual-mono
compensation, quality grade, platform normalization target or source inference.
True peak, loudness range and DR-meter behavior are separate pending work.

## Streaming and report contract

The decoder supplies interleaved native-channel samples. Each channel has its
own filter state. Squared outputs are summed, so phase-inverted stereo cannot
cancel before measurement. No channel-count averaging is applied.

Thirty 100 ms power sums form a fixed ring. Re-summing four or thirty positive
entries avoids drift from subtracting an old loud window after a quiet passage.
Pass one computes an online mean of eligible 400 ms powers. Pass two restarts
the filters at frame zero and applies its fixed relative threshold as well as
the absolute gate. This gives exact, unquantized gating without a duration-sized
block list or a loudness histogram. Both existing PCM hashes still must match.
The first meter plus the second use only fixed small state and at most four
biquad pairs total; no extra decode pass, FFT or waveform buffer is introduced.
Packet-level cancellation/deadline checks remain unchanged.

Supported loudness rates are the core's 8–384 kHz rates divisible by ten. Other
rates (including 11025 Hz) explicitly return `unsupported` for loudness, preserving
all other applicable analysis. This avoids an undocumented rounded 100 ms grid.
Implementing fractional-hop coverage is future work, with its own oracle.

`analyzed_frames` is the shared decoded length. `interval` is the union of complete
400 ms windows, or null if none. `trailing_frames` counts frames outside that
union (the whole input when too short or unsupported). Block/hop sizes, absolute
and relative gate counts and threshold are reported. `short_term_windows` gives
independent 3 s applicability. No padding or filter tail is appended. Silence,
no absolute-gated blocks and insufficient coverage give null integrated loudness;
ungated maxima may still be available below the gate. Exactly zero or underflowed
window powers give null maxima, avoiding JSON infinities.

The `programme_loudness` observation has family `listening_level` and no single
channel index. Its status follows integrated-loudness availability. The typed
report preserves separate maxima and counts. It never produces an ancestry hit.

## Validation protocol

Rust regressions cover published coefficients, the reference-frequency mono
tone, same/opposite-phase stereo, silent right channel, linear gain, both gates,
silence, tiny floats, prefix isolation, exact 400 ms/3 s boundaries and tails,
sample-rate support, JSON and the conservative reporting policy.

`scripts/check_loudness.py` generates only 24-bit PCM controls and saves immutable
expectations before calling Rust. SciPy filters full native channels, then NumPy
directly averages stored 400 ms/3 s slices and explicitly selects gates. At 48 kHz
it uses the published coefficient tables rather than the production design
formula. Other rates use the common frequency-response design but independent
window aggregation. FFmpeg native-rate `ebur128` supplies a second implementation;
FFmpeg s32le decoding must match the Rust PCM SHA-256 exactly, including prefixes.

Predeclared tolerances: dense oracle **1e-6 LU**; FFmpeg integrated **0.11 LU**
(histogram/printed precision), FFmpeg M/S metadata **0.002 LU**. Counts, status,
intervals and nullability must match the dense oracle exactly. Pre-window
zero-padded FFmpeg displays are excluded. Silence's FFmpeg sentinel does not
become a Rust measurement. These tolerances establish arithmetic agreement,
not forensic accuracy or standards certification.

## Execution record

All **27 generated analysis cases** passed the independent dense oracle and exact
FFmpeg s32le hashes. They cover 21 distinct files and six additional prefix runs.
All applicable readings from **22 cases** also passed FFmpeg loudness comparisons.
Maximum errors: **9.024e-13 LU** versus SciPy/NumPy, **0.008846 LU** for FFmpeg
integrated, **0.000465 LU** for momentary and **0.000481 LU** for short-term maxima.
Counts, status, intervals and nullability agreed exactly with the dense oracle.
NumPy 2.3.5 and SciPy 1.17.1 were used. No oracle regeneration or tolerance
adjustment was needed; no production/numerical-test failure occurred.

| Generated control | Observed result |
| --- | --- |
| 997 Hz, 0.1 peak, mono at 48 kHz | -23.010270 LUFS integrated |
| Same tone in both channels, same or opposite phase | -19.999970 LUFS; no antiphase cancellation |
| Four seconds loud then four seconds 40 dB quieter | 77 absolute-gated blocks, 40 relative-gated; -23.176277 LUFS |
| Silence / sub-gate tone | Null integrated level; below-gate tone retains ungated maxima |
| DC step at frame zero | Finite startup transient, one gated block; no invented steady-state noise floor |
| Noise with quiet sections | 50 absolute-gated / 33 relative-gated blocks |
| Three-second prefix before a loud tail | -43.010265 LUFS, later loud content excluded |
| 8–384 kHz and unsupported 11025/44101 Hz | Native-rate agreement or explicit unsupported status |
| One sample and 400 ms/3 s boundary prefixes | Exact coverage, complete-window counts and nullability |

Published-coefficient unit test plus all six new integration tests passed in
release mode. Formatting, all-target Clippy with warnings denied, release CLI
build/version/text checks, Python syntax and Android ARM64 no-CLI library
compilation passed. The full release regression run passed **all 92 tests**
(85 prior tests plus seven loudness tests), with zero failures or ignored tests.
No validation jobs or partial implementations remain running.
The first text-smoke command named a nonexistent fixture and correctly returned
a structured failure; the rerun on an existing generated resource WAV passed.

Unchanged generated 10/120/600-second stereo inputs matched PCM and used
**20.62/23.61/26.14 MiB** peak Windows working set in **2.3692/9.7606/35.3437 s**.
Compilation and independent checks overlapped this run. These are resource
observations, not controlled speed comparisons or phone performance claims.

No full debug suite, separate publication-only build, new encoder/resampler
matrix, private collection pass, grouped accuracy evaluation, remote CI,
Android linking/device test or certified EBU/ITU meter test-file suite was run.
The pinned Python checkout remains clean at its baseline. Existing staged work
was preserved. No commit, push or external backup was made.

Local-only evidence: `tests/loudness.rs`, `scripts/check_loudness.py`,
`corpus/local/generated/loudness-v13/`, `corpus/local/results/loudness-v13/`
and `corpus/local/results/resources-v13/`. These are local files, excluded from
source-only publication. No private recording was changed or uploaded.
