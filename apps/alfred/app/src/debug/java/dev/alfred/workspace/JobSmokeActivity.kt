package dev.alfred.workspace

import android.app.Activity
import android.os.Bundle
import dev.alfred.shared.*
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.io.ByteArrayInputStream
import java.util.UUID
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean

/** Generated-only lifecycle/failure harness. No release entry point. */
class JobSmokeActivity : Activity() {
    override fun onCreate(state: Bundle?) {
        super.onCreate(state)
        val mode = intent.getStringExtra("mode") ?: "controls"
        Thread({
            val receipt = File(filesDir, "job-smoke.json")
            try {
                val jobs = SharedJobs.get(this)
                jobs.awaitReady()
                when (mode) {
                    "controls" -> controls(jobs)
                    "recover" -> {
                        val attempt = File(filesDir, "kill-attempt").readText()
                        val job = jobs.snapshot().first { it.attemptId == attempt }
                        check(job.state == if (intent.getBooleanExtra("committed", false)) "completed" else "interrupted")
                        check(File(filesDir, "input-snapshots").listFiles().orEmpty().isEmpty())
                        check(File(filesDir, "input-probes").listFiles().orEmpty().isEmpty())
                        File(filesDir, "job-recovery.json").writeText(JSONObject().put("passed", true).put("state", job.state).toString())
                    }
                    "denied" -> {
                        Thread.sleep(6000) // Harness moves to Home before the attempted start.
                        val job = jobs.submit("test", listOf(UUID.randomUUID().toString()), JSONObject(), 4096, FeatureWork { c ->
                            c.phase("running"); c.phase("finalizing")
                        }).get(10, TimeUnit.SECONDS)
                        waitFor { jobs.snapshot().first { it.attemptId == job.attemptId }.terminal }
                        val final = jobs.snapshot().first { it.attemptId == job.attemptId }
                        if (android.os.Build.VERSION.SDK_INT >= 31) check(final.error == "service_start_rejected")
                        File(filesDir, "job-denied.json").writeText(JSONObject().put("passed", true).put("state", final.state).put("error", final.error).toString())
                    }
                    else -> killWork(jobs, mode)
                }
                receipt.writeText(JSONObject().put("passed", true).put("mode", mode).put("api", android.os.Build.VERSION.SDK_INT).toString())
            } catch (error: Throwable) {
                receipt.writeText(JSONObject().put("passed", false).put("mode", mode).put("error", error.stackTraceToString()).toString())
            }
            runOnUiThread { finish() }
        }, "alfred-jobs-smoke").start()
    }
    private fun waitFor(block: () -> Boolean) {
        val deadline = System.nanoTime() + 30_000_000_000
        while (!block()) { check(System.nanoTime() < deadline) { "bounded wait expired" }; Thread.sleep(50) }
    }
    private fun expect(code: String, action: () -> Unit) {
        try { action(); error("Expected $code") } catch (error: InputFailure) { check(error.code == code) }
    }
    private fun controls(jobs: SharedJobs) {
        val testRoot = File(cacheDir, "job-storage-" + UUID.randomUUID()).apply { mkdir() }
        val store = ResultStore(testRoot, { Long.MAX_VALUE }, maxJobs = 2, quota = 65536)
        val source = File(cacheDir, "generated-payload").apply { writeText("{\"integer\":18446744073709551615,\"nullable\":null,\"future\":true}") }
        fun stored(): Pair<String, ResultDescriptor> {
            val attempt = UUID.randomUUID().toString()
            store.reserve(attempt, 4096)
            val descriptor = store.import(attempt, source, "generated", "future")
            store.commit(JobRecord(UUID.randomUUID().toString(), attempt, "test", listOf("one"), "{}", System.currentTimeMillis(), "completed"), listOf(descriptor), JSONArray())
            return attempt to descriptor
        }
        val first = stored()
        val pin = store.pin(first.first)
        expect("busy") { store.delete(first.first) }
        check(store.dispatch(first.second, setOf("generated" to "v1")) == "unsupported_version")
        check(store.validate(first.first, first.second).readBytes().contentEquals(source.readBytes()))
        expect("invalid_request") { store.validate(first.first, first.second.copy(path = "../outside")) }
        expect("invalid_payload") { store.validate(first.first, first.second.copy(sha256 = "bad")) }
        val second = stored(); stored()
        check(store.entries().size == 2 && store.entries().none { it.getString("attempt_id") == second.first })
        pin.close(); store.delete(first.first)
        val orphan = File(testRoot, UUID.randomUUID().toString()).apply { mkdir(); File(this, "lost.partial").writeText("partial") }
        store.recover(); check(!orphan.exists())
        expect("storage_full") { ResultStore(File(cacheDir, "no-space"), { 0 }).reserve(UUID.randomUUID().toString(), 1) }
        expect("storage_full") { store.reserve(UUID.randomUUID().toString(), 65537) }
        testRoot.deleteRecursively()

        val blocked = AtomicBoolean(true)
        val entered = AtomicBoolean(false)
        var oldContext: JobContext? = null
        val item = UUID.randomUUID().toString()
        val active = jobs.submit("generated", listOf(item), JSONObject().put("scope", "full"), 4096, FeatureWork { c ->
            oldContext = c; c.phase("running")
            c.persist(source, "generated", "future"); c.outcome(item, "analyzed")
            entered.set(true)
            while (blocked.get()) Thread.sleep(25) // A blocked reader models delayed cooperative cancel.
            c.check(); c.phase("finalizing")
        }).get(10, TimeUnit.SECONDS)
        waitFor { entered.get() }
        val opened = java.util.concurrent.atomic.AtomicInteger(0)
        val work = FeatureWork { c -> opened.incrementAndGet(); c.phase("running"); c.phase("finalizing") }
        val queued = jobs.submit("queued", listOf(item), JSONObject(), 4096, work).get(10, TimeUnit.SECONDS)
        val another = jobs.submit("queued", listOf(item), JSONObject(), 4096, work).get(10, TimeUnit.SECONDS)
        try { jobs.submit("excess", listOf(item), JSONObject(), 4096, work).get(10, TimeUnit.SECONDS); error("Expected busy") }
        catch (error: java.util.concurrent.ExecutionException) { check((error.cause as InputFailure).code == "busy") }
        jobs.cancel(queued.attemptId); jobs.cancel(active.attemptId)
        waitFor { jobs.snapshot().first { it.attemptId == active.attemptId }.cancelRequested }
        check(jobs.busy() && opened.get() == 0)
        blocked.set(false)
        waitFor { !jobs.busy() }
        check(jobs.snapshot().first { it.attemptId == active.attemptId }.state == "cancelled")
        check(jobs.results.entries().first { it.getString("attempt_id") == active.attemptId }.getJSONArray("payloads").length() == 1)
        check(opened.get() == 1 && jobs.snapshot().first { it.attemptId == another.attemptId }.state == "completed")
        val retry = jobs.submit("generated", listOf(item), JSONObject(), 4096, FeatureWork { c ->
            c.phase("running")
            expect("cancelled") { oldContext!!.progress(item, "stale", frames = "999") }
            c.progress(item, "decoding", 1, "1", null)
            c.phase("finalizing")
        }, active.jobId).get(10, TimeUnit.SECONDS)
        check(retry.attemptId != active.attemptId)
        waitFor { !jobs.busy() }
        check(jobs.snapshot().first { it.attemptId == retry.attemptId }.state == "completed")
        // Returning at decode completion without finalization is a host failure.
        val invalid = jobs.submit("invalid", listOf(item), JSONObject(), 4096, FeatureWork { it.phase("running") }).get(10, TimeUnit.SECONDS)
        waitFor { !jobs.busy() }
        check(jobs.snapshot().first { it.attemptId == invalid.attemptId }.state == "failed")
        val share = jobs.results.entries().first { it.getString("attempt_id") == active.attemptId }.getJSONArray("payloads").getJSONObject(0)
        val shared = jobs.results.share(this, active.attemptId, ResultDescriptor.from(share))
        check(shared.authority == packageName + ".results")
        contentResolver.openInputStream(shared)!!.use { check(it.readBytes().contentEquals(source.readBytes())) }
        jobs.results.export(this, active.attemptId, ResultDescriptor.from(share), shared)
        val transfer = ResultTransfer(this, jobs.results)
        try {
            transfer.export(active.attemptId, ResultDescriptor.from(share), android.net.Uri.parse("content://missing.generated.provider/result")).get(10, TimeUnit.SECONDS)
            error("Expected export failure")
        } catch (error: java.util.concurrent.ExecutionException) { check((error.cause as InputFailure).code == "io_error") }
        check(jobs.results.load(active.attemptId).getJSONArray("payloads").length() == 1)
        jobs.results.delete(active.attemptId)
        contentResolver.openInputStream(shared)!!.use { check(it.readBytes().contentEquals(source.readBytes())) }
        File(filesDir, "job-controls.json").writeText(JSONObject().put("passed", true).put("checks", 23).toString())
    }
    private fun mark(attempt: String, phase: String) {
        File(filesDir, "kill-attempt").writeText(attempt)
        File(filesDir, "job-marker.json").writeText(JSONObject().put("phase", phase).put("attempt_id", attempt).toString())
    }
    private fun killWork(jobs: SharedJobs, mode: String) {
        val item = UUID.randomUUID().toString()
        val job = jobs.submit("lifecycle", listOf(item), JSONObject().put("scope", "full"), 4L * 1024 * 1024, FeatureWork { c ->
            if (mode == "copy") {
                val store = InputStore(File(filesDir, "input-snapshots"))
                val stream = object : java.io.InputStream() {
                    override fun read(): Int = error("bulk reads only")
                    override fun read(buffer: ByteArray, offset: Int, count: Int): Int {
                        mark(c.record.attemptId, "copy")
                        Thread.sleep(30000)
                        c.check(); return -1
                    }
                }
                store.acquire(c.record.attemptId, null, c.cancellation) { stream }.use { }
            }
            c.phase("running")
            if (mode == "native") nativeWork(c)
            if (mode == "background") {
                mark(c.record.attemptId, "background")
                repeat(150) { Thread.sleep(100); c.check() }
            }
            if (mode in setOf("running", "timeout")) {
                mark(c.record.attemptId, mode)
                while (true) { Thread.sleep(100); c.check() }
            }
            val payload = File(cacheDir, "kill-generated").apply { writeText("{\"generated\":true}") }
            c.persist(payload, "generated", "v1"); c.outcome(item, "analyzed")
            c.phase("finalizing")
            if (mode == "finalizing") {
                mark(c.record.attemptId, mode)
                while (true) { Thread.sleep(100); c.check() }
            }
        }).get(10, TimeUnit.SECONDS)
        if (mode == "completed" || mode == "background") {
            waitFor { !jobs.busy() }
            check(jobs.snapshot().first { it.attemptId == job.attemptId }.state == "completed")
            mark(job.attemptId, "completed")
        } else {
            waitFor { !jobs.busy() }
            if (mode == "timeout") {
                check(jobs.snapshot().first { it.attemptId == job.attemptId }.state == "interrupted")
                File(filesDir, "job-timeout.json").writeText(JSONObject().put("passed", true).toString())
            }
        }
    }
    private fun nativeWork(c: JobContext) {
        val store = InputStore(File(filesDir, "input-snapshots"))
        val wave = File(cacheDir, "lifecycle-generated.wav")
        val frames = 48000 * 180
        val header = java.nio.ByteBuffer.allocate(44).order(java.nio.ByteOrder.LITTLE_ENDIAN)
            .put("RIFF".toByteArray()).putInt(36 + frames * 2).put("WAVEfmt ".toByteArray()).putInt(16)
            .putShort(1).putShort(1).putInt(48000).putInt(96000).putShort(2).putShort(16).put("data".toByteArray()).putInt(frames * 2).array()
        wave.outputStream().use { out -> out.write(header); val block = ByteArray(96000); repeat(180) { out.write(block) } }
        store.acquire(c.record.attemptId, wave.length(), c.cancellation) { wave.inputStream() }.use { snapshot ->
            OwnedDirectory.create(File(filesDir, "input-probes"), c.record.attemptId).use { output ->
                NativeClient().use { client ->
                    val request = JSONObject().put("version", 1).put("kind", "feature").put("feature", "spectrogram")
                        .put("job_id", c.record.jobId).put("attempt_id", c.record.attemptId).put("selection_id", UUID.randomUUID())
                        .put("generation", 1).put("item_ids", JSONArray(c.record.items))
                        .put("leases", JSONArray().put(JSONObject().put("id", snapshot.id).put("attempt_id", c.record.attemptId)
                            .put("path", snapshot.file.absolutePath).put("bytes", snapshot.bytes).put("sha256", snapshot.sha256).put("name", "generated.wav")))
                        .put("scope", JSONObject().put("kind", "full")).put("deadline_ms", 120000)
                        .put("output_dir", output.file.absolutePath).put("available_memory_bytes", 2147483648L)
                        .put("low_memory", false).put("reserved_output_bytes", 4L * 1024 * 1024)
                    c.runNative(request, snapshot, output, client) { handle ->
                        waitFor {
                            val envelope = ExactJson.parse(client.poll(handle).get()) as Map<*, *>
                            val value = envelope["value"] as Map<*, *>
                            (value["progress"] as? Map<*, *>)?.get("phase") == "decoding"
                        }
                        mark(c.record.attemptId, "native")
                        // Keep the native handle live until the external kill is observed.
                        Thread.sleep(1500)
                    }
                }
            }
        }
        wave.delete()
    }
}
