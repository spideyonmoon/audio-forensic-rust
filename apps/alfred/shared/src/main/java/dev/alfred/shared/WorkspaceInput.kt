package dev.alfred.shared

import android.app.ActivityManager
import android.content.Context
import android.os.Handler
import android.os.Looper
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.math.BigInteger
import java.util.UUID
import java.util.concurrent.ArrayBlockingQueue
import java.util.concurrent.ThreadPoolExecutor
import java.util.concurrent.TimeUnit

data class ProbeCapability(val itemId: String, val status: String, val reason: String,
                           val hash: String? = null, val rate: Long? = null, val frames: BigInteger? = null)
data class WorkspaceState(val selection: InputSelection? = null,
                          val probes: Map<String, ProbeCapability> = emptyMap(),
                          val busy: Boolean = false, val notice: String = "", val sameTrack: Boolean = false)
data class Capability(val state: String, val reason: String, val resource: String, val prefixSeconds: Long? = null)
data class FeatureInputs(val selection: InputSelection, val probes: Map<String, ProbeCapability>,
                         val acquire: (InputItem, String, InputCancellation) -> OwnedInput,
                         val retainGrants: () -> AutoCloseable)

/** BigInteger.longValueExact is unavailable on Android 11. Check its range
 * explicitly without losing the exact integer tokens in stored payloads.
 */
fun BigInteger.checkedPositiveLong(): Long {
    if (signum() <= 0 || bitLength() > 63) throw InputFailure("invalid_request")
    return toLong()
}

fun operationCapability(operation: Operation, state: WorkspaceState): Capability {
    val items = state.selection?.items.orEmpty()
    val resource = if (operation.id == FeatureId.SPECTROGRAM) "spectrogram" else "reference_product"
    if (!operation.acceptsSelectionShape(items.size)) return Capability("needs_input", "Select ${operation.minimum}–${operation.maximum} tracks.", resource)
    if (state.selection?.incomplete == true) return Capability("needs_input", "selection_limit: confirm a smaller selection from the incomplete folder list.", resource)
    if (items.any { state.probes[it.id] == null }) return Capability("needs_input", "Acquisition and codec verification pending.", resource)
    val unsupported = items.count { state.probes[it.id]?.status != "available" }
    // Forensics preserves mixed per-item outcomes; the other initial operations
    // need all selected inputs. The batch route records the unsupported entries.
    if (unsupported == items.size || (unsupported > 0 && operation.id != FeatureId.FORENSICS)) {
        return Capability("unavailable", "Unsupported or unavailable content in $unsupported selected track(s).", resource)
    }
    if (operation.id == FeatureId.COMPARE && !state.sameTrack) return Capability("needs_input", "Declare that these are variants of the same track.", resource)
    val prefixes = items.mapNotNull { state.probes[it.id]?.rate?.takeIf { rate -> rate > 0 }?.let { rate ->
        val budget = 8_640_000L / rate
        val duration = state.probes[it.id]?.frames?.divide(BigInteger.valueOf(rate))?.min(BigInteger.valueOf(budget))?.toLong()
        duration ?: budget
    } }
    val prefix = prefixes.minOrNull()?.coerceAtMost(180)
    return Capability("available", if (unsupported > 0) "Ready with $unsupported explicit unsupported/unavailable track outcome(s)."
        else "Codec verified; final decode and resource admission still apply.", resource, prefix)
}

/** A04 serial acquisition/probing only. A05 owns persisted jobs/service/history.
 * The bounded queue keeps only the most recent requested selection. A replaced
 * selection cancels its provider/native work and cannot publish stale callbacks.
 */
class WorkspaceInput(context: Context, private val changed: (WorkspaceState) -> Unit) : AutoCloseable {
    private val app = context.applicationContext
    private val main = Handler(Looper.getMainLooper())
    private val worker = ThreadPoolExecutor(1, 1, 0, TimeUnit.MILLISECONDS,
        ArrayBlockingQueue<Runnable>(1), { r -> Thread(r, "alfred-input") }, ThreadPoolExecutor.DiscardOldestPolicy())
    private val native = NativeClient()
    private val staging = File(app.filesDir, "input-snapshots")
    private val outputs = File(app.filesDir, "input-probes").apply { mkdirs() }
    private val store = InputStore(staging)
    private var generation = 0L
    private var cancellation = InputCancellation()
    private var closed = false
    @Volatile private var grants: SafSelection? = null
    private var state = WorkspaceState()

