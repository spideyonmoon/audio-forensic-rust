# Audio Forensic Rust

Alfred now lives in its [separate repository](https://github.com/spideyonmoon/alfred/tree/codex/extract-android).
Owner requested extraction on 2026-10-09; Android continuation and UI scratchpad
coordination belong there. Core engine ownership and historical acceptance remain here.

Offline audio measurements and provisional forensic observations, available as a
Rust library and desktop CLI. No Python, FFmpeg, SoX, MediaInfo, network service
or Android runtime is needed to use the engine.

Engine **0.32.0** · Rust **1.85+** · MIT

## Current scope

| Area | Status |
| --- | --- |
| WAV / FLAC | Native mono/stereo decoding and documented integrity/precision limits |
| ALAC in M4A | Native 16/24-bit mono/stereo, 8–384 kHz; AAC in M4A is unsupported |
| Metadata | Bounded read-only fields, raw keys/duplicates, technical metadata and audit observations |
| Measurements | Spectral, dynamics, loudness, depth/profile inputs and provisional detector observations |
| Reference assessment | Versioned pinned-Python scores, rule traces and qualified candidate interpretations |
| Spectrogram | Bounded offline data and complete calibrated Rust PNG canvas |
| DSD | **Deferred beyond the first release and Alfred launch**; no shipped DSD analysis |
| Alfred Android app | A01–A06b complete; independent Forensics, Spectrogram and live/saved Compare accepted in Android 11–16 generated UI/runtime CI; A07 physical gate next |

The measurement report keeps ancestry `INCONCLUSIVE` and evidence index `null`.
Reference scores are uncalibrated method outputs, not probabilities or proof of
recording history. Supported geometries, units, nulls, prefixes and caveats remain
explicit. See the [reference guide](docs/REFERENCE.md) and
[validation records](docs/validation/README.md) for exact limits and evidence.

## Quick start

```text
cargo build --release --locked
cargo run --release -- --json path/to/track.flac
cargo run --release -- --progress --summary path/to/track.flac
cargo run --release -- --product-json --fast path/to/album-directory
cargo run --release -- --info path/to/track.m4a
cargo run --release -- --saved saved-product.json
cargo run --release -- --compare original.flac variant.wav
```

Library consumers can omit the CLI and process signal handlers:

```text
cargo check --lib --no-default-features --locked
```

Start with the [host integration guide](docs/CORE_INTEGRATION.md),
[background consumer](examples/background_analysis.rs) and
[versioned JSON schemas](schemas/README.md). For a spectrogram:

```text
cargo run --locked --no-default-features --example read_spectrogram -- input.flac full new-spectrum.png
```

Full report, saved export and qualified comparison examples are in the
[product workflow guide](docs/PRODUCT_REPORT.md).

Output paths must be new; the export never overwrites an existing image.
The [reference guide](docs/REFERENCE.md#build-and-run) covers Windows tooling,
CLI options, exit codes, precision and numerical conventions.

## Delivery and Alfred

**Standalone 0.32.0 accepted at P09 (2026-10-06). A01–A06b complete.
Next: A07 (Astra + owner)**, physical-device/resource acceptance under the
[Android integration contract](ANDROID_CONTRACT.md).
The owner deferred F02/DSD on 2026-10-06 to conserve Astra budget; its frozen
future format/conversion contract remains planned.

After P09, Alfred begins at A01 as a file/folder-first offline Android workspace.
It consumes this library through a separate feature adapter. Shared selection,
input, jobs and storage belong to Alfred; Audio Forensics, Spectrogram and Audio
Compare are independently reachable features. Spectrogram's longer-term ambition
is a competitive viewing/exploration tool; P06/A06a are its initial foundation.
A01's contract retains reusable spectrogram/comparison backends in this core;
reuse existing implementations now. Tag Studio, Audio Converter and
Archival Tools remain future placeholders. Alfred starts here in a separate app
subtree and later moves into its own repository; this core remains independent.

See [roadmap](ROADMAP.md), [task cards](ROADMAP_TASKS.md),
[Alfred architecture](ALFRED_ARCHITECTURE.md) and
[launch contract](RELEASE_CONTRACT.md). Endgame detector research does not block
initial launch. Completed work and historical evidence remain preserved.

## Repository map

| Location | Purpose |
| --- | --- |
| `src/` | Portable Rust engine and CLI |
| `examples/`, `schemas/` | Library consumers and result contracts |
| `tests/`, `scripts/` | Generated fixtures, validation helpers and local build tooling |
| `assets/` | Licensed embedded rendering assets |
| [docs/](docs/README.md) | Integration, reference guide, validation and historical records |
| [task-results/](task-results/README.md) | Completed packet evidence |
| Root planning documents | Current roadmap, contract, handoff and owner decisions |
| `reference/`, `corpus/local/` | Ignored reference/private audio and local research evidence |
| `target/`, `.tools/` | Ignored build receipts and machine-local tools |

For continuation, read [AGENTS.md](AGENTS.md), [HANDOFF.md](HANDOFF.md),
[PORTING_PLAN.md](PORTING_PLAN.md) and the current task card. Documentation has
moved; bare historical validation filenames resolve through the docs index.

## Validation and preservation

P08 differential acceptance passed **271 Rust tests**, all 30 mapped groups
with documented deviations, and explicit deferred-DSD rejection controls.
[P09 accepts the scoped 0.32.0 standalone release](docs/validation/CORE_RELEASE_VALIDATION.md#p09-decision--accept-2026-10-06).
The frozen source/binary and P08 receipts were rechecked; fresh saved-render and
comparison smoke passed. This acceptance is not publication or Android acceptance.

[Validation index](docs/validation/README.md) separates actual per-version checks
from planned acceptance. Android target compilation does not establish linking,
APK creation or phone behavior. Public fixtures contain generated signals only;
private recordings and local receipts remain ignored and are not a source backup.

This local workspace contains the test/fixture/tooling evidence needed for full
validation. Published source snapshots can have different contents; check the
current source tree rather than assuming private/local evidence was published.

## License

[MIT](LICENSE). Embedded Noto Sans uses the [SIL Open Font License](assets/fonts/OFL.txt).
