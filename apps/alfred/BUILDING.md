# Alfred scaffold builds

A04 development workspace, **0.1.0-a04**, development ID
`dev.alfred.workspace.debug`. Android 11–16: min 30, compile/target 36.
The workspace supplies bounded single/multiple/folder SAF selection, offered
persisted read grants, snapshot staging, native metadata probes and independent
feature routes. Feature execution/results and background lifecycle remain
A05/A06/a/b. Rotation clears the current selection; A05 owns restoration.
No INTERNET or unrestricted storage permission.
Future tools have no buttons. Phone validation remains A07, not a build claim.

## Pinned prerequisites

| Tool | Version |
| --- | --- |
| Rust | 1.85.0, aarch64-linux-android std |
| JDK | Temurin 17.0.16+8 (local Windows bootstrap) |
| Gradle wrapper | 8.13, distribution SHA256 in wrapper properties |
| AGP / Kotlin and Compose compiler | 8.11.1 / 2.2.21 |
| SDK platform / build-tools | android-36 revision 2 / 36.0.0 |
| Command-line tools / NDK | 19.0 / 30.0.16248370 |
| Activity Compose / Material3 | 1.10.1 / 1.3.2 |

AGP 8.11 supports API 36, requires Gradle 8.13 and JDK 17:
[official compatibility](https://developer.android.com/build/releases/agp-8-11-0-release-notes).
Kotlin 2.2.21 supports this Gradle/AGP pair:
[Kotlin compatibility](https://kotlinlang.org/docs/gradle-configure-project.html).
The [tool archive manifest](scripts/toolchain-windows.json) pins URLs and upstream
checksums, including exact SDK archive revisions. Gradle dependency lockfiles and
`gradle/verification-metadata.xml` pin the resolved artifacts and SHA256 values.
The native workspace has its own Cargo.lock. No dynamic versions.

## Windows provisioning and build

Run from the repository root in PowerShell 7. Provisioning downloads approximately
1.4 GB of archives and needs several GB of free disk space. Tools/caches remain
under ignored `.tools/alfred/`; it changes no system installation or global PATH.
Read [Android SDK terms](https://developer.android.com/studio/terms) first; the
explicit switch below authorizes the license acceptance step. Provisioning never
silently accepts licenses without that switch.

```powershell
& apps/alfred/scripts/provision.ps1 -AcceptAndroidLicenses
& apps/alfred/scripts/build.ps1 -Online
```

The first build needs network access for the pinned Gradle/Maven/Rust artifacts;
provision those caches before an offline build. Routine repeat:

```powershell
& apps/alfred/scripts/build.ps1 -Clean
```

`-Online` permits dependency retrieval, not application networking. The default
passes both Cargo and Gradle `--offline`, with Cargo `--locked` and Gradle strict
checksum verification. `-Clean` removes only Gradle build outputs before
repopulating native packaging and rebuilding. `-ResolveLocks` is only for an intentional dependency
update: review changed lockfiles/checksums. `-GenerateWrapper` is only for the
initial wrapper or a reviewed Gradle upgrade, using the checksum-pinned local
distribution. Do not regenerate verification metadata to hide a mismatch.

On this Windows host the first cold Gradle setup failed to rename an immutable
transform cache directory; a fresh process using the retained cache passed.
This matches [Gradle's Windows cache issue](https://github.com/gradle/gradle/issues/31438).
If this occurs, retain the failed log and retry once after that Gradle process
exits. Do not disable security software or manually promote partial caches. A
repeated failure requires investigation or a reviewed compatible tool upgrade;
no cache deletion or dependency checksum regeneration is a build fix.

Build driver links API 30 arm64 with the exact NDK clang and explicit 16 KiB ELF
alignment, retains panic=unwind and packages uncompressed native code. APK
verification inspects every `.so`, all ELF LOAD segments and actual ZIP offsets,
then runs zipalign and apksigner. Missing native code fails preBuild. The bridge
exposes caught-unwind worker handles and exact JSON transport;
see [native adapter](NATIVE_ADAPTER.md) for ownership and generated-input checks.

Output: `app/build/outputs/apk/debug/app-debug.apk`, with `alignment.json` beside it.
`build/receipts/build.json` records core HEAD, dirty paths, toolchain, Cargo hashes
and APK hash; these are ignored local evidence, not source history or backups.
The standard Android debug signing key is generated locally under ignored
`.tools/alfred/debug.keystore` (public Android debug defaults), development-only; public signing/key
custody stays U04/A08. Never publish this as an accepted analysis app.

On other hosts provision the same Rust/JDK/SDK/NDK versions, set JAVA_HOME and
ANDROID_HOME, then use Cargo plus the wrapper directly. Replace the NDK prebuilt
host/linker path with that host's API-30 clang. Windows and Linux CI builds have
passed; macOS execution and physical phone runtime remain unverified. Do
not add x86_64 to the primary APK; build a separate emulator variant in A03/A07.

## GitHub Actions compatibility checks

Heavy builds run in `.github/workflows/alfred-android.yml`, on app/core changes
or manual dispatch. Linux builds separate ARM64 phone and x86_64 emulator APKs,
links Rust against API 30, runs strict dependency verification and Android lint,
then verifies ELF/ZIP alignment and signatures. Artifacts include APKs and build
receipts, retained seven days. No private audio is used or uploaded.

The emulator matrix covers API 30–36 (Android 11–16, including Android 12L),
with seven concurrent emulators. It checks install, workspace rendering, JNI
generated-input product/metadata/PNG acceptance, bounded staging/provider controls
and real DocumentsUI single/multiple/folder read grants. These are x86_64
emulator checks, not ARM64 phone or 16 KiB runtime acceptance. A07 still measures
physical memory/background behavior. Common admission limits remain unchanged;
no Hot 11S-specific chunking or throttling is introduced.

Harness-only pushes use `.github/workflows/alfred-runtime.yml` and reuse verified
APK artifacts. The source guard checks compiled/packaging inputs and binds the
APK SHA-256 to its verification receipt; source changes require a fresh build.
Manual dispatch accepts an existing `build_run` and affected API JSON array
(for example `[30]`) to retry only the failed platform. Runtime reuse skips the
already accepted A03 native smoke and retains all A04 input/provider/UI checks.
No build is skipped when application/native/packaging inputs change.

For a provisioned Linux host use `bash scripts/build-linux.sh arm64-v8a` or
`bash scripts/build-linux.sh x86_64`. CI installs the same pinned SDK/NDK/Rust
versions; first resolution needs network. It never regenerates dependency
checksums to bypass verification failures. Windows offline builds stay available.

## Independent core consumption

`native/` is its own Cargo workspace; adapter consumes `../../../..` with
`default-features=false`. Shared has no feature dependency; features depend on
Shared; App registers all three routes. No core source is copied or refactored.
Independent root commands need no Android tools:

```text
cargo +1.85.0 build --locked --bin audio-forensic
cargo +1.85.0 build --locked --lib --no-default-features
```

After extraction replace only the path dependency with a full accepted core Git
revision and the app lockfile, under [the contract](../../ANDROID_CONTRACT.md).
P06/P07 backends/contracts/tests stay core. A03 starts the analysis adapter;
A04 replaces session selection with bounded SAF identities/probing/staging.