    @Synchronized fun featureInputs(): FeatureInputs? {
        val selection = state.selection ?: return null
        if (state.busy || selection.incomplete) return null
        val probes = state.probes.toMap()
        val owner = grants ?: return null
        return FeatureInputs(selection, probes, { item, attempt, cancel ->
            check(Looper.myLooper() != Looper.getMainLooper()) { "Acquisition must run off main" }
            if (selection.items.none { it.id == item.id }) throw InputFailure("input_changed")
            val hash = probes[item.id]?.hash ?: throw InputFailure(probes[item.id]?.reason ?: "input_missing")
            store.acquire(attempt, item.declaredBytes, cancel, hash) {
                val descriptor = app.contentResolver.openFileDescriptor(item.uri, "r", cancel.signal) ?: throw InputFailure("input_missing")
                android.os.ParcelFileDescriptor.AutoCloseInputStream(descriptor)
            }
        }, { owner.retain() })
    }

    @Synchronized fun select(result: PickerResult) {
        if (closed) return
        cancellation.cancel()
        generation++
        val current = generation
        val cancel = InputCancellation().also { cancellation = it }
        state = WorkspaceState(busy = true, notice = "Reading selected documents…")
        changed(state)
        worker.execute {
            SharedJobs.get(app).awaitReady()
            while (SharedJobs.get(app).busy()) { cancel.check(); Thread.sleep(250) }
            val previous = grants
            grants = null
            val acquired = SafSelection(app.contentResolver)
            try {
                val selection = acquired.read(result, current, cancel)
                cancel.check()
                acquired.adoptFrom(previous, result)
                previous?.close()
                grants = acquired
                var next = WorkspaceState(selection, busy = !selection.incomplete,
                    notice = if (selection.incomplete) "selection_limit: folder listing stopped at 512 records or 32 candidates. Select a smaller set and confirm it."
                    else "Copying and probing one document at a time…")
                publish(current, next)
                if (!selection.incomplete) {
                    for (item in selection.items) {
                        cancel.check()
                        val capability = try { probe(selection, item, cancel) } catch (error: InputFailure) {
                            if (error.code == "cancelled") throw error
                            ProbeCapability(item.id, "unavailable", error.code)
                        }
                        next = next.copy(probes = next.probes + (item.id to capability))
                        publish(current, next)
                    }
                    publish(current, next.copy(busy = false, notice = "Input checks complete. Header support does not verify decoded PCM."))
                }
            } catch (error: Exception) {
                acquired.close()
                if (grants === acquired) grants = null
                publish(current, WorkspaceState(notice = failureCode(error)))
            } finally {
                previous?.close()
                if (!isCurrent(current)) { acquired.close(); if (grants === acquired) grants = null }
                if (isClosed()) { grants?.close(); grants = null; native.close() }
            }
        }
    }
    @Synchronized private fun isCurrent(current: Long) = !closed && current == generation
    @Synchronized private fun isClosed() = closed
    private fun publish(current: Long, value: WorkspaceState) {
        main.post { synchronized(this) {
            if (isCurrent(current)) { state = value.copy(sameTrack = state.sameTrack); changed(state) }
        } }
    }
    @Synchronized fun assertSameTrack(value: Boolean) { state = state.copy(sameTrack = value); changed(state) }
    @Synchronized fun confirmFolder(ids: Set<String>) {
        val selection = state.selection ?: return
        val items = selection.items.filter { it.id in ids }
        if (!selection.incomplete || items.isEmpty() || items.size >= 32) return
        // A deliberate smaller direct selection uses the existing tree-grant URI.
        select(PickerResult(items.map { it.uri }, 0, false))
    }
    @Synchronized fun cancel() { cancellation.cancel(); state = state.copy(notice = "Cancellation requested; input remains busy until its reader exits."); changed(state) }
    @Synchronized override fun close() {
        if (closed) return
        closed = true; cancellation.cancel(); generation++
        // Queue a final grant release after all provider reads have exited.
        worker.queue.clear()
        worker.execute { grants?.close(); grants = null; native.close() }
        worker.shutdown()
    }

