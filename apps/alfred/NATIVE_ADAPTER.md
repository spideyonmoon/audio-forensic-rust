# Native adapter v1

The app-owned `native/adapter` wraps the unchanged 0.32.0 core. JNI transfers
UTF-8 byte arrays, capped at 64 KiB. `NativeClient` invokes it on a bounded control
executor, independent of the native worker; never await its futures on main.
`ExactJson` reads integer tokens as BigInteger and decimals as BigDecimal. Store
and export original payload bytes, including null and unknown fields.

`describe/start/poll/cancel/close` return `{ok,value}` or `{ok:false,error}`.
Errors contain code, message, attempt_id (null before request identity is known)
and phase. `describe` identifies versions and `active_handle`. Start validates
before spawning and never performs metadata/file I/O on the caller. One slot is
retained until close; even after close, `active_handle` remains occupied until
source release. Tokens are positive, monotonic, process-local and never restored.
Cancel and close do not join. A terminal poll is repeatable and means source
release plus manifest publication, not just final decode progress.

## Request

Required fields: `version:1`, `kind:probe|feature`, job/attempt/selection UUIDs,
unsigned generation, ordered item UUIDs, nullable feature, leases, explicit
`scope:{kind:full}` or `{kind:prefix,seconds:...}`, deadline_ms (1–1,200,000),
absolute private output_dir, available_memory_bytes, low_memory and
reserved_output_bytes. Optional fields: saved descriptors, same_track_asserted,
spectrogram, png and preset (publication default; standard/large supported).
Unknown request fields are rejected, preventing misspelled options.

A lease includes id, attempt_id, absolute ready snapshot path, actual bytes,
encoded SHA-256 and bounded display name. One lease/item is accepted for probe,
Forensics or Spectrogram. The worker verifies the encoded hash with a 256 KiB
buffer and cancellation/deadline checks, then opens core passes over that same
file. The host owns immutability and counted deletion leases. Failed start leaves
ownership with the caller; successful start retains it until terminal poll or,
after close, until describe.active_handle differs from the old handle.

Features are `forensics`, `spectrogram`, saved `compare`, and saved `render`.
Forensics preserves the complete P07 product and optional spectrogram/PNG.
Spectrogram does not run reference product profiles. Comparison requires 2–32
saved products, matching item IDs and same_track_asserted=true. Each saved entry
has an absolute root and payload descriptor. A04/A06b orchestrate live comparison
by probing/reacquiring one snapshot at a time under one negotiated scope, then
passing the resulting saved products. Saved rendering accepts one product or
SpectrogramAnalysis descriptor, validates its binding and uses the core renderer.
No saved input is rewritten to make its coverage compatible.

## Storage and resources

Shared A04/A05 storage must reserve output quota and keep 256 MiB free beyond
reserved writes before calling start; `reserved_output_bytes` is that reservation,
not a request to evict history. It must supply actual Android memory admission
values. The debug harness supplies synthetic admission values for tiny generated
fixtures; its success is not a phone RSS/memory guarantee.

The worker creates a new attempt UUID subdirectory under output_dir. JSON streams
to bounded create-new partials, fsyncs, renames, and publishes manifest.json last.
Descriptors contain relative filename, kind/version, actual bytes and SHA-256.
Reads reject path traversal, size/hash mismatch and unsupported versions. Failed
publication removes only the newly-created attempt. PNG failure is separate from
the valid report; PNG is rendered after source/DSP buffers are released.

The shared host owns adopted result reader leases, closed/obsolete output cleanup,
history promotion/eviction and interruption recovery (A04/A05). After close,
do not delete the attempt directory or snapshot until native admission releases.
Cancelled probe payloads must not be promoted into history. No native call deletes
the source snapshot. A process death leaves an orphan attempt for host recovery.

Product admission checks the actual probed rate, requested prefix, 8,640,000-frame
capture cap, smooth padded FFT length, two silence vectors, 512 MiB overhead margin
and 1 GiB reservation. A full high-rate request conservatively assumes the full
180-second capture even if the header claims a shorter duration: declarations
are not verified EOF. It offers an explicit smaller prefix rather than trusting
a false duration to under-admit. 48k/180s, 96k/90s, 192k/45s and 384k/22.5s fit the
engineering bound. A07 must measure real peak RSS and tighten if necessary.

## Checks

Host: `cargo test --release --locked --manifest-path apps/alfred/native/Cargo.toml`.
Android CI builds/links both ABIs, verifies packaging and invokes debug-only
NativeSmokeActivity on emulator APIs 30–36. The harness/assets are absent from
release builds. The smoke verifies JNI controls, exact integers/null, generated
PCM hashes and PNG publication; source lifecycle races and injected spawn/panic
failures are deterministic host tests. Neither proves physical ARM64 behavior.
