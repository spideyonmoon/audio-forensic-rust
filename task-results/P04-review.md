# P04 review — 2026-10-05

Owner requested a review of all P04-related work completed by the previous
session, with corrections where needed. Starting source: clean `ed62325`.

## Scope and findings

- P04c is the only completed P04 packet. P04a is READY and P04b is TODO;
  there is no P04s card. Missing G09/G10/G12 spectral/header/resampling inputs
  and G16–G21 source-profile adapters are accurately assigned to P04a/P04b.
  This review does not implement those separate packets or mark them complete.
- Inspected P04c's card, parity/release contracts, implementation, frozen
  generated oracles, validation helpers and saved results against the unchanged
  Python source. Reviewed f32 basis conversion, strict STFT/activity geometry,
  base reductions, source scatter stride/mode, phase gaps/energy gates,
  Random(42) segment plan/ring, vote/nearest-wall/adaptive dependencies,
  AAC/Vorbis basis selection, and the third-pass source/hash/failure boundary.
- No production-code defect requiring correction was found in this review.
  Intentional applicability/rounding differences are documented. In particular,
  Vorbis reconstructs L/R after widening f32 M/S; adaptive dependencies remain
  unknown rather than borrowing native or STFT-noise values.
- Found a test gap: the original third-pass mutation control damaged the WAV
  header. It exercised malformed decoding, not the successful-decode PCM hash
  mismatch branch. Expanded that test with a single f32 mantissa-bit change
  at frame 40000, activated only on pass 3. Header, sample count and PCM kind
  remain unchanged. The assertion requires the exact source-change diagnostic
  and suppression of reference inputs, retaining the old malformed case.

## Validation

Focused optimized Rust 1.85/no-CLI lib and reference-input API run passed
**50 tests (46 lib + 4 integration), zero failed/ignored/filtered**. The expanded
mutation test reached the exact PCM-mismatch branch and suppressed the payload.
Command: `cargo test --offline --release --no-default-features --lib --test reference_inputs`.
Separate reference schema reproduction and parity inventory checks passed;
the Python reference is clean at `c6ecce2296256b516709d87088896d1be913908c`.
The first build attempt lacked local RUSTUP_HOME; after correcting that, the
sandbox blocked rust-lld execution. The authorized offline test retry passed.
Focused Rust 1.85 Clippy passed with warnings denied:
`cargo clippy --offline --no-default-features --lib --test reference_inputs -- -D warnings`.
Formatting verification passed. The first sandbox lint attempt also blocked
linker execution; the authorized retry pinned Rust 1.85 explicitly and passed.

Production behavior/version/schema and frozen oracles are unchanged. No full
regression, new streaming corpus run, Android/device validation, private audio
analysis, commit or push is part of this review. Next delivery task remains P04a.
No jobs remain running. Changed files: `tests/reference_inputs.rs`, this review,
`REFERENCE_INPUTS_VALIDATION.md`, `ROADMAP.md` and `HANDOFF.md`.
