# Version 0.17 preceding-event band energy — 2026-10-02

Schema `0.17.0`, policy `observations-only-v17`. Descriptive context around the
existing envelope peaks. No codec pre-echo verdict, ancestry score or source claim.

## Reference assessment and intentional differences

The pinned `_psychoacoustic_artifacts` selects absolute -3 dB envelope peaks,
uses preceding 20–10 ms high-band windows and a median-power comparison, then
claims codec pre-echo. It includes edge-ineligible peaks in its denominator and
gates the entire pass on a codec/cutoff guess. Rust reuses its existing explicitly
bounded native high-pass envelope peak selection. It exposes actual eligible
context counts and removes the outer source gate and interpretation. A preceding
musical attack or earlier event can produce the same energy observation.

## Numerical and bounded contract

Each channel uses a causal fourth-order Butterworth high-pass at 10 kHz followed
by a fourth-order low-pass at 20 kHz (eight poles total), with zero initial states.
This differs from the reference's fourth-order bandpass design. Full requested
band support requires rate >40000 Hz; unsupported bands do not silently shrink.
Both passes use the same f64 filters; states below 1e-30 are flushed, as in the
existing envelope filter. The filter has frequency-dependent timing; 20 ms
startup exclusion does not prove that all transient filter effects have vanished.

First-pass baseline power is the median of squared filtered samples in
`[ceil(rate/50), min(analyzed_frames,180*rate))`. A fixed 963-counter histogram
uses exact zero, positive power through 1e-20, 960 0.25 dB bins across -200..40 dB
power, and overflow. Bounds enclose both middle order statistics (averaged for
even counts). Positive underflow has [0,1e-20] bounds. Overflow has no finite
upper bound and yields inconclusive threshold/fraction. Expose sample count,
scope and both bounds. The comparison threshold is strictly above
`max(1e-12,3*median_upper_power)`; no calibrated audibility or codec threshold.

Second pass retains at most `ceil(rate/4)` squared filtered samples (250 ms)
per channel, capped at 180 seconds. When the existing chronological envelope
selector accepts a peak center p, query the **complete** native-indexed window
`[p-ceil(rate/50), p-ceil(rate/100))`. Require the entire context after startup
and still in history. Long plateaus can be detected late; expired context is
explicitly inconclusive and excluded from the denominator. No edge padding,
audio compaction or fabricated zero context is used. Causal band filtering and
the existing envelope timing both affect the observation; p is not impulse onset.

Count every selected peak and every eligible/ineligible context. Fraction uses
eligible contexts only. No selected/eligible events gives a null fraction, not
a successful clear result. List at most 128 context records (including unavailable
ones), with complete aggregate counts and truncation. Selection/short-input
availability, full input/baseline intervals, warmup, lookback, history size and
per-event raw power/status/reason are explicit. Valid zero power is measured.

Persistent energy history is at most 768000 bytes/channel at 384 kHz; the
first-pass histogram is 7704 bytes/channel and is dropped before history capture.
No full waveform, extra FFT or decode pass is added. Query work is bounded by a
10 ms window per accepted event, and existing 10 ms peak spacing limits event
rate. Cancellation/deadlines remain packet-cooperative. Global decoder/container
allocations are separate acceptance work.

## Validation protocol

Unit control explicitly expires late context and checks eligible-only counts.
Generated integration tests cover isolated impulses, musical attacks, complete
band/rate applicability, zero/insufficient events, startup exclusions, prefixes,
all-event counts with 128 listed records, JSON and the actual 180-second cap.

Independent controls use SciPy-designed SOS filters, full-array squaring and
sorted power order statistics. Peak centers come from an independently computed
rectified/convolved envelope and chronological SciPy local maxima, with the
existing baseline histogram contract. Immutable expectations are stored before
Rust runs; all PCM must match FFmpeg s32le exactly. Counts/status/windows/nulls
agree exactly, power tolerance is `1e-12+1e-8*abs(expected)`. Sub-1e-20 median
bounds permit exact-zero/positive-underflow state-flush differences with exact
median containment; ordinary quantile bins must agree. Gain/antiphase, clean
impulses, prior energy, smooth attacks, noise, rates, silence, prefixes and event
listing caps are required. These arithmetic controls do not validate codec accuracy.

## Execution record

Implementation is saved. First Clippy run requested `?` syntax for the new
optional event return; this is corrected. A new let-chain was also replaced to
preserve the manifest's Rust 1.85 syntax floor (actual toolchain is 1.98.1).
All six new integration tests passed with LLVM linking. All 30 v0.18 units and
five original transient integration tests subsequently passed, including the
corrected helper below. Final v0.18 full regression passed **130 tests, zero
failed/ignored**. Source, tests, scripts and evidence
are local files only; no private audio, pinned reference, staged index or
publication ref was changed. No commit, push or external backup.
The first regression run passed all 29 units and six PCM integration tests,
but two existing transient tests rejected the new band's valid unsupported
status at low rates: their helper had assumed every transient-family member
shared the 1 kHz envelope feature's applicability. It now checks the original
`highpass_envelope_peaks` detector by ID. No filter or old peak-selection
arithmetic was changed to satisfy that assertion.

All **16 generated stereo cases / 32 native analyses** passed independent
SciPy filtering, dense sorted-power baselines and chronological peak selection,
with exact FFmpeg s32le PCM hashes. Maximum power error: **4.163337e-17**.
Counts/statuses, intervals, nulls and median containment agreed. Isolated impulses
had fraction 0; smooth musical attacks 0.95; the dense 200-peak train 0.995;
prior 15 kHz energy before impulses 2/3. These controls expose the ambiguity of
preceding energy and must not be turned into a codec verdict. Evidence is saved
locally in `corpus/local/generated/preceding-energy-v17/` and
`corpus/local/results/preceding-energy-v17/`. v0.18 regression will use a separate
output directory; original expectations/results remain preserved. That independent
regression passed all 16 cases / 32 native analyses with unchanged maximum error
and exact hashes: `corpus/local/results/preceding-energy-regression-v18/`.
