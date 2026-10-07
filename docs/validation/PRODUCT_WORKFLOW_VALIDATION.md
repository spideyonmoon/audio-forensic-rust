# P07 product workflow validation — 2026-10-06

Engine 0.32.0. Scope: W01–W08/W10 and G26–G28 over the existing P03/P03a,
P04/P05 and P06 outputs, not a new detector calibration or full-core release gate.
Native measurement schema 0.18.0/policy/INCONCLUSIVE/null index are unchanged.
The independent forensic envelope is `audio-forensic-product-v1`; the proposed
`alfred-product-v1` is not emitted. Comparison is `audio-forensic-comparison-v1`.

## Implemented contracts

The source product API runs bounded metadata/P03/P03a/P04 and optional P06
collectors together on one open guarded source, worker and cooperative deadline.
Three verified PCM passes belong to the reference adapter; native report coverage
still records its two native passes. Metadata parser inspection now returns its
existing container preflight geometry, avoiding a separate file/worker or repeated
preflight. Existing native and metadata consumers retain their signatures.
No DSP, score weight, detector threshold, native schema or public audio oracle
was changed. Product optional fields/units/scopes/raw tags are retained, including
all 155 Python field locations and qualified legacy aliases. Elapsed time is
wall-clock analysis time, includes worker waiting and is not an ETA.

Info mode reads declarations and bounded tags without audio decoding. Saved
rendering checks version, selected stream, hash/coverage, assessments recomputed
only from stored inputs, adapter/statistics methods/domains and artifact binding.
It does not re-open audio. Full text includes structured inspectable details,
qualified reference scores/source/depth candidates, native channels, units,
intervals, missing values and diagnostics. Alias availability is rebuilt from
stored data; no policy effects depend on it.

The comparison tuple executes the pinned _compare_key criteria with stable
input ties, explicit null missing fields and attributed sort-only defaults.
It retains legacy first-channel DR/depth categories, separate from product
native/overall results. Caller asserts one-track variants; compatible methods,
reference basis and duration/EOF coverage are required. All-unavailable or
incompatible inputs have no winner; incompatible ranks are null. No alignment,
provenance verification, population calibration or sound-quality ranking.

P06 owns PNG composition/numerics. Product exports call write_png_new with new
caller paths, preserve optional title/presentation, expose all three presets,
and retain separate export failures with no path. Existing output bytes survive
collision attempts. Saved successful export paths are historical receipts;
filesystem existence is not inferred from them.

## Scoped evidence

| Check | Actual result |
| --- | --- |
| Rust 1.85 optimized focused suite | 103 passed, zero failed/ignored: library 59, CLI units 3, prior CLI 4, metadata 10, product 6, product CLI 3, assessment 2, report 1, spectral 9, PNG 6; target/p07/tests-final.log |
| Final strict tool-domain / directory-failure-progress edits | 9 targeted product/product CLI tests passed, zero failed/ignored; target/p07/tests-domains.log |
| Combined collection equality | Same generated 18s source: exact serialized native measurements, bound reference inputs, byproducts, tool statistics, spectral data and presentation equal existing independent APIs |
| No-decode info | Header-only source spy rejects any PCM payload read; passed. Existing post-PCM metadata test also passed |
| Saved / absent fields | Native JSON remains exact; saved rerender after audio deletion, null optional products, absent presentation/title, u64 max, escaping and stale version/hash/stream/score rejection passed |
| Progress/failures | Mixed directory/file batch, empty/nonrecursive/sorted paths, failed files, real pass 3 cancellation, immediate deadline and prefix scopes passed; old native CLI contract passed |
| Comparison | 14 frozen controls execute four original pinned Python functions; every tuple axis/missing default/stable tie passed. Generated gain variants rank deterministically. Ties, all unavailable, partial, incompatible coverage/missing method and invalid DR domain passed |
| Artifact workflow | Stored title, Standard preset actual 1600×900 PNG, optional old presentation, create-new collision preservation and independent export failure passed |
| Independent schema / PCM workflows | 107-definition product and 3-definition comparison schemas validated offline; 9 invalid version/policy/unknown-property documents rejected by schema and CLI; independent exact PCM hash, u64 max and saved no-audio checks passed; target/p07/independent/summary.json |
| Actual 60s alias | Generated 61s 8kHz PCM: all product collectors/coverage and optional spectrogram bounded to 480000 frames, EOF false; passed |
| Compressed product path | Generated FLAC and ALAC-M4A metadata/measurement/product schema passed with optional spectrogram |
| MSRV Clippy/no-CLI | Rust 1.85 all-target Clippy with warnings denied passed; no-default-features library/examples compilation passed; target/p07/clippy-final.log and no-cli-final.log |
| Contract/inventory | Product/field map/comparison/native schema drift checks passed; pinned reference clean; inventory 155 fields/139 methods/30 groups/34 rules, acyclic task gates passed |

