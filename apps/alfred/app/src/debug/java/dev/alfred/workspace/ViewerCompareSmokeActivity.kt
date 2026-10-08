package dev.alfred.workspace

import android.app.Activity
import android.net.Uri
import android.os.Bundle
import android.graphics.BitmapFactory
import dev.alfred.shared.*
import dev.alfred.spectrogram.SpectrogramWork
import dev.alfred.compare.CompareWork
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.util.UUID
import java.util.concurrent.TimeUnit

/** Generated signals and explicit malformed saved controls only; no policy in Kotlin. */
class ViewerCompareSmokeActivity : Activity() {
    private val checks = JSONArray()
    private lateinit var jobs: SharedJobs
    private var previewAttempt = ""
    private var compareAttempt = ""
    private var oldAttempt = ""
    override fun onCreate(state: Bundle?) {
        super.onCreate(state)
        Thread({
            try {
                jobs = SharedJobs.get(this); jobs.awaitReady()
                spectrogram(); compare()
                File(filesDir, "viewer-compare-smoke.json").writeText(JSONObject().put("passed", true)
                    .put("checks", checks).put("preview_attempt", previewAttempt).put("compare_attempt", compareAttempt).put("old_attempt", oldAttempt).toString())
            } catch (error: Throwable) {
                File(filesDir, "viewer-compare-smoke.json").writeText(JSONObject().put("passed", false)
                    .put("checks", checks).put("error", error.stackTraceToString()).toString())
            }
            runOnUiThread { finish() }
        }, "alfred-viewer-compare-smoke").start()
    }
    private fun waitFor(block: () -> Boolean) {
        val until = System.nanoTime() + 120_000_000_000L
        while (!block()) { check(System.nanoTime() < until); Thread.sleep(50) }
    }
    private fun completed(record: JobRecord, expected: String = "completed"): JSONObject {
        waitFor { jobs.snapshot().first { it.attemptId == record.attemptId }.terminal && !jobs.busy() }
        val final = jobs.snapshot().first { it.attemptId == record.attemptId }
        check(final.state == expected) { final.toString() }
        return jobs.results.load(record.attemptId)
    }
    private fun inputs(names: List<String>, missing: Boolean = false, cancel: Boolean = false): FeatureInputs {
        val items = names.map { InputItem(UUID.randomUUID().toString(), Uri.EMPTY, it, it, null, "generated") }
        val store = InputStore(File(filesDir, "input-snapshots"))
        return FeatureInputs(InputSelection(UUID.randomUUID().toString(), 1, items),
            items.associate { it.id to ProbeCapability(it.id, "available", "generated", rate = 44100) },
            { item, attempt, cancellation ->
                if (cancel) cancellation.cancel()
                if (missing && item == items.last()) throw InputFailure("input_missing")
                store.acquire(attempt, null, cancellation) {
                    when {
                        item.name.startsWith("generated-") -> java.io.ByteArrayInputStream(wave(if (item.name.contains("short")) 18 else 24))
                        item.name.endsWith(".dsf") -> java.io.ByteArrayInputStream("DSD     ".toByteArray())
                        else -> assets.open(item.name)
                    }
                }
            }, { AutoCloseable {} })
    }
    private fun wave(seconds: Int): ByteArray {
        // Same public generated control/repetition as tests/product.rs: 18 seconds
        // supplies the unchanged nine two-second reference segment probes.
        val source = assets.open("clip_noise.wav").use { it.readBytes() }
        check(String(source, 36, 4, Charsets.US_ASCII) == "data")
        val bytes = java.io.ByteArrayOutputStream().apply {
            write(source, 0, 44); repeat(seconds / 2) { write(source, 44, source.size - 44) }
        }.toByteArray()
        java.nio.ByteBuffer.wrap(bytes).order(java.nio.ByteOrder.LITTLE_ENDIAN)
            .putInt(4, bytes.size - 8).putInt(40, bytes.size - 44)
        return bytes
    }
    private fun descriptors(manifest: JSONObject): List<ResultDescriptor> {
        val array = manifest.getJSONArray("payloads")
        return (0 until array.length()).map { ResultDescriptor.from(array.getJSONObject(it)) }
    }
    private fun document(attempt: String, descriptor: ResultDescriptor) =
        JSONObject(jobs.results.validate(attempt, descriptor).readText())
    private fun transfer(attempt: String, descriptor: ResultDescriptor) {
        val original = jobs.results.validate(attempt, descriptor).readBytes()
        val uri = jobs.results.share(this, attempt, descriptor)
        contentResolver.openInputStream(uri)!!.use { check(it.readBytes().contentEquals(original)) }
        jobs.results.export(this, attempt, descriptor, uri)
        contentResolver.openInputStream(uri)!!.use { check(it.readBytes().contentEquals(original)) }
        val intent = ResultTransfer(this, jobs.results).share(attempt, descriptor).get(10, TimeUnit.SECONDS)
        check(intent.type == if (descriptor.kind == "png") "image/png" else "application/json")
    }
    private fun savedFixture(value: JSONObject, kind: String, version: String): CompareWork.SavedProduct {
        return savedFixtureText(value.toString(), kind, version)
    }
    private fun savedFixtureText(value: String, kind: String, version: String): CompareWork.SavedProduct {
        val attempt = UUID.randomUUID().toString()
        val source = File(cacheDir, "viewer-compare-fixture.json").apply { writeText(value) }
        jobs.results.reserve(attempt, source.length() + 65536)
        val descriptor = jobs.results.import(attempt, source, kind, version)
        jobs.results.commit(JobRecord(UUID.randomUUID().toString(), attempt, if (kind == "spectrogram") "spectrogram" else "forensics",
            listOf(UUID.randomUUID().toString()), "{}", System.currentTimeMillis(), "completed"), listOf(descriptor), JSONArray())
        return CompareWork.SavedProduct(attempt, descriptor)
    }
    /** Change only a top-level value; retain every original numeric token. */
    private fun replaceTopValue(text: String, field: String, value: String): String {
        var depth = 0; var quoted = false; var escaped = false
        val marker = "\"$field\":"
        for (index in text.indices) {
            val char = text[index]
            if (!quoted && depth == 1 && text.startsWith(marker, index)) {
                val start = index + marker.length
                var nested = 0; var string = false; var escape = false
                for (end in start until text.length) {
                    val c = text[end]
                    if (string) {
                        if (escape) escape = false else if (c == '\\') escape = true else if (c == '"') string = false
                    } else when (c) {
                        '"' -> string = true
                        '{', '[' -> nested++
                        '}', ']' -> if (nested > 0) nested-- else return text.substring(0, start) + value + text.substring(end)
                        ',' -> if (nested == 0) return text.substring(0, start) + value + text.substring(end)
                    }
                }
                error("Unterminated generated field")
            }
            if (quoted) {
                if (escaped) escaped = false else if (char == '\\') escaped = true else if (char == '"') quoted = false
            } else when (char) { '"' -> quoted = true; '{', '[' -> depth++; '}', ']' -> depth-- }
        }
        error("Missing generated field: $field")
    }
    private fun render(input: CompareWork.SavedProduct, budget: Long, cancel: Boolean = false): JSONObject {
        val item = UUID.randomUUID().toString()
        val record = jobs.submit("spectrogram", listOf(item), JSONObject().put("generated", true), 128L * 1024 * 1024,
            FeatureWork { context ->
                context.phase("running")
                OwnedDirectory.create(File(filesDir, "input-probes"), context.record.attemptId).use { output ->
                    val source = context.results.validate(input.attempt, input.descriptor)
                    val d = input.descriptor
                    val request = JSONObject().put("version", 1).put("kind", "feature").put("feature", "render")
                        .put("job_id", context.record.jobId).put("attempt_id", context.record.attemptId)
                        .put("selection_id", UUID.randomUUID().toString()).put("generation", 1)
                        .put("item_ids", JSONArray().put(item)).put("leases", JSONArray())
                        .put("saved", JSONArray().put(JSONObject().put("root", source.parentFile!!.absolutePath)
                            .put("payload", JSONObject().put("path", d.path).put("kind", d.kind).put("version", d.version).put("bytes", d.bytes).put("sha256", d.sha256))))
                        .put("scope", JSONObject().put("kind", "full")).put("png", true).put("preset", "standard")
                        .put("deadline_ms", 120000).put("available_memory_bytes", 2147483648L).put("low_memory", false)
                        .put("reserved_output_bytes", budget).put("output_dir", output.file.absolutePath)
                    NativeClient().use { client ->
                        val result = context.runNative(request, null, output, client,
                            savedPins = listOf(context.results.pin(input.attempt)), onStarted = {
                                if (cancel) { jobs.cancel(context.record.attemptId); waitFor { jobs.snapshot().first { it.attemptId == context.record.attemptId }.cancelRequested } }
                            })
                        context.adoptNative(result, output)
                    }
                }
                context.phase("finalizing")
            }, retainGrants = { jobs.results.pin(input.attempt) }).get(10, TimeUnit.SECONDS)
        return completed(record, if (cancel) "cancelled" else "completed")
    }
    private fun spectrogram() {
        check(NativeBootstrap.load() == "Native host v1 loaded")
        val input = inputs(listOf("noise16.wav"))
        var wrapper: JSONObject? = null
        for ((preset, dimensions) in listOf("standard" to (1600 to 900), "publication" to (2560 to 1440), "large" to (3840 to 2160))) {
            val record = SpectrogramWork.submit(this, input, JSONObject().put("kind", "prefix").put("seconds", 1), preset).get(10, TimeUnit.SECONDS)
            val manifest = completed(record)
            val list = descriptors(manifest)
            val payload = list.single { it.kind == "spectrogram" }
            wrapper = document(record.attemptId, payload)
            check(wrapper.getJSONObject("measurement").getString("status") == "analyzed")
            val coverage = wrapper.getJSONObject("measurement").getJSONObject("coverage")
            check(coverage.getLong("analyzed_frames") == 24577L && coverage.getString("decoded_pcm_sha256") == "1981c82a788cf0be394f6c6326e6e7576a788076b51e7f7834286cada2b0f0e4") { coverage.toString() }
            val png = list.single { it.kind == "png" }
            val bytes = jobs.results.validate(record.attemptId, png).readBytes()
            val dimensionsBytes = java.nio.ByteBuffer.wrap(bytes, 16, 8)
            check(dimensionsBytes.int == dimensions.first && dimensionsBytes.int == dimensions.second)
            val bitmap = BitmapFactory.decodeByteArray(bytes, 0, bytes.size, BitmapFactory.Options().apply { inSampleSize = 4 })!!
            check(bitmap.width > 0 && bitmap.height > 0); bitmap.recycle()
            // PNG iTXt contains the native source title, calibration method and PCM binding.
            val text = String(bytes, Charsets.ISO_8859_1)
            check(text.contains("noise16.wav") && text.contains("alfred-calibrated-png-v1") && text.contains(coverage.getString("decoded_pcm_sha256")))
            transfer(record.attemptId, png)
            if (preset == "publication") previewAttempt = record.attemptId
            checks.put("spectrogram:$preset:scope/hash/title/calibration/dimensions/decode/exact-export/MIME")
        }
        val old = JSONObject(wrapper!!.toString()).apply { remove("presentation") }
        val saved = savedFixture(old, "spectrogram", "1")
        oldAttempt = saved.attempt
        val rendered = render(saved, 64L * 1024 * 1024)
        check(descriptors(rendered).any { it.kind == "png" })
        checks.put("old-wrapper:absent-presentation/native-render")
        val failed = render(saved, 16L * 1024 * 1024)
        val failedList = descriptors(failed)
        check(failedList.any { it.kind == "spectrogram" } && failedList.none { it.kind == "png" })
        val nativeManifest = document(failed.getString("attempt_id"), failedList.single { it.kind == "alfred-result" })
        check(nativeManifest.getJSONObject("artifact").getString("status") == "failed")
        checks.put("PNG-failure:measurement/wrapper-retained")
        render(saved, 64L * 1024 * 1024, cancel = true)
        checks.put("saved-native-cancellation:released-pins")
        val cancelled = SpectrogramWork.submit(this, inputs(listOf("noise16.wav"), cancel = true), JSONObject().put("kind", "full"), "publication").get()
        completed(cancelled, "cancelled")
        val missing = SpectrogramWork.submit(this, inputs(listOf("noise16.wav"), missing = true), JSONObject().put("kind", "full"), "publication").get()
        completed(missing, "failed")
        check(File(filesDir, "input-snapshots").listFiles().orEmpty().isEmpty())
        check(File(filesDir, "input-probes").listFiles().orEmpty().isEmpty())
        checks.put("spectrogram:acquisition-failure/cancel/cleanup")
    }
    private fun compare() {
        val live = inputs(listOf("generated-a.wav", "generated-b.wav"))
        val high = live.copy(probes = live.selection.items.mapIndexed { index, item ->
            val rate = if (index == 0) 48000L else 96000L
            item.id to ProbeCapability(item.id, "available", "generated", rate = rate, frames = java.math.BigInteger.valueOf(rate * 300))
        }.toMap())
        check(CompareWork.prefixBudget(high) == 90L)
        try { CompareWork.submit(this, high, JSONObject().put("kind", "full"), true); error("full-rate admission bypass") } catch (_: IllegalArgumentException) { }
        try { CompareWork.submit(this, live, JSONObject().put("kind", "prefix").put("seconds", 1), false); error("same-track bypass") } catch (_: IllegalArgumentException) { }
        checks.put("compare:90-second-common-budget/full-admission/same-track")
        val record = CompareWork.submit(this, live, JSONObject().put("kind", "prefix").put("seconds", 18), true).get()
        val manifest = completed(record)
        val list = descriptors(manifest)
        val comparison = list.single { it.kind == "comparison" }
        val report = document(record.attemptId, comparison)
        check(report.getString("status") == "available" && report.getInt("winner_input_index") == 0) { report.toString() }
        check(report.getJSONArray("ranking").getJSONObject(0).getInt("input_index") == 0)
        val products = list.filter { it.kind == "product" }.map { CompareWork.SavedProduct(record.attemptId, it) }
        val coverage = products.map { document(it.attempt, it.descriptor).getJSONObject("measurement_report").getJSONObject("coverage") }
        check(coverage.all { it.getLong("analyzed_frames") == 18 * 44100L && !it.getBoolean("reached_end") })
        check(coverage[0].getString("decoded_pcm_sha256") == coverage[1].getString("decoded_pcm_sha256"))
        checks.put("live-compare:common-scope/exact-PCM/stable-tie")
        fun savedCompare(inputs: List<CompareWork.SavedProduct>, expected: String = "completed"): JSONObject {
            val next = CompareWork.submitSaved(this, inputs, true).get()
            return completed(next, expected)
        }
        val saved = savedCompare(products)
        compareAttempt = saved.getString("attempt_id")
        val savedList = descriptors(saved)
        val savedReport = savedList.single { it.kind == "comparison" }
        check(document(compareAttempt, savedReport).toString() == report.toString())
        transfer(compareAttempt, savedReport)
        jobs.results.delete(record.attemptId)
        savedList.forEach { jobs.results.validate(compareAttempt, it) }
        checks.put("saved-compare:same-Rust-result/exact-export/independent-retention")
        val baseText = jobs.results.validate(compareAttempt, savedList.first { it.kind == "product" }).readText()
        val absent = savedFixtureText(replaceTopValue(baseText, "tool_statistics", "null"), "product", "audio-forensic-product-v1")
        val copied = savedList.filter { it.kind == "product" }.map { CompareWork.SavedProduct(compareAttempt, it) }
        val incompatible = savedCompare(listOf(copied[0], absent))
        val incompatibleReport = document(incompatible.getString("attempt_id"), descriptors(incompatible).single { it.kind == "comparison" })
        check(incompatibleReport.getString("status") == "incompatible" && incompatibleReport.isNull("winner_input_index"))
        check(incompatibleReport.getJSONArray("ranking").getJSONObject(1).isNull("rank"))
        checks.put("saved-compare:absent-tool-domain/incompatible/null-ranks")
        val shortRecord = CompareWork.submit(this, inputs(listOf("generated-a.wav", "generated-short.wav")), JSONObject().put("kind", "prefix").put("seconds", 20), true).get()
        val shortManifest = completed(shortRecord)
        val shortReport = document(shortRecord.attemptId, descriptors(shortManifest).single { it.kind == "comparison" })
        check(shortReport.getString("status") == "incompatible" && shortReport.isNull("winner_input_index")) { shortReport.toString() }
        checks.put("live-compare:shorter-actual-EOF/incompatible")
        for ((names, expected) in listOf(listOf("generated-a.wav", "unsupported.dsf") to "available", listOf("first.dsf", "second.dsf") to "unavailable")) {
            val mixedRecord = CompareWork.submit(this, inputs(names), JSONObject().put("kind", "prefix").put("seconds", 18), true).get()
            val mixed = completed(mixedRecord)
            val mixedReport = document(mixedRecord.attemptId, descriptors(mixed).single { it.kind == "comparison" })
            check(mixedReport.getString("status") == expected)
            if (expected == "unavailable") check(mixedReport.isNull("winner_input_index"))
            checks.put("live-compare:$expected/decoded-unavailable-products")
        }
        val partialRecord = CompareWork.submit(this, live, JSONObject().put("kind", "prefix").put("seconds", 1), true).get()
        val shortUnavailable = completed(partialRecord)
        val abstention = document(partialRecord.attemptId, descriptors(shortUnavailable).single { it.kind == "comparison" })
        check(abstention.getString("status") == "unavailable" && abstention.isNull("winner_input_index"))
        checks.put("live-compare:short-prefix/partial-assessment/no-invented-winner")
        for (field in listOf("product_schema_version", "method_id", "input_domain")) {
            val invalid = when (field) {
                "product_schema_version" -> replaceTopValue(baseText, field, "\"future\"")
                "method_id" -> baseText.replace("\"method_id\":\"python-reference-c6ecce2-v1\"", "\"method_id\":\"future\"")
                else -> baseText.replace("\"input_domain\":\"native lanes converted to f32, three-second RMS/peak histograms\"", "\"input_domain\":\"foreign\"")
            }
            check(invalid != baseText)
            val fixture = savedFixtureText(invalid, "product", "audio-forensic-product-v1")
            val rejected = savedCompare(listOf(copied[0], fixture), "failed")
            check(descriptors(rejected).count { it.kind == "product" } == 2)
            check(jobs.snapshot().first { it.attemptId == rejected.getString("attempt_id") }.error == if (field == "product_schema_version") "unsupported_version" else "invalid_request")
            checks.put("saved-compare:reject-$field/original-products-retained")
        }
        val future = copied[0].copy(descriptor = copied[0].descriptor.copy(version = "future"))
        try { CompareWork.submitSaved(this, listOf(future, copied[1]), true).get(); error("future descriptor admitted") }
        catch (error: java.util.concurrent.ExecutionException) { check((error.cause as InputFailure).code == "unsupported_version") }
        checks.put("saved-compare:future-descriptor-before-admission")
        val queueInputs = listOf(savedFixtureText(baseText, "product", "audio-forensic-product-v1"), savedFixtureText(baseText, "product", "audio-forensic-product-v1"))
        val release = java.util.concurrent.atomic.AtomicBoolean(false)
        val blocker = jobs.submit("compare", listOf(UUID.randomUUID().toString()), JSONObject(), 65536,
            FeatureWork { context ->
                context.phase("running")
                while (!release.get()) { context.check(); Thread.sleep(25) }
                context.phase("finalizing")
            }).get()
        waitFor { jobs.snapshot().first { it.attemptId == blocker.attemptId }.state == "running" }
        val queued = CompareWork.submitSaved(this, queueInputs, true).get()
        try { jobs.results.delete(queueInputs[0].attempt); error("queued input unpinned") }
        catch (error: InputFailure) { check(error.code == "busy") }
        jobs.cancel(queued.attemptId)
        waitFor { jobs.snapshot().first { it.attemptId == queued.attemptId }.terminal }
        queueInputs.forEach { jobs.results.delete(it.attempt) }
        release.set(true); completed(blocker)
        try { CompareWork.submitSaved(this, queueInputs, true).get(); error("deleted input admitted") }
        catch (error: java.util.concurrent.ExecutionException) { check((error.cause as InputFailure).code == "invalid_payload") }
        checks.put("saved-compare:queue-pin/delete-busy/queued-cancel/release/missing-input")
        try { CompareWork.submitSaved(this, listOf(copied[0].copy(descriptor = copied[0].descriptor.copy(bytes = ResultStore.DOCUMENT_LIMIT + 1)), copied[1]), true); error("saved bound bypass") }
        catch (error: InputFailure) { check(error.code == "resource_limit") }
        checks.put("saved-compare:serialized-input-bound")
        val missing = CompareWork.submit(this, inputs(listOf("noise16.wav", "noise16.flac"), missing = true), JSONObject().put("kind", "prefix").put("seconds", 1), true).get()
        val partial = completed(missing, "failed")
        check(descriptors(partial).count { it.kind == "product" } == 1)
        checks.put("live-compare:missing-input/collected-product-retained")
        check(File(filesDir, "input-snapshots").listFiles().orEmpty().isEmpty())
        check(File(filesDir, "input-probes").listFiles().orEmpty().isEmpty())
    }
}
