# Shared job and result integration

A05 supplies infrastructure; A06/a/b supply feature execution/results screens.
Forensics is accepted by A06; independent Spectrogram and live/saved Compare
passed A06a/b generated worker/Compose acceptance on Android 11–16 (2026-10-09).
Saved Compare pins selected attempts at queue admission, retains original coverage/
order and copies original products into its own result history. Native terminal
error codes survive the host adapter. No core computation is duplicated.

Call SharedJobs.get(applicationContext).submit from an explicit visible user
action. Persist chosen scope/options and ordered item IDs. For retry, supply
fresh FeatureWork and the same job ID; attempts/tokens are always fresh. Pass
retainGrants = inputs.retainGrants to pin selection grants after queue admission.
Queued work owns no open input/staged audio. Never get a future on main.

The manager enters acquiring after foreground promotion/output reservation.
FeatureWork calls phase(running), processes one item at a time, then calls
phase(finalizing) before returning. Use context.check between items/calls and
inputs.acquire(item, context.record.attemptId, context.cancellation) in a use
block. Encoded hashes bind reacquisition to the verified selection. Supply actual
Android memory/native scope and a core deadline of at most 20 min.

Use context.runNative(request, snapshot, ownedOutput, client). Transient output
belongs under files/input-probes. The helper pins paths, rejects foreign attempts,
polls at 250 ms, closes and waits off main for actual native release. Failed release
proof preserves pins/blocks admission until force-stop/recreation. Tokens are not
persisted/restored. No Activity disposal joins work. After release verify native
descriptor path/size/hash bindings, then context.persist unchanged payload/artifact
bytes. Record per-item outcomes independently of host state/PNG diagnostics.
Source/native-output use blocks must exit before FeatureWork returns. The manager
commits the manifest last after work/grant leases release; decode completion alone
cannot succeed. Reserve total payload/artifact/manifest bytes within 512 MiB,
at most 64 MiB per document. Input disk safety includes the live reservation.

Observe immutable snapshot/progressSnapshot only while visible, at most every
250 ms. One latest event has attempt/item identity, phase/pass/frames and nullable
expected frames. Cancel stays busy until worker exit; cancelRequested is visible.
Platform timeout stops service/wake lock immediately and requests cancellation
off main. START_NOT_STICKY prevents background/boot resurrection. Startup keeps
committed manifests, interrupts other attempts, removes owned orphans and releases
journaled app-owned grants. Retry reacquires input explicitly.

Read/export/delete off main. Pin attempts while reading; ResultStore.load verifies
descriptors, dispatch checks kind/version and retains unknown original bytes.
Deletion/eviction reject leased entries. History owns no audio. ResultDestination
uses ACTION_CREATE_DOCUMENT; ResultTransfer permits one transfer/32 receipts.
Launch the returned share Intent/chooser on main; it carries ClipData/temporary
read permission for FileProvider cache copies. Only result-shares is exposed.
Copies remain usable after history deletion and become cleanup-eligible after
ten minutes on the next share request, bounded at 32 copies/128 MiB. No upload.

Generated acceptance uses JobSmokeActivity/jobs-smoke.py on APIs 30–36. Emulator
service/rotation/kill/timeout evidence does not establish physical ARM64 memory,
thermal, screen-off or 16-KiB acceptance. Resource defaults remain for A07.
