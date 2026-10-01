# Version 0.4 AAC lattice validation — 2026-10-01

This historical v0.4 record is preserved. The subsequent spectral-structure
milestone is recorded in [STRUCTURE_VALIDATION.md](STRUCTURE_VALIDATION.md).

AAC observations are now implemented for native mono and explicitly declared
stereo mid/side signals at 44.1/48 kHz. They are provisional measurements, not
verified processing histories. Schema `0.4.0` and policy `observations-only-v4`
keep overall ancestry `INCONCLUSIVE` and evidence index null.

## Implementation and intentional reference differences

The reference remains unchanged at
`c6ecce2296256b516709d87088896d1be913908c`. The arithmetic follows
`SpectralEngine._mdct_quant_error`: a 2048-sample KBD alpha=4 window, the shared
orthonormal MDCT, 49 scalefactor bands, eight scaling levels and 128 phases
stepped by eight samples. PCM is multiplied by 32768 only in this detector's
analysis copy. Its coefficient powers, population gates, rounding-error sums
and band-dependent gamma thresholds follow the reference.

Gamma is `K/12 + sqrt(K/180) * ndtri(.01 + .99*ndtr(-(K/12)/sqrt(K/180)))`,
precomputed for the nine distinct band sizes. The nominal .01 in this formula
is an analytical approximation parameter, **not** a measured detector false-hit
rate after searching many phases/scales. A score of 0.10 is the inherited
provisional hit threshold; it is not calibrated on independent recordings.

The first decoding pass keeps overlapping 2048-sample energies on a 1024-sample
grid, using sums of adjacent hop energies. Storage stops at 180 analyzed seconds:
at most 8436 f64 energy values per basis at 48 kHz. Actual decoded counts, not
untrusted declared duration, bound selection. At most sixteen high-energy anchors
per basis are retained, separated by more than 2048 samples. The second pass
captures only their 3064-sample spans. The existing two-pass PCM integrity checks
remain in force. No full-track PCM, cumulative-energy array or coefficient matrix
is retained. At most 784384 bytes of f64 captured audio exist for stereo AAC;
bounded energy, vote, transform and existing detector buffers are additional.
This is not a process-wide allocation cap on dependency-owned memory.

Explicit differences from Python:

- Mono uses native channel 0. Stereo computes `mid=(L+R)/2`, `side=(L-R)/2` in
  f64, without Python's preceding f32 mid/side rounding. Native-channel
  measurements and integer hashes remain unchanged. Every basis is reported
  separately rather than returning only the largest score across bases.
- Energies use local hop sums instead of subtracting cumulative sums. Exactly
  tied energies prefer the earlier anchor deterministically; selection can
  differ from NumPy's unspecified equal-key ordering. Selected probes are
  reported chronologically. The reference's conservative end-boundary test is
  retained even though the stepped search uses seven fewer samples.
- At least four active anchors and four probes with at least sixteen eligible
  bands at the same phase/scale are required. If no such combination exists,
  the score is null and status is inconclusive. Python can return zero for
  sparse spectra, or maximize using fewer usable probes. No fake-clear zero is
  produced for tones, silence or insufficient evidence.
- A usable probe's flagged count is divided by **all 49 bands**, as in Python;
  a probe with fewer than sixteen eligible bands contributes zero to the mean.
  The mean still includes all selected active anchors in its denominator.
  `selected_eligible_bands` and `selected_flagged_bands` expose the winning
  phase/scale's raw counts; eligibility determines whether they contribute.
- Cancellation/deadlines are checked every eight phase transforms, between
  bases, and in the existing packet loop. Errors propagate to the structured
  file outcome. AAC does not use profile-based score bypasses or ancestry scores.

Reports include the search cap, actual sample intervals (zero-based, exclusive
end), anchor RMS, basis, selected phase offset and scalefactor index. The phase
offset is measured from `anchor_frame - 512`, not from the beginning of the file.
Negative findings apply only to sampled windows and this specific geometry.

## Numerical and regression checks

`scripts/generate_aac_reference.py` reads the pinned Python code and the existing
generated pink-noise fixtures. It writes two generated AAC-to-FLAC fixtures and
`tests/fixtures/aac_reference.json`. The ordinary Rust tests need neither Python
nor FFmpeg. No historical oracle was changed to satisfy a mismatch. During
initial oracle construction, MDCT input samples were added to expose an additional
intermediate numerical comparison; expected scores and signals were unchanged.

Numerical tolerances:

- KBD samples and Princen-Bradley squared-window sum: absolute `2e-14`.
- Band gamma values against SciPy: absolute `1e-13`.
- KBD-windowed 1024 MDCT coefficients against SciPy's DCT-IV, on s16-scaled
  generated audio: absolute `1e-8`.
- Final lattice scores: absolute `1e-12`; selected anchor positions exact,
  anchor RMS absolute `1e-10` (local sums versus cumulative sums).
- Decoded PCM SHA-256: exact equality to independent FFmpeg output.

| Generated mono fixture | Python / Rust lattice score | Status |
| --- | ---: | --- |
| Clean pink noise, 44.1 kHz | 0.016581632653 | No hit |
| AAC 256k, TNS disabled, 44.1 kHz | 0.139030612245 | Provisional hit |
| Clean pink noise, 48 kHz | 0.016581632653 | No hit |
| AAC 320k, TNS disabled, 48 kHz | 0.142857142857 | Provisional hit |

