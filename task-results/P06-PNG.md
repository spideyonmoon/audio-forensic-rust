# P06 follow-up — calibrated Rust PNG canvas

Completed 2026-10-06 at the owner's explicit request. **DONE**, engine **0.30.0**.

Scope: self-contained Rust PNG with filename/title, native stream metadata
including scoped bitrate, Hann/hop details, calibrated kHz/time axes/grid,
ember-v1 dB legend and native-channel p95 cutoff annotations. Higher resolution
lossless output and antialiased embedded font; no shell/GUI/network runtime.
Preserve P06 artifact v1/numerical geometry, native report schema/policy and
all unrelated work. Update P07/A06 contracts so the engine owns composition.

Delivered `src/spectrogram_png.rs`: Standard 1600×900, Publication 2560×1440
default and Large 3840×2160, lossless RGB8/sRGB PNG with 300-dpi pHYs and embedded
render descriptor iTXt. Header includes source basename or supplied bounded
title, codec/rate/depth/complete-packet audio bitrate/channels, Hann/hop,
basis/full-prefix scope and exact Nyquist. Full frequency/time axes, ticks/grid,
0 to −140 dB ember-v1 bar and native-channel p95 dashed markers/labels are drawn
in Rust. Distinct cutoffs stay distinct; silent/mid cancellation stays visible.
Frequency smoothing averages linear power, time columns retain actual widths;
larger images do not increase the unchanged FFT's measured precision.

`SpectrogramPresentation` v1 binds scoped encoded-packet bitrate to the existing
two-pass PCM hash. Counters add no buffer or pass; partial packets are excluded.
Old saved wrappers without it remain readable and show unavailable bitrate.
`render_canvas` / `write_png_new` validate saved binding, honor cooperative
controls and create new outputs; failed exports return null paths. The example
now defaults to PNG, retains explicit .ppm raw export and accepts a size preset.
P07/A06 cards, release contract, integration, schemas README, roadmap, parity
map, README, porting plan and validation now assign composition/PNG to Rust,
workflow/path/storage/sharing to P07/A06. Native schema/policy/artifact v1 remain
unchanged. No private recordings, commit, push, upload or endgame work.

Snapshot: ignored `target/p06-png/preservation-before.json`.
Final preservation: **375 prior files**, **358 unchanged**, **17 intentional
edits**, zero missing/unexpected; seven new scoped source/font/test/result
files. All **48 private recordings/notes**, owner checklist, Git index, native
schema, spectral v1 schema and frozen DFT fixture retained. Clean pinned reference
verified by inventory. Receipt: `target/p06-png/preservation-final.json`.

## Actual checks

| Check | Actual result |
| --- | --- |
| Optimized Rust 1.85, no CLI | **80 passed**, zero failed/ignored: 54 library + 9 artifact + 6 PNG + 10 source + 1 serialization, final `tests-verified.log` |
| New PNG behavior | Exact decoded RGB equality; sRGB/300dpi/iTXt; full/prefix axis ends; native cutoff identity/close distinct channels; integer/float WAV bitrate; old wrapper/null bitrate; saved hash rejection; title/max RGB bounds; silence/antiphase; collision/missing directory/cancel/deadline passed |
| Offline export example | Built with Rust 1.85; `example-verified.log` |
| Independent PNG decode | Six generated control PNGs at all three presets passed stdlib CRC/zlib/row-filter RGB decode, exact color-bar ends and axis/metadata checks; `verified/summary.json` |
| Frozen P06 preservation | All six spectral artifacts exactly equal prior serialized P06 data; all native numeric fields equal with source/version display exceptions; same hashes/two-pass coverage and schema 0.18.0/artifact v1 validated |
| Compressed bitrate | Generated FLAC: **135197 encoded audio bytes / 48000 frames**, exactly **1081576 bps** after skipping FLAC metadata; ALAC: **20016 stsz audio bytes / 5001 frames at 44100 Hz**, exactly **1412046.550689862 bps**; both PNGs independently decoded; `compressed.log` |
| Prefix with no complete packet | Generated FLAC 0.03 s / 1440 frames yields an available PNG with **bitrate unavailable**, null interval and explicit reason; no invented PCM bitrate |
| Long time axis | Generated 8000 Hz reinterpretation of copied long384k control, 1311745 frames / **163.968125 s**; ticks 0:00, 0:30, 1:00, 1:30, 2:00, 2:30, 2:43.968; exact axis end stored; `time-axis.log` |
| MSRV Clippy | All targets with and without CLI, warnings denied, passed; `clippy-{no-cli,cli}-verified.log` |
| Android ARM64 | Rust 1.85 no-CLI target compilation passed, including png/fontdue; `android-verified.log`. No linking/APK/device claim |
| Contract/tooling | Seven-definition artifact schema and unchanged 47-definition native schema checks; helper syntax, parity inventory/pinned clean reference, Rust formatting and Git whitespace passed |
| Visual QA | Inspected final FLAC publication canvas, high-rate Large burst, antiphase and prefix Standard images: legible complete context, correct orientation/legend/scope, visible native p95/mid warning |

