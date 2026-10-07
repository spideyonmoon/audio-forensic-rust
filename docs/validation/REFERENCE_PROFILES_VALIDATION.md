# P04a/P04b reference-profile validation — 2026-10-05

P04a/P04b complete in engine 0.27.0. Pinned Python remains at c6ecce2296256b516709d87088896d1be913908c. New `tests/fixtures/reference_profiles.json` was generated before Rust implementation; the generator uses exclusive creation and read-only `--check`. Existing oracles are unchanged.

## Domains and deviations

Reference-input schema/method advances to v2; native measurement schema remains 0.18.0. P04c fields retain their numerical methods. New spectral calculations use active f32 mid frames except side comparison, which uses aligned unmasked stride-four mid/side frames. Banding uses the global selected-region maximum and population temporal standard deviation. LPF/DSD are spectral observations, not container identity. Ordered resampling selects the first eligible source-rate candidate and first mode (notch, wall, mirror) from the explicit reference-mid collector; native eight-active-frame and quiet-band gates remain (D03). Header comparisons require full EOF for duration; total-file bitrate includes metadata and the extension gate is audit-only (D07). Unknown declared bitrate is null.

Source profiles use a fixed first-180s f32 mid capture, first-60s cassette slice, and whole-cap zero-phase FFT band extraction padded to SciPy complex `next_fast_len` (2/3/5/7/11). This is deliberately distinct from native circular STFT correlation. Shared product void band is cutoff+800 to Nyquist-100, with source 93%-Nyquist and 400Hz-width gates. Vinyl reuses the shared profile when present; otherwise it separately measures the source cutoff+1000/Nyquist-100 fallback with the source geometry gate. Cassette bands retain cutoff+1000/+500 and min(20k,Nyquist-100). Outputs expose RMS, standard deviation, physical sample lag, Pearson and complete-second RMS variation independently. Exact-zero/constant correlation and unsupported bands stay unavailable. The DSD spectral ratio never means native DSD decoding.

The actual pinned `_smooth_envelope` uses centered nearest-edge mean(abs(x))*pi/2, not Hilbert. Fourth-order causal Butterworth filters are generated from analog poles and the bilinear transform in f64. Peak candidates retain source absolute height/distance and median-relative click threshold. Preceding-energy output uses eligible attacks (D03); the source all-attack denominator remains audit. Clicks and preceding energy are descriptive, including musical attacks. D05 aliasing/mirroring points are explicitly excluded.

Quiet profile uses the existing corrected first-300 100ms-block survey/collector on native f64 mono or native-channel mean, retaining original zero-block indices and exact integer precision before averaging. This differs deliberately from spectral f32 mid. First-30s effective depth counts exact integer words separately per channel; float inputs are unavailable, not silently rounded into integer evidence. No claim of original depth is made. Quiet HF/music ratio collects at most 30s of qualifying silence across the entire analyzed prefix and uses the source 10–40s music interval when available; otherwise the full <40s interval. Adaptive segment wall and fake-hires candidate consume measured filtered void, never raw STFT magnitude noise.

## Bounded storage plan (before acceptance)

At 384kHz, N=180*r=69,120,000 f32 samples (276,480,000 bytes) is the maximum main PCM capture; it stops growing after 180s. Two silence vectors have at most next_power_of_two(30*r) capacity each, a conservative 134,217,728 bytes combined. Spectral state is fixed 2049-bin arrays; quiet selection stays at 300 scalar blocks and at most 30 selected blocks. No spectrogram is stored.

FFT extraction uses one full complex-f32 transform plus planner scratch and one f32 inverse result; profiles are reduced and released sequentially. At N=69,120,000, payload is 12*N+8*S bytes beyond retained PCM/silence, where S is max forward/inverse scratch. The implementation checks S <= 2*N before scratch allocation; violations are structured `resource_limit` failures and suppress the reference payload. Maximum FFT scratch is therefore **1,105,920,000 bytes**; the conservative combined capture/silence/FFT/result payload bound is **2,346,057,728 bytes** at 384kHz. FFT plans/twiddles are additional. Transient arrays are reduced sequentially; the largest ordinary f64 pair uses 16*N bytes, with peak candidate indices additionally bounded by at most ceil(N/2), and distance-selected events bounded by the cap/distance. HF silence/music FFTs have at most 30*r samples and use f64 Hann/FFT (including potentially prime lengths). Existing native/P04c state and allocator/plan overhead are separate. This is a finite desktop algorithm/storage bound, not a claim that the worst-case workload fits an Android phone; A01/A07 must budget or stage it explicitly.

Cancellation is checked every 65,536 samples in the long smoothing/filter/peak scans, during normal decode batches and between whole FFT/reduction operations. A full FFT or median selection remains a bounded cooperative work unit, not a hard real-time deadline.

## Checks

Final optimized Rust 1.85/no-CLI run: **54 passed** (50 library + 4 API), zero failed/ignored/filtered. This includes five frozen spectral arrays, seven frozen filtered/transient controls (8/48/96kHz, silence, attacks, 61/181s lengths), header/side branches, unavailable geometry/cancellation, and existing P04c immutable oracles/API source mutation/deadline/prefix controls. API checks additionally verify source cap intervals and declared EOF duration.

Final no-CLI example build and focused Clippy with warnings denied passed. Three generated serialized streaming controls passed: unequal stereo, segment splice and 55s/96kHz active gap. They verify exact PCM hashes/intervals, unchanged P04c tolerances, new banding/side/sparsity/envelope and filtered RMS/std/lag/temporal values against the pinned source, v2/native schemas, and 18 schema mutation rejections. Reports/summary remain ignored under `target/p04c-reference-inputs/streaming/`; these are local receipts, not an external backup. Frozen new oracle reproduction, v2 schema reproduction and parity inventory passed. Existing schema v1 and frozen oracle artifacts were not rewritten.

Commands (local Rustup/Cargo paths and existing gcc/lld wrapper configured):
- `cargo +1.85.0 test --offline --release --no-default-features --lib --test reference_inputs`
- `cargo +1.85.0 build --offline --release --no-default-features --example read_reference_inputs`
- `cargo +1.85.0 clippy --offline --release --no-default-features --lib --test reference_inputs -- -D warnings`
- `python scripts/generate_reference_profiles.py --check`
- `python scripts/generate_reference_inputs_schema.py --check`
- `python scripts/check_reference_inputs_streaming.py --reader target/release/examples/read_reference_inputs.exe`
- `python scripts/check_parity_contract.py`

No full integration/CLI suite, Android compile/link/device work, 384kHz worst-case RSS benchmark, private recording analysis, commit, push or upload. The reference checkout and user recordings were not modified; prior owner/review changes are retained. No jobs remain running.

Repaired attempts: Initial compile caught an adaptive-wall argument ordering error and was fixed. A later cancellation-signature/match compile error was also fixed. Initial optimized test linking was blocked by the sandbox's rust-lld execution restriction; retry uses authorized offline linking. Validation builds disable LTO and use 16 codegen units to avoid repeated release-link optimization expense, without changing runtime DSP semantics or Cargo.toml.

Final cap acceptance also ran the reader on generated 61s and 181s/8kHz f32 noise from `generate_reference_profiles.samples`: exact independently computed f64 PCM SHA256, `check_profiles`, first-60s cassette interval, first-180s capture and exactly 300 quiet blocks all passed. Receipts: ignored `target/p04ab-boundaries/cap-{61,181}.json` and generated WAVs. Formatting and `git diff --check` passed after final documentation/status updates.