All four use sixteen anchors. Unit checks exercise bounded energy/capture
storage through and past the 180-second cap, exact captured span boundaries,
KBD/MDCT/gamma parity and cancellation from inside the search. Integration checks
cover exact PCM/anchor/score agreement, JSON serialization, declared basis,
dual mono, quiet side, anti-phase stereo, tones/harmonics, silence, isolated
transient, insufficient prefixes and unsupported rates.

## Broader generated processing experiment

`python scripts/check_aac.py` completed 24 cases. Every decoded PCM hash matched
FFmpeg. All results kept ancestry inconclusive and evidence index null. Commands,
versions and related processing histories are in ignored local
`corpus/local/results/aac-v4/manifest.json`; per-file JSON, `summary.json` and
`observations.json` share that directory. The source signals use seed 84 and
are not independent recording groups.

| Processing | AAC outcome |
| --- | --- |
| Clean stereo pink noise at both rates | No hit on either basis; scores 0.0166–0.0217 |
| AAC 128/256/320k, TNS disabled, both rates | Hits on both bases; scores 0.1135–0.3227 |
| AAC 320k, TNS enabled, both rates | Decoded PCM identical to disabled; not validation against active TNS |
| AAC 320k followed by gain 0.37 | Both bases hit on this case |
| AAC 320k followed by 8-sample trim | Both bases hit, phase offset shifts from 512 to 504 |
| AAC 320k followed by 137-sample trim | **Miss** on both bases, scores 0.0166 |
| AAC dual mono | Mid hit, silent side abstains |
| AAC anti-phase stereo | Side hit, silent mid abstains |
| Generated attacks through AAC | Both bases hit on this case |
| Clean dual mono / quiet side / anti-phase | Active basis no hit; quiet basis abstains |
| Silence | Both bases abstain with null scores |
| Clean attack/decay noise | No hit on either basis |
| MP3 320k / Vorbis q8 / Opus 128k controls | No AAC hit on either basis |

The 137-sample-trim miss exposes the inherited eight-sample phase step. The
threshold was not retuned and the miss is retained in the raw evidence. Arbitrary
trimming tolerance is **not** claimed. Enabling TNS in an encoder does not prove
TNS was exercised; this generated source does not establish TNS robustness.
No HE-AAC/SBR fixture or independently characterized music was evaluated here.
Other windows, short-block runs, TNS, SBR, resampling and later processing can
hide a lattice. A high score can also have other causes. Neither a hit nor a
miss establishes recording history or real-world detector accuracy.

## Build, suite and resource status

One final full debug-suite run passed **46 tests**: 17 unit, 4 AAC integration,
13 core/CLI, 5 segment/MQA, 2 original parity/integrity and 5 transform integration.
No failures or ignored tests. The four AAC integration tests also passed in
release mode. Clippy passed for all targets with warnings denied; formatting
passed. Android ARM64 library compilation passed with the default CLI feature
disabled. The release executable built successfully and reports version `0.4.0`.
NDK linking, APK creation, phone
execution and remote CI were not run.

Commands from the project root (local Rust/Cargo 1.98.1 GNU toolchain; the host
GCC driver was set to `C:\msys64\ucrt64\bin\gcc.exe` for linking):

```powershell
./scripts/cargo.ps1 test --locked --offline
./scripts/cargo.ps1 test --release --locked --offline --test aac
./scripts/cargo.ps1 build --release --locked --offline
& ./scripts/cargo.ps1 @('fmt', '--all', '--', '--check')
& ./scripts/cargo.ps1 @('clippy', '--locked', '--offline', '--all-targets', '--', '-D', 'warnings')
./scripts/cargo.ps1 check --lib --no-default-features --target aarch64-linux-android --locked --offline
python -m py_compile scripts/check_aac.py scripts/generate_aac_reference.py
python scripts/check_aac.py
python scripts/validate_local.py corpus/local/generated/resource-check --output corpus/local/results/resources-v4 --source-history 'Generated sine controls reused unchanged from earlier duration experiment'
```

The Python scripts compiled without syntax errors. Fixture generation also ran
successfully using NumPy 2.3.5, SciPy 1.17.1 and FFmpeg 7.1.1; the versions and
commands are preserved in the numerical oracle and experiment manifest.
The release text CLI also passed a 0.1-second-prefix smoke check on
`tests/fixtures/aac_256_44100.flac`: AAC reports inconclusive with no score or
probes, the prefix is explicit, and ancestry remains inconclusive.

The earlier generated 16-bit stereo 48 kHz sine WAVs were reused unchanged:

| Duration | Peak working set | Process wall time | FFmpeg PCM |
| --- | ---: | ---: | --- |
| 10 seconds | 19.48 MiB | 0.624 s | Exact match |
| 120 seconds | 19.72 MiB | 1.886 s | Exact match |
| 600 seconds | 19.80 MiB | 6.834 s | Exact match |

Results are in ignored `corpus/local/results/resources-v4/summary.json`. This
measures the whole Windows CLI, not just AAC. The debug suite ran concurrently;
timing is not a controlled performance comparison. The observed footprint is
consistent with bounded storage, not a hard memory guarantee or Android result.
The private recording collection was not rerun for v0.4; its latest full-file
results remain the v0.3 historical record, including the three corrupt inputs.

## Next work

Complete remaining core feature families (starting with an assessment of the
pinned auCDtect/source-profile features), then evaluate fixed policies on
independent source groups. An exhaustive AAC phase search or explicit TNS/SBR
support would be separate changes requiring their own cost and false-hit checks.
Android linking, device measurements, bindings and UI remain behind the core
acceptance gates.