    private fun failureCode(error: Exception) = when (error) {
        is InputFailure -> error.code
        is SecurityException -> "permission_denied"
        is java.io.FileNotFoundException -> "input_missing"
        is android.os.OperationCanceledException -> "cancelled"
        else -> "io_error"
    }
    @Suppress("UNCHECKED_CAST")
    private fun obj(value: Any?) = value as Map<String, Any?>
    private fun response(bytes: ByteArray): Map<String, Any?> {
        val result = obj(ExactJson.parse(bytes))
        if (result["ok"] != true) throw InputFailure(obj(result["error"])["code"] as String)
        return obj(result["value"])
    }
    private fun probe(selection: InputSelection, item: InputItem, cancel: InputCancellation): ProbeCapability {
        val attempt = UUID.randomUUID().toString()
        val snapshot = store.acquire(attempt, item.declaredBytes, cancel) {
            // ParcelFileDescriptor.AutoCloseInputStream works for pipes and seekable files.
            val descriptor = app.contentResolver.openFileDescriptor(item.uri, "r", cancel.signal)
                ?: throw InputFailure("input_missing")
            android.os.ParcelFileDescriptor.AutoCloseInputStream(descriptor)
        }
        var pin: AutoCloseable? = null
        var handle: Long? = null
        var output: OwnedDirectory? = null
        var outputPin: AutoCloseable? = null
        var sourceReleased = true
        try {
            store.admitWrite(0)
            output = OwnedDirectory.create(outputs, attempt)
            val memory = ActivityManager.MemoryInfo()
            (app.getSystemService(Context.ACTIVITY_SERVICE) as ActivityManager).getMemoryInfo(memory)
            val request = JSONObject().put("version", 1).put("kind", "probe").put("feature", JSONObject.NULL)
                .put("job_id", UUID.randomUUID()).put("attempt_id", attempt).put("selection_id", selection.id)
                .put("generation", selection.generation).put("item_ids", JSONArray().put(item.id))
                .put("leases", JSONArray().put(JSONObject().put("id", snapshot.id).put("attempt_id", attempt)
                    .put("path", snapshot.file.absolutePath).put("bytes", snapshot.bytes).put("sha256", snapshot.sha256).put("name", item.name)))
                .put("scope", JSONObject().put("kind", "full")).put("deadline_ms", 120000)
                .put("output_dir", output.file.absolutePath).put("available_memory_bytes", memory.availMem)
                .put("low_memory", memory.lowMemory).put("reserved_output_bytes", InputStore.PROBE_OUTPUT)
            cancel.check()
            pin = snapshot.retain()
            outputPin = output.retain()
            val started = response(native.start(request.toString().toByteArray(Charsets.UTF_8)).get())
            handle = (started["handle"] as BigInteger).checkedPositiveLong()
            sourceReleased = false
            while (true) {
                cancel.check()
                val poll = response(native.poll(handle).get())
                val terminal = poll["terminal"]
                if (terminal != null) {
                    sourceReleased = poll["leases_released"] == true
                    if (!sourceReleased) throw InputFailure("io_error")
                    val completed = obj(terminal)
                    if (completed["status"] != "completed") throw InputFailure(completed["status"] as? String ?: "io_error")
                    val summary = obj(obj(completed["result"])["summary"])
                    val technical = summary["technical"] as? Map<*, *>
                    val codec = technical?.get("codec") as? String
                    val supported = summary["status"] == "available" && codec?.lowercase() in setOf("pcm", "flac", "alac")
                    val reason = (summary["reason"] as? Map<*, *>)?.get("code") as? String
                    return ProbeCapability(item.id, if (supported) "available" else "unavailable", reason ?: if (supported) "codec_supported" else "unsupported_codec",
                        snapshot.sha256, (technical?.get("sample_rate") as? BigInteger)?.checkedPositiveLong(), technical?.get("declared_frames") as? BigInteger)
                }
                Thread.sleep(250)
            }
        } finally {
            try {
                if (handle != null) {
                    // Close detaches lookup, but does not grant permission to delete.
                    response(native.close(handle).get())
                    while (!sourceReleased) {
                        val active = response(native.describe().get())["active_handle"] as? BigInteger
                        if (active?.toLong() != handle) sourceReleased = true else Thread.sleep(250)
                    }
                }
            } finally {
                snapshot.close(); output?.close()
                if (sourceReleased) {
                    pin?.close(); outputPin?.close()
                }
            }
            // If transport fails before proof of worker release, preserve its
            // snapshot/output rather than deleting a possibly live native input.
        }
    }
}