Initial compile errors from shorthand floats were corrected. Initial PNG test
passed 5/6 and failed because its expectation wrongly suppressed native cutoffs
when only the stereo mid canceled; it now asserts actual native cutoffs and the
mid warning. Visual QA found integer formatting repeated short-duration ticks;
fractional ticks now display decimals, regression checks pass. Close low-frequency
cutoff label placement reserves separate rows and retains 0.001 kHz precision.
One test command rejected the PowerShell-stripped separator; quoted `--` retry
ran successfully. No reference/numerical tolerance changed to hide a failure.

The independent comparison initially failed on an older Windows receipt's
UTF-8 caveat imported as cp1252. The checker explicitly reads subprocess/JSON
UTF-8 and repairs only that known Pearson-correlation caveat for comparison
(eight channel occurrences). Saved baseline files are unchanged; no numeric
value or spectral oracle is normalized or regenerated. This text exception is
counted in `verified/summary.json`.

## Reproduce and integrate

Use `scripts/cargo.ps1 +1.85.0` with the configured local linker and
`--target-dir target/p06-msrv`. Test arguments:
`test --locked --offline --release --config profile.release.lto=false --config profile.release.codegen-units=16 --no-default-features --lib --test spectrogram --test spectrogram_png --test source_contract --test report_contract -- --test-threads=1`.
Build `--example read_spectrogram` with the same release options. Clippy uses
`--all-targets` with/without `--no-default-features`, then `-- -D warnings`;
Android uses `check --locked --offline --no-default-features --target aarch64-linux-android`.
PowerShell must quote the argument separator `'--'` when invoking the wrapper.

`scripts/check_spectrogram_png.py --binary target/p06-msrv/release/examples/read_spectrogram.exe --controls target/p06-spectrogram/serialized --output NEW_DIRECTORY`
independently reads the original finite generated controls, compares frozen
reports/artifacts and decodes the Rust PNG without using its encoder/decoder.
The compressed controls are public `tests/fixtures/rate_wall.flac` and
`tests/fixtures/alac/44100-16-2-front.m4a`; their encoded-byte expectations come
from FLAC bytes after the final metadata block and the M4A stsz sample-size sum,
not a decoded bitrate estimate. Extra local receipts are `compressed/` and
`time-axis/`. No user recording was read for validation.

Maximum RGB payload **24,883,200 bytes**, default **11,059,200**, plus v1 data,
font structures, glyph/encoder/allocator/native buffers; no RSS/phone budget
claimed. Embedded Noto Sans Regular/OFL and hashes are in `assets/fonts/README.md`.
Per-character Unicode kerning has no complex-script shaping; missing title
glyphs are visible and recorded. No runtime GUI, shell, font installation or
network is required.

No full CLI/core regression, private audio/calibration, DSD work, Android NDK
linking/APK/physical-device work or source publication was run. Logs/hashes/PNGs
under ignored `target/p06-png/` are local evidence, not Git history or external
backup. All source changes remain local/uncommitted. **No jobs running.**
Next default delivery task remains **F02**. P07/A06 use `CanvasOptions` and
`write_png_new`, retain optional presentation/title, and own storage/sharing.
