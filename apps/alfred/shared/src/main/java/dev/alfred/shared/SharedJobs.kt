package dev.alfred.shared

import android.content.Context
import android.content.Intent
import android.os.SystemClock
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.util.UUID
import java.util.concurrent.CompletableFuture
import java.util.concurrent.Executors
import java.util.concurrent.ArrayBlockingQueue
import java.util.concurrent.ThreadPoolExecutor
import java.util.concurrent.TimeUnit
import java.util.concurrent.RejectedExecutionException

data class JobRecord(val jobId: String, val attemptId: String, val feature: String,
                     val items: List<String>, val options: String, val createdMs: Long,
                     val state: String = "queued", val error: String? = null, val cancelRequested: Boolean = false) {
    val terminal get() = state in setOf("completed", "cancelled", "failed", "interrupted")
    fun json() = JSONObject().put("version", 1).put("job_id", jobId).put("attempt_id", attemptId)
        .put("feature_id", feature).put("item_ids", JSONArray(items)).put("options", JSONObject(options))
        .put("created_ms", createdMs).put("state", state).put("error", error ?: JSONObject.NULL)
        .put("cancel_requested", cancelRequested)
    companion object {
        fun from(json: JSONObject): JobRecord {
            if (json.getInt("version") != 1) throw InputFailure("unsupported_version")
            val items = json.getJSONArray("item_ids")
            return JobRecord(json.getString("job_id"), json.getString("attempt_id"), json.getString("feature_id"),
                (0 until items.length()).map { items.getString(it) }, json.getJSONObject("options").toString(),
                json.getLong("created_ms"), json.getString("state"), json.optString("error").takeIf { it != "null" }, json.getBoolean("cancel_requested"))
        }
    }
}

data class JobProgress(val attemptId: String, val itemId: String?, val phase: String,
                       val pass: Int?, val frames: String?, val expectedFrames: String?)

/** Feature work closes all native/source leases before returning. The shared worker
 * owns scheduling, never a feature-specific report or an Activity. */
fun interface FeatureWork { fun run(context: JobContext) }

class JobContext internal constructor(val record: JobRecord, val cancellation: InputCancellation,
    val results: ResultStore, private val jobs: SharedJobs) {
    private val started = SystemClock.elapsedRealtime()
    val payloads = mutableListOf<ResultDescriptor>()
    val outcomes = JSONArray()
    fun check() {
        if (SystemClock.elapsedRealtime() - started >= 30 * 60 * 1000) throw InputFailure("interrupted")
        cancellation.check()
    }
    fun phase(state: String) { check(); jobs.phase(record.attemptId, state) }
    fun progress(item: String?, phase: String, pass: Int? = null, frames: String? = null, expected: String? = null) {
        check()
        require(item == null || item in record.items)
        jobs.progress(JobProgress(record.attemptId, item, phase, pass, frames, expected))
    }
    fun persist(source: File, kind: String, version: String, artifact: Boolean = false): ResultDescriptor {
        check()
        val descriptor = results.import(record.attemptId, source, kind, version, artifact)
        payloads.add(descriptor)
        return descriptor
    }
    fun outcome(item: String, status: String, diagnostic: String? = null) {
        require(item in record.items)
        outcomes.put(JSONObject().put("item_id", item).put("status", status).put("diagnostic", diagnostic ?: JSONObject.NULL))
    }
}

