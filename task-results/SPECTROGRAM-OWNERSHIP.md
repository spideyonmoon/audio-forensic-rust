# Spectrogram ambition and reusable-code ownership — 2026-10-06

DONE: owner-requested planning clarification only. Spectrogram is a first-class
Alfred viewing/exploration feature with the ambition to rival existing apps;
image generation is only its initial foundation. A06a remains a bounded initial
P06 integration, not completion of the larger feature or competitive parity.
Interactive requirements, competitors and expansion acceptance remain unfrozen.
Candidate requirement areas are discussion prompts, not selected APIs or new
launch gates. No competitive survey or implementation was requested/performed.

A01 now explicitly owns long-term reusable spectrogram and comparison boundaries
before Alfred extraction. Its acceptance requires an ownership/consumer matrix
for computation, rendering, contracts and tests, a justified location decision,
dependency/version/extraction plan and independent CLI/host compatibility.
This does not preselect a new crate or move existing functionality now. Reuse
P06 and existing comparison code without duplication/refactoring; core must never
depend on Alfred. Audio Compare remains a first-class feature whose initial
reference-method ranking does not define every future comparison workflow.

Architecture, roadmap/cards, release-scope clarification, porting plan, owner
requirements, README, AGENTS and current handoff updated. Completed P06/P/F
records, validation content, Rust source, schemas and fixtures unchanged this
turn. P07 remains next; F02/DSD remains deferred beyond initial release/launch.
No implementation task or job started, commit, push, upload or repository move.

Validation: parity inventory and local Markdown target checks passed; whitespace
check passed. These are documentation checks, not numerical/Android acceptance.
No Rust/DSP/Android build or tests rerun. Next: requested P07, then existing gates;
A01 later resolves ownership, and a named future requirements packet expands
Spectrogram scope before implementation.
