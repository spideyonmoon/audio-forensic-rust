# Alfred initial Android integration contract

Alfred now lives in its [separate repository](https://github.com/spideyonmoon/alfred/tree/codex/extract-android).
Owner requested extraction on 2026-10-09; Android continuation and UI scratchpad
coordination belong there. Core engine ownership and historical acceptance remain here.

Contract **alfred-host-v1**, frozen by A01 on **2026-10-07** for A02–A07.
The three review corrections were incorporated on 2026-10-07 before any adapter
implementation; this remains the initial, unshipped transport contract.
This specifies work to implement; it does not claim an APK, linked bridge or
device acceptance. The accepted independent engine is 0.32.0 (Rust 1.85 minimum).
[Release scope](RELEASE_CONTRACT.md), [product payloads](docs/PRODUCT_REPORT.md)
and [Alfred ownership](ALFRED_ARCHITECTURE.md) remain authoritative for semantics.

## Application and dependency boundary

Create `apps/alfred/` in A02, with its own Gradle build and `native/` Cargo
workspace (not a member of the root core workspace). Logical Android modules:
`app`, `shared` (selection/input/jobs/storage/native transport),
`feature-forensics`, `feature-spectrogram`, `feature-compare`. Use Kotlin/Compose;
shared code contains no scoring or assumption that every result is a forensic
report. Feature modules depend on shared contracts; shared contracts do not
depend on features. The app registers their operations. Future tools have no
working buttons or APIs in this release.

`native/` builds an app-owned Rust JNI `cdylib`, `alfred_native`, depending on
`audio-forensic` with `default-features = false` through `../../../..` from
`apps/alfred/native/adapter/Cargo.toml`. Unsafe JNI/FD glue belongs only there.
Root source, reports, CLI, schemas and tests stay Android-independent. Root
Cargo commands must continue to work without Gradle, Kotlin or an Android SDK.
Do not modify the core to store URIs, navigation, Android job IDs or databases.

After extraction use a Git Cargo dependency pinned to the full accepted commit
revision of this repository, with the app's own committed Cargo.lock. During
co-location use the explicit path dependency above and record the core revision
and dirty state in build receipts. Do not copy source or consume a floating
branch. Rust ABI is private to the compiled library; JNI transport version 1
is checked at load, independently of engine and feature payload versions.

## Android build baseline

Choose minSdk **30**, compileSdk/targetSdk **36**, covering Android 11–16.
Owner expanded support on 2026-10-07 for Infinix Hot 11S/Android 11.
Keep common memory admission limits; no device-specific chunking or throttling.
GitHub Actions owns heavy build/lint/emulator checks; physical acceptance remains A07.
Primary APK ABI is `arm64-v8a` (`aarch64-linux-android`); add `x86_64`
(`x86_64-linux-android`) only for emulator tests. No 32-bit launch ABI.
Use NDK **30.0.16248370** (r30 LTS shown by the official downloads page checked
2026-10-07). Compile native code against API 30 using that NDK's clang linker.
Pin Rust 1.85.0 initially, Cargo.lock, NDK, SDK platform/build-tools, Gradle
wrapper, AGP, Kotlin and dependency versions in A02; choose a mutually compatible
stable Gradle/AGP/Kotlin set there and record checksums. No dynamic versions.
AGP must support API 36 and be at least 8.5.1 for native ZIP alignment; the
Android 16 setup guide requires at least 8.9.0-rc01, so use a stable version
meeting both conditions. SDK installation/licence availability is an A02
prerequisite, not evidence that any tool is already installed here.

Retain **panic=unwind** for the adapter and core; no abort optimization. Preserve
ALAC's existing decoder unwind containment. Each JNI entry and the worker catch
Rust unwind and return a host error; no panic or Rust object crosses JNI. Abort,
allocation failure or OS kill can still terminate the process: recovery is
interruption, never a successful or fabricated report.

Require ELF LOAD alignment and APK ZIP alignment for 16 KiB pages (explicit Rust
link argument `-Wl,-z,max-page-size=16384`, even with NDK defaults). Check every
packaged `.so`, dependencies and load on both 4 KiB and 16 KiB systems. NDK target
compilation alone proves neither linking nor runtime compatibility.

Official references checked during A01:

- [Android 16 SDK setup](https://developer.android.com/about/versions/16/setup-sdk): API 36 and AGP prerequisite.
- [NDK downloads](https://developer.android.com/ndk/downloads): selected exact LTS revision.
- [16 KiB page support](https://developer.android.com/guide/practices/page-sizes): native ELF/ZIP alignment and runtime checks.

## Selection, capabilities and source leases

The home is a workspace: pick a document, multiple documents or a folder, then
choose Forensics, Spectrogram or Compare independently. A selection has an
app-generated UUID, generation, ordered item UUIDs, URI, display name, nullable
declared byte length and grant state. Names/extensions are display/hints only.
Duplicate document identities in one selection are collapsed; different files
with equal names are retained. A changed selection increments generation and
cannot receive an older selection's capability callback.

Use SAF `ACTION_OPEN_DOCUMENT` with multiple selection and
`ACTION_OPEN_DOCUMENT_TREE`. Take persistable read grants only when offered;
record failure and allow session-only input. No unrestricted storage permission,
write grant, account or upload is required. A URI is never converted to a guessed
filesystem path. Deleted/revoked/missing/denied provider input is a host failure.
[Official SAF guide](https://developer.android.com/training/data-storage/shared/documents-files).

Folder enumeration is **nonrecursive** initially, asynchronous and cancellable.
Read at most 512 child records, retain at most 32 candidate tracks, and cap each
stored name at 1024 UTF-8 bytes. Stop at either limit with `selection_limit` and
an explicit incomplete-list notice; do not silently process a partial folder.
Let the user select a smaller set. Do not reuse the CLI's accumulate-then-limit
directory collector. Sort the bounded retained list by display name then document
identity. Direct selection is limited to 32 tracks; subfolders are not traversed.

Capability result is `available`, `needs_input` or `unavailable`, with operation,
item IDs, reason code, scope choices and required resource class. Initial rules:

| Operation | Selection | Check and result |
| --- | --- | --- |
| Forensics | 1–32 tracks | Sequential per-track product collection, explicit mixed outcomes |
| Spectrogram | One track | Existing P06 collection/canvas; no forensic-screen prerequisite |
| Compare | 2–32 variants of one track or their saved products | User-declared same-track relationship, one shared live scope, then P07 compatibility checks; no music identification or new audio-difference DSP |

Before acquisition completes report `needs_input`, not codec support. Probe
actual bounded container/codec metadata through the cancellable `start(kind=probe)`
operation below, using `metadata::read_metadata_source`; a readable header is not
verified PCM. Apply the existing
FLAC/WAV/ALAC support rules; AAC-M4A, DSF/DFF and other unsupported codecs get a
reason, not a misleading supported extension. DSD stays deferred F02. Final
decode may still fail after successful capability checks. Saved comparison also
checks product version, method/domain and coverage using P07; unavailable or
incompatible ranking retains entries without inventing a winner.

### Comparison scope negotiation

Discovery labels Compare as a reference-method comparison of variants of one
track. Require `same_track_asserted = true` from the user's explicit selection
of that relationship before admitting a live or saved comparison; otherwise
return `needs_input` (native request validation returns `invalid_request`). Do
not infer track identity from names, duration, sample rate or matching metadata.

For live Compare, probe all inputs serially and negotiate **one scope for the
whole operation before product analysis starts**. The request records either
`full` for every input, when all pass full-analysis admission, or a common
user-selected prefix `[0, T)` applied unchanged to every input's `max_seconds`.
Never automatically mix full and prefix or assign separate per-file prefixes.
For the initial UI, use positive whole-second T so its endpoint is an exact
frame at every supported integer sample rate; require T to fit every input's
resource budget. For two five-minute variants at 48/96 kHz, offer at most 90
seconds for both under the current capture cap: 4,320,000 and 8,640,000 frames.
Shorter user-selected common prefixes are allowed; a known shorter input also
limits the offered interval. Unknown duration does not guarantee reaching T.

Preserve the one-snapshot staging limit during negotiation: retain small probe
summaries and encoded hashes, release each probe/snapshot before acquiring the
next, then reacquire each input serially for analysis. A reacquired snapshot
must match its probed encoded hash; otherwise return `input_changed` and require
fresh negotiation. Retain necessary URI grants through the operation. This may
copy an input twice but never retains an entire selection's staged audio.

Recheck actual duration/EOF, method, domain and version with `compare_products`
after all results are available; a shared request does not override the core's
compatibility decision or guarantee compatible coverage. Saved products retain
their original intervals: never trim their JSON or rewrite coverage to make them
match. Incompatible saved or mixed saved/live inputs have no winner; offer a new
explicitly requested live run with a common scope if the sources are available.

Initial acquisition always **stages to app-private storage**, including seekable
providers, to give every native pass an immutable local snapshot. This is a
deliberate first-release stability choice, not a whole-file RAM buffer. A04
tests seekable and nonseekable providers through this same path. Copy one input
at a time using a 256 KiB buffer, count actual bytes even when length is unknown,
reject beyond 700 MiB, and preserve encoded bytes exactly. At known length check
disk admission before copy; during copy recheck quota/free space and cancellation.
Use a random create-new `.partial` file, close/fsync then rename within the same
directory to a ready snapshot. Failed/cancelled copies never become ready.

A source lease contains UUID, job attempt UUID, snapshot path, actual byte count,
encoded SHA-256 and state `ready/in_use/released`. Only shared input storage owns
paths and deletion. The native adapter opens the ready local file on its worker
and owns that `File` through the call; no provider read or Java callback occurs
inside DSP. Selection/grants may be released after snapshot creation, but the
snapshot cannot be deleted while the worker or an export still leases it. Reuse
a ready snapshot only through a counted lease; a retry uses a fresh File and
cancellation token. Compare processes inputs serially, retaining saved products,
not all audio snapshots. No staged audio enters Git or external exports by default.

## Native bridge and exact transport

Use hand-written JNI with Kotlin external methods and UTF-8 **byte arrays** for
small control JSON. Do not use JavaScript, f64 coercion or JNI modified-UTF-8
strings for serialized reports. Keep original result JSON as files, atomically
committed by shared storage, and transfer descriptors rather than giant repeated
arrays. Read with an exact JSON parser: u64 fields require unsigned/BigInteger
handling (including 18446744073709551615); Kotlin signed Long is not a universal
representation. Preserve null, finite values, unknown fields and original bytes
on storage/export. UI formatting never rewrites saved payloads.

The concrete transport methods below are adapter-owned; names may follow JNI
conventions but semantics/version/limits must match:

| Method | Inputs | Output/ownership |
| --- | --- | --- |
| `describe` | None | bridge 1, core version and accepted payload versions |
| `start` | Request v1: kind (`probe` or `feature`), attempt UUID, selection UUID/generation, item IDs, feature ID when applicable, leases, explicit scope/deadline, private output directory; Compare also carries `same_track_asserted` | Opaque positive handle or structured host rejection; takes worker leases only on success; probe takes exactly one ready lease |
| `poll` | Handle | Latest progress and terminal descriptor, repeatable; no blocking join |
| `cancel` | Handle | Idempotent cancellation request; completion may have won the race |
| `close` | Handle | Invalidate handle, request cancellation, detach; idempotent, no join |

Control request/response maximum is 64 KiB. Handle is a positive jlong token
from a non-reused process-local sequence, never a pointer. At most one active
slot and one completed slot exist; drain/close completion before next start.
Registry operations serialize lookup/close under a short mutex and clone shared
state before releasing it. No JNI call, file I/O, DSP or join holds that mutex.
Token overflow is a host rejection; tokens never wrap or become valid again.
Close removes public lookup immediately while the worker retains its state and
lease. A concurrent poll already holding state may return the old attempt ID;
Kotlin discards it after close. Later poll/cancel on a stale handle returns
`stale_handle`; repeated close is successful. Native slot admission remains
occupied until worker exit and source release, even after close/cancel.

Probing uses this same handle lifecycle, native slot and core admission permit;
there is no separate blocking `probe` JNI entry. `start(kind=probe)` returns
before metadata I/O and supplies a fresh token/deadline to
`metadata::read_metadata_source` on the worker. The worker retains the snapshot
lease until it exits. On selection change, shared input invalidates the old
generation, cancels/closes its probe handle and discards late results. Close
does not release the worker's live lease or admit new work prematurely. Failed
start leaves the lease with the caller for release; ordinary completion transfers
the saved metadata descriptor through repeatable poll, then the host closes the
handle. Cancellation/close must run on a control executor that can execute while
the worker is busy, never queued behind the metadata/DSP call it must cancel.

A probe poll carries only identity/generation, progress or terminal status, a
small technical capability summary (codec/rate/channels/precision and nullable
declared frames), a bounded reason and a metadata payload descriptor. Persist
the **complete core MetadataReport v1** as UTF-8 JSON using the same atomic
manifest, 64 MiB document limit, hashes, quota and deletion rules as feature
payloads. Its descriptor carries kind/version/path/bytes/hash; no tag list is
embedded in the 64 KiB control response. The core can retain 1 MiB of tag text
before JSON overhead. Preserve its own truncation/availability diagnostics and
every retained field; never truncate metadata or reject a supported source just
to fit control JSON. Successful metadata access remains available independently
when product DSP admission fails. Cancelled/obsolete unadopted probe outputs are
cleaned after their worker/reader leases are released, not promoted into history.

`AnalysisJob` returns only `AnalysisReport`; do not pretend it returns products.
A03 implements an app-owned single-worker wrapper around
`analyze_source_product` with latest-value progress/token/result, and around
`analyze_source_with_spectrogram` for the independent Spectrogram operation.
Preserve the semantics of `AnalysisJob` and the core's process-wide active
analysis permit; do not introduce a second parallel analysis lane or unbounded
waiting threads. Probe/render/saved-compare also run serially in this initial
adapter. All JNI methods are invoked off the UI thread.

Host failures have `code`, bounded message, attempt UUID and phase, separately
from file statuses. Codes: `busy`, `invalid_request`, `stale_handle`,
`unsupported_version`, `permission_denied`, `input_missing`, `input_changed`,
`selection_limit`, `size_limit`, `storage_full`, `resource_limit`,
`thread_start_failed`, `worker_panicked`, `native_load_failed`, `interrupted`,
`io_error`. Preserve core file statuses `analyzed/unsupported/failed/cancelled/
timed_out` and their diagnostics. Never convert a host error into an analyzed
file or detector verdict. Messages are at most 4096 UTF-8 bytes; do not log raw
metadata/URIs/private paths into public build receipts.

## Shared jobs and Android lifecycle

Persist job UUID, attempt UUID, feature ID, ordered item IDs, scope/options,
creation time and state before starting acquisition. A retry is a new attempt;
old progress cannot mutate it. States are `queued -> acquiring -> running ->
finalizing -> completed`, with terminal alternatives `cancelled/failed/
interrupted`. File-level outcomes live inside the feature result, so completion
can contain unsupported/failed tracks. PNG failure can coexist with analyzed
measurements. Completion means payload/artifact manifests are committed and
leases released; terminal decode progress alone is insufficient.

One shared operation runs at a time, with at most **two queued operations**
(each at most 32 track IDs/options, no open sources or staged copies). Excess
submission is `busy`; cancelling queued work removes it without opening input.
A batch has per-item outcomes and at most one active track. Cancel stops further
tracks and requests the active token; completed item results remain inspectable.
No reset of a used token and no automatic retry loop.

Progress contains attempt/item ID, phase, pass number, frames and nullable
expected frames. Replace one latest event, poll at most every 250 ms while
observed, and conflate UI updates. No duration-sized event stream or fabricated
percentage/ETA for detector stages. Cancellation is cooperative: blocked copy
reads and whole FFT units may delay it. The UI immediately shows cancellation
requested; the worker/lease stays busy until exit. Teardown/close never waits on
the main thread or deletes live files.

Use a same-process, nonexported started foreground service owned by shared jobs,
not an Activity or a WorkManager long-running DSP job. Start from an explicit
user action while visible; foreground promotion and a cancel notification precede
copy/DSP. Android 15/16 uses `mediaProcessing`; Android 14 uses `specialUse`
with subtype explaining offline user-requested audio analysis, since the newer
type is unavailable there. Declare the corresponding type-specific permissions
and `FOREGROUND_SERVICE`. On Android 11–13 use the ordinary foreground service
without passing newer service-type bits. Request notification permission only on
Android 13+ (API 33+), with honest denied-state
handling. This version-specific type choice is an engineering interpretation of
the [service-type definitions](https://developer.android.com/develop/background-work/services/fgs/service-types),
to verify in A05/A07; it is not an approval for a future Play Store release.

Android imposes start restrictions and mediaProcessing time budgets. Use
`START_NOT_STICKY`; do not auto-start from boot/background or evade platform
limits. Catch service-start rejection as a host failure. On Android 15+ timeout,
request cancellation and stop the foreground service immediately without joining;
mark unfinished work interrupted if it cannot finalize. Acquire a partial wake
lock only while active, with bounded timeout and unconditional release, to test
screen-off completion; a foreground service alone is no CPU wake guarantee.
[Start restrictions](https://developer.android.com/develop/background-work/services/fgs/restrictions-bg-start),
[service timeouts](https://developer.android.com/develop/background-work/services/fgs/timeout).

Rotation recreates observers, not work. Leaving the screen keeps the notification
and job alive. Explicit cancel does not promise a fixed stop latency. On process
recreation mark every nonterminal persisted attempt `interrupted`; native tokens
are never persisted/restored. Do not resume halfway through a decode pass or
quietly recompute. Clean orphan `.partial`/unleased snapshots; a retry reacquires
input/grants. Completed atomically committed results survive process death.

## Resource admission and storage limits

These are conservative **initial engineering limits**, not measured Redmi RSS.
A07 must validate before accepting the app. Staging bounds encoded size, not DSP
memory: never downsample, change channels, quantize or silently shorten a request.

| Resource | Initial bound |
| --- | --- |
| Selection/batch/compare | 32 items; folder scan 512 child records, no recursion |
| Staging | 700 MiB per input; one active copy/snapshot; 768 MiB total staged bytes |
| Disk safety | Keep 256 MiB free beyond reserved staging/result writes; reject rather than evict live leases |
| Jobs | One active, two queued; fresh acquisition for each queued operation |
| Product reference capture | At most 8,640,000 native-rate frames in the first 180 s |
| DSP reservation | 1 GiB for product work; start only when system available memory is at least 1.5 GiB and not in low-memory state |
| Spectrogram | Existing 1280 × 513 data; Publication 2560 × 1440 default; Standard 1600 × 900 and Large 3840 × 2160 selectable |
| Saved payload | 64 MiB per document, at most 32 products per comparison |
| History | 32 completed jobs and 512 MiB total result/artifact bytes; whichever is reached first |
| Export | One active; max 32 recorded attempts, user-selected destination |
| Time | 30 min total operation watchdog including acquisition; 20 min per core call deadline |

For product admission, use verified container rate and requested interval to
bound capture frames; unknown duration is conservatively 180 seconds at that
rate for full requests. Reject full requests exceeding the frame cap with an
explicit offered prefix, for example 180s/48k, 90s/96k, 45s/192k or 22.5s/384k.
The user chooses that scope before start. A 60-second fast request is admitted
only if its native-rate frame bound fits. Full 44.1/48k tracks fit the profile
capture cap regardless of file duration, but still have whole-stream runtime
limits. Metadata remains complete within core bounds, and all reference fields
remain exposed with their actual availability; a prefix may remove duration/EOF
interpretations and make comparison incompatible. No reduced scope is labelled
full, no missing score becomes zero, and no Android-only scoring is introduced.

Why a cap is necessary: P04's documented maximum-rate payload is
**2,346,057,728 bytes**, excluding FFT plans/native state/allocator overhead.
Disk staging cannot remove that requirement. Under the new frame cap, retained
PCM plus checked complex-f32 FFT scratch/results is roughly `32*N`, where N is
the actual padded fast FFT length; also budget two silence vectors at
`8*next_power_of_two(min(30*rate, analyzed_frames))`. Other f64 HF FFTs, transient
work, P04c, plans, serialization and UI are additional. The 1 GiB reservation is
a policy margin, not a proof covering all allocations. A03 must bound/check the
padded FFT and rate/interval before invoking product work; A07 measures peak RSS
on admitted worst cases and lowers limits if needed. If the margin cannot be
established, reject that product scope, retain metadata access and offer an
explicit smaller prefix; do not attempt the known >2 GB full-rate case.
See [profile bounds](docs/validation/REFERENCE_PROFILES_VALIDATION.md).

Native Spectrogram uses existing bounded two-pass collection without P04 product
profiles; it still shares the sole worker and gets a conservative 256 MiB
reservation with 512 MiB available-memory admission until A07 measurement.
Preserve the release contract's Publication default for saved rendering and PNG
export, and expose all three existing presets. A smaller UI display preview
does not change the selected export preset or saved options. A06a verifies all
three presets and A07 tests device resource behavior; a render resource failure
is explicit, never an automatic lower-resolution replacement or an unrecorded
removal of a preset. Render after releasing DSP/source buffers. The largest existing RGB canvas is
24,883,200 bytes plus encoder/font/data overhead. Compare saved products one
bounded document at a time where possible; aggregate limit 128 MiB of serialized
input bytes, additional in-memory expansion must be measured. Do not parse 32
maximum-sized documents concurrently. A resource rejection is a host outcome,
not a format-support claim.

History stores payload files/artifacts plus a small shared manifest, with no
audio copies. Before finalization reserve space within quotas, evict oldest
unleased completed entries, or return `storage_full`. Use create-new temporary
files and atomic manifest commit; no result is successful merely because a path
was allocated. User deletion waits for leases, removes only app-owned paths and
releases grants no longer referenced. Partial artifact failure preserves a valid
forensic report with its separate artifact diagnostic. Export uses SAF create
document, sharing uses FileProvider temporary read grants; no silent overwrite
or automatic upload. Export failure leaves local results intact.

## Feature payloads, artifacts and ownership after extraction

Shared result manifest v1 contains job/attempt/feature IDs, host terminal state,
payload kind/version/path/bytes/hash and artifact descriptors (kind/version,
path/bytes/hash/status). It does not add universal verdict/score/channel fields.
Private relative paths must resolve inside the assigned job directory, never
traverse outside it. Validate sizes/hashes/version before loading; unknown future
payloads remain exportable as original bytes and display `unsupported_version`.

Forensics emits the existing product envelope per track; a batch manifest lists
each separately committed payload/outcome. Spectrogram persists the existing
`SpectrogramAnalysis` wrapper (report plus optional artifact/presentation) and
its independently available PNG; it does not construct a product assessment just
to open the viewer. Compare persists P07's comparison envelope plus references
to retained input products, without rewriting their scores. Artifact descriptors
bind to the payload's PCM hash/coverage; unavailable artifacts have no usable
path. Decode result status, payload persistence and image export status remain
distinct. Preserve measurement ancestry `INCONCLUSIVE`/index null and qualified
uncalibrated reference wording throughout all three features.

| Area | Computation and rendering owner | Contracts/tests and consumers |
| --- | --- | --- |
| Forensics | Existing independent core P02–P07; Alfred feature owns UI only | Core report 0.18.0, product `audio-forensic-product-v1`, P05 method/version and generated parity tests stay core; CLI/other hosts/Alfred consume them |
| Spectrogram backend | Existing P06 data collection, calibrated canvas/font/PNG remain core, unchanged | Artifact/presentation v1, numerical/PNG/schema/fixtures stay core; CLI/examples/other hosts/Alfred consume them |
| Spectrogram product | Alfred feature owns workspace entry, viewing, title/preset selection, lifecycle/history/export UI | App navigation/display/SAF/device tests move with Alfred; A06a is the initial slice, not competitive parity |
| Comparison backend | Existing P07 compatibility, ranking, null/no-winner logic and saved rendering stay core | `audio-forensic-comparison-v1`, qualified method/domain/coverage rules and generated tests stay core; CLI/other hosts/Alfred consume them |
| Compare product | Alfred feature owns independent entry, input pairing/history presentation and export UI | App routing/selection/lifecycle/device tests move with Alfred; broader comparison semantics remain future requirements |
| Shared host/native bridge | Alfred owns selection, staging, scheduling, quotas, JNI registry and platform transport | `alfred-host-v1`, host manifest and bridge/device tests move with Alfred; safe core has no dependency on them |

**Decision:** retain the current reusable Spectrogram and Compare computation,
rendering and contracts in the independently consumable library. No new
host-neutral crate is justified for the initial integrations. This is code
ownership, not a claim that Alfred's independent features belong to the
Forensics screen. A future named requirements packet may justify a shared
component; any extraction must supply versioned dependencies and compatibility
facades for existing library/CLI hosts, preserving tests, numerical behavior,
licensed assets and stored version dispatch. Do not duplicate/refactor now.

Repository extraction steps (record remaining work in A08; date not a launch gate):

1. Freeze accepted core revision and app/bridge/toolchain receipts; verify core
   standalone CLI/no-CLI checks and app integration checks separately.
2. Move `apps/alfred/`, its app-owned native adapter, host contracts/build guides
   and app tests to the Alfred repository, retaining licences and history where
   practical. Core source/schemas/generated DSP fixtures/tests remain here.
3. Replace the path dependency with the full-revision Git dependency and lockfile;
   resolve/build offline from provisioned caches, without vendored source copies.
4. Run the same core-payload bridge/generated-input tests against that pinned
   dependency, plus independent root CLI/no-CLI checks. App changes cannot require
   Android dependencies in the core; preserve old saved report/PNG/compare support.
5. Keep a pointer to this original A01 record; Alfred owns subsequent Android
   contract revisions. Core changes continue through independently versioned
   releases, with explicit adapter compatibility updates.

## Finite implementation acceptance ledger

These tests are requirements for later cards, **not executed A01 evidence**.

| Packet | Required checks |
| --- | --- |
| A02 | Pinned repeatable debug APK build, independent root CLI/no-CLI builds, correct arm64 packaging and native alignment inspection; record installed/missing toolchain |
| A03 | NDK link/load and generated WAV/FLAC/ALAC exact-PCM product/PNG smoke; AAC/DSD unsupported; u64-max/null round trip; malformed/version/size rejection; repeated poll/cancel/close, stale tokens, start failure, panic recovery, close during work and source release/admission race; cancellable probe handles and complete >64 KiB metadata via a payload descriptor |
| A04 | One/many/folder selection and incomplete enumeration; 512/32 limits; seekable/nonseekable/unknown-length inputs; exact 700 MiB import and 700 MiB + 1 rejection; grant revoke/missing input; disk-full/cancelled copy cleanup; immutable snapshot and lease deletion; selection-change probe cancellation, late-result rejection and eventual lease/output cleanup |
| A05 | Rotation/background/screen-off, denied notification/start, timeout and cancellation; process kill during copy/DSP/finalization; fresh retry and stale-attempt suppression; queue bounds, no blocked UI joins; atomic history/eviction/deletion/share ownership |
| A06 | All useful product fields inspectable; qualified reference verdict, complete bounded metadata, nulls/large integers/abstention; saved/future-version dispatch, mixed batch failures and PNG failure preserving measurements |
| A06a | Direct workspace Spectrogram entry, P06 data/hash/interval/presentation integrity, Publication 2560 × 1440 default and all three existing PNG presets, preview/export distinction, old wrapper and failed/cancelled artifact behavior |
| A06b | Direct workspace Compare entry, required same-track assertion, shared 90-second scope for five-minute 48/96 kHz variants, single-snapshot re-acquisition/hash checks, shorter/unknown actual EOF and incompatible saved intervals, unlike methods/domains/versions and all-unavailable no-winner results; bounded document retention |
| A07 | Physical Redmi 13 4G Android 16: actual ABI/API/page size/RAM recorded, peak RSS/runtime/thermal and admitted profile bounds, 700 MiB import, screen-off/cancel/process recovery; Android 11–16 emulator compatibility; Hot 11S/Android 11 physical checks when available plus 16 KiB emulator load, clearly labelled; owner U03 acceptance |

Unresolved product choices: final application ID/signing key (U04/A08); broader
interactive Spectrogram requirements/competitive targets and comparison expansion
(later named packets); future tools and deferred F02. None requires inventing an
API now. A07 can tighten unmeasured resource defaults with explicit receipts;
it cannot silently waive fidelity, 700 MiB import or useful-result exposure.
No consequential architecture decision is left for the implementer to guess.