class SharedJobs private constructor(private val app: Context) {
    private val root = File(app.filesDir, "shared-jobs")
    val results = ResultStore(File(app.filesDir, "job-results"), reservationChanged = { InputStore.resultReservations = it })
    private val control = ThreadPoolExecutor(1, 1, 0, TimeUnit.MILLISECONDS, ArrayBlockingQueue<Runnable>(16),
        { Thread(it, "alfred-job-control") }, ThreadPoolExecutor.AbortPolicy())
    private val worker = Executors.newSingleThreadExecutor { Thread(it, "alfred-job-worker") }
    private val records = linkedMapOf<String, JobRecord>()
    private data class Pending(val record: JobRecord, val reserve: Long, val work: FeatureWork, val leases: List<AutoCloseable>)
    private val pending = ArrayDeque<Pending>()
    private var active: Pending? = null
    @Volatile private var cancellation: InputCancellation? = null
    @Volatile private var latest: JobProgress? = null
    @Volatile private var stopping = false
    @Volatile private var forced: String? = null
    @Volatile private var published = emptyList<JobRecord>()
    @Volatile private var occupied = false
    @Volatile private var activeSince = 0L
    @Volatile private var foregroundReady = false
    @Volatile private var nativeReleaseUnknown = false
    private val initialized = CompletableFuture<Unit>()
    init {
        control.execute {
            try {
                if (!root.isDirectory && !root.mkdirs()) throw InputFailure("storage_full")
                root.listFiles().orEmpty().filter { it.name.endsWith(".new") }.forEach { it.delete() }
                results.recover()
                SafSelection.recoverGrants(app)
                root.listFiles().orEmpty().filter { it.name.endsWith(".json") || it.name.endsWith(".json.bak") }
                    .map { it.name.removeSuffix(".bak") }.distinct().forEach { name ->
                        var record = JobRecord.from(readJson(File(root, name)))
                        // A committed result survived even if the process died before the state write.
                        val committed = results.entries().firstOrNull { it.getString("attempt_id") == record.attemptId }
                        if (!record.terminal) record = record.copy(state = committed?.getString("state") ?: "interrupted", error = if (committed == null) "interrupted" else null)
                        save(record)
                    }
                // Process initialization only: these are the two app-owned A04 roots.
                listOf("input-snapshots", "input-probes").forEach { name ->
                    val dir = File(app.filesDir, name)
                    dir.listFiles().orEmpty().forEach { if (!it.deleteRecursively()) throw InputFailure("io_error") }
                }
                initialized.complete(Unit)
            } catch (error: Throwable) { initialized.completeExceptionally(error) }
        }
    }
    @Synchronized private fun save(record: JobRecord) {
        atomicJson(File(root, record.attemptId + ".json"), record.json())
        records[record.attemptId] = record
        // Bounded small job journal, including failed/cancelled attempts.
        records.values.filter { it.terminal }.sortedBy { it.createdMs }.dropLast(32).forEach {
            records.remove(it.attemptId); File(root, it.attemptId + ".json").delete()
        }
        published = records.values.toList()
    }
    fun awaitReady() { initialized.get() }
    /** Invoke from a visible user action. Future completion never requires a UI join. */
    fun submit(feature: String, items: List<String>, options: JSONObject, reserveBytes: Long,
               work: FeatureWork, jobId: String = UUID.randomUUID().toString(), retainGrants: () -> AutoCloseable? = { null }): CompletableFuture<JobRecord> {
        val future = CompletableFuture<JobRecord>()
        val optionsCopy = options.toString()
        val itemsCopy = items.toList()
        try { control.execute {
            var grantLease: AutoCloseable? = null
            try {
                initialized.get()
                require(feature.length in 1..64 && itemsCopy.size in 1..32 && itemsCopy.distinct().size == itemsCopy.size)
                require(itemsCopy.all { it.length in 1..128 } && optionsCopy.toByteArray().size <= 64 * 1024)
                UUID.fromString(jobId)
                val record = JobRecord(jobId, UUID.randomUUID().toString(), feature, itemsCopy, optionsCopy, System.currentTimeMillis())
                synchronized(this) {
                    if (stopping || nativeReleaseUnknown || pending.size >= if (active == null) 3 else 2) throw InputFailure("busy")
                    grantLease = retainGrants()
                    save(record)
                    pending.addLast(Pending(record, reserveBytes, work, listOfNotNull(grantLease)))
                    occupied = true
                }
                try { app.startForegroundService(Intent(app, JobService::class.java)) }
                catch (_: RuntimeException) { reject(record.attemptId, "service_start_rejected") }
                future.complete(snapshot().firstOrNull { it.attemptId == record.attemptId } ?: record)
            } catch (error: Throwable) { grantLease?.close(); future.completeExceptionally(error) }
        } } catch (_: RejectedExecutionException) { future.completeExceptionally(InputFailure("busy")) }
        return future
    }
    fun snapshot(): List<JobRecord> = published
    fun progressSnapshot(): JobProgress? = latest
    @Synchronized internal fun progress(event: JobProgress) {
        if (active?.record?.attemptId == event.attemptId && records[event.attemptId]?.terminal == false) latest = event
    }
    @Synchronized internal fun phase(attempt: String, state: String) {
        val old = records[attempt] ?: return
        if (active?.record?.attemptId != attempt || old.terminal) return
        val allowed = mapOf("queued" to "acquiring", "acquiring" to "running", "running" to "finalizing")
        require(allowed[old.state] == state)
        save(old.copy(state = state))
    }
    fun cancel(attempt: String) { control.execute { synchronized(this) {
        val queued = pending.firstOrNull { it.record.attemptId == attempt }
        if (queued != null) { pending.remove(queued); queued.leases.forEach { it.close() }; save(queued.record.copy(state = "cancelled", cancelRequested = true)); occupied = active != null || pending.isNotEmpty(); return@synchronized }
        val record = records[attempt] ?: return@synchronized
        if (!record.terminal && active?.record?.attemptId == attempt) {
            save(record.copy(cancelRequested = true)); cancellation?.cancel()
        }
    } } }
    @Synchronized private fun reject(attempt: String, code: String) {
        pending.firstOrNull { it.record.attemptId == attempt }?.let {
            pending.remove(it); it.leases.forEach { lease -> lease.close() }; save(it.record.copy(state = "failed", error = code))
            occupied = active != null || pending.isNotEmpty()
        }
    }
    internal fun promoted() { foregroundReady = true; control.execute { launchNext() } }
    @Synchronized private fun launchNext() {
        if (active != null || stopping || !foregroundReady) return
        val next = pending.removeFirstOrNull() ?: return
        active = next; forced = null; latest = null
        activeSince = SystemClock.elapsedRealtime()
        val cancel = InputCancellation().also { cancellation = it }
        worker.execute {
            val context = JobContext(next.record, cancel, results, this)
            var terminal = "completed"; var code: String? = null
            try {
                results.reserve(next.record.attemptId, next.reserve)
                phase(next.record.attemptId, "acquiring")
                next.work.run(context)
                context.check()
                synchronized(this) {
                    if (records.getValue(next.record.attemptId).state != "finalizing") throw InputFailure("invalid_request")
                }
            } catch (error: Throwable) {
                code = if (error is InputFailure) error.code else "host_error"
                terminal = when (code) { "cancelled" -> "cancelled"; "interrupted", "native_release_unconfirmed" -> "interrupted"; else -> "failed" }
                if (code == "native_release_unconfirmed") { nativeReleaseUnknown = true; interrupt(code) }
            }
            try {
                next.leases.forEach { it.close() }
                synchronized(this) {
                    forced?.let { terminal = "interrupted"; code = it }
                    val final = records.getValue(next.record.attemptId).copy(state = terminal, error = code)
                    results.commit(final, context.payloads, context.outcomes)
                    save(final)
                }
            } catch (_: Throwable) {
                synchronized(this) { save(records.getValue(next.record.attemptId).copy(state = "failed", error = "storage_full")) }
            } finally {
                results.releaseReservation(next.record.attemptId)
                synchronized(this) { active = null; activeSince = 0; cancellation = null; latest = null; occupied = nativeReleaseUnknown || pending.isNotEmpty(); if (stopping && !nativeReleaseUnknown) stopping = false }
                control.execute { launchNext() }
            }
        }
    }
    /** Platform timeout/destruction: request cancellation; never join a reader/native worker. */
    fun interrupt(code: String = "interrupted") {
        stopping = true; forced = code
        control.execute { synchronized(this) {
        cancellation?.cancel()
        pending.toList().forEach { it.leases.forEach { lease -> lease.close() }; save(it.record.copy(state = "interrupted", error = code)) }; pending.clear()
        occupied = nativeReleaseUnknown || active != null; if (!occupied) stopping = false
    } } }
    fun busy() = occupied
    fun running() = activeSince != 0L
    fun releaseUnconfirmed() = nativeReleaseUnknown
    fun expired() = activeSince != 0L && occupied && SystemClock.elapsedRealtime() - activeSince >= 30 * 60 * 1000
    internal fun serviceDetached() { foregroundReady = false; control.execute { if (!busy()) stopping = false } }
    companion object {
        @Volatile private var instance: SharedJobs? = null
        fun get(context: Context): SharedJobs = instance ?: synchronized(this) {
            instance ?: SharedJobs(context.applicationContext).also { instance = it }
        }
    }
}