Independent generated 18-second PCM SHA-256:
`532e04842f171ac56c9ff02101e3427e191afbfde60ad693b12beb2f31b80179`.
It hashes exact little-endian MSB-aligned signed32 samples from the public
24-bit generated WAV. It is not the source file hash or an authenticity claim.

## Initial attempts and remaining checks

The first scripting attempt used Windows cp1252 instead of UTF-8 and stopped
before any file edit; explicit PYTHONUTF8 corrected it. Initial compilation
found inspect's prior unit return and the wrong Coverage field spelling;
returning existing ContainerInfo and using analysis_passes fixed them. Sandbox
execution blocked rust-lld; offline locally authorized linker runs proceeded.
No download or new toolchain installation was needed.

The initial focused run passed 81 tests and failed one product prefix assertion;
later executables had not run. The assertion expected exactly 0.125 seconds at
44.1kHz rather than floor(0.125×44100)=5512 whole frames / 44100. Only the test
expectation changed; prefix code and prior oracle remained unchanged. The final
103-test run and 9-test scoped final checks passed. The initial schema exporter
flagged an ordinary &impl Serialize generic bound as custom serialization; the
product-only exporter ignores that non-model bound explicitly without changing
the strict native exporter. No accepted numerical fixture was regenerated.

Initial Clippy rejected a collapsible nested conditional in alias presentation.
The isolated equivalent style correction passed final all-target Clippy. Final
binary smoke rejected an invalid tool domain, emitted terminal progress for an
empty directory and consumed the two independently checked saved products;
`target/p07/final-boundaries/summary.json`. No numerical method changed.
Helper syntax, Rust formatting, whitespace and all 251 final local Markdown
targets passed; the final schema accepted generated, saved and absent-field
receipts (`target/p07/final-doc-checks.json`). The link reader initially assumed
UTF-8 for a pre-existing cp1252 noise-floor validation record; read-only fallback
preserved that document's bytes and encoding.
P07 DONE. Existing dirty/untracked work is preserved; no reset/clean/commit/push.

## Bounds and not-run scope

Product/info CLI batches: 32 inputs/expanded results. Saved JSON: 64 MiB per
file. Per-product export history: 32 attempts. Core P04/P06 buffers retain their
existing published bounds; this is not a measured Android/RSS budget. Larger
saved batches should be split. Reference coverage compatibility is intentionally
conservative: duration/EOF equality, no implicit alignment or unlike-track rank.
Blocking host I/O/cooperative cancellation limitations remain unchanged.

No full-core/P08 regression, DSD decoding, private recording analysis,
calibration, MQA confirmation, Android target/link/APK/device check, remote CI,
publication, commit or push ran. F02 remains deferred; P08/P09 remain separate
release gates. Ignored target/p07 receipts are local files, not Git history or
an external research backup. Ordinary examples and API/limits are in
[the product guide](../PRODUCT_REPORT.md).
