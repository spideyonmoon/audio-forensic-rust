package dev.alfred.workspace

import android.app.Activity
import android.net.Uri
import android.os.Bundle
import dev.alfred.shared.*
import dev.alfred.forensics.ForensicDocument
import dev.alfred.forensics.ForensicsWork
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.math.BigInteger
import java.util.UUID
import java.util.concurrent.TimeUnit

/** Public generated audio only. Exercises the actual feature worker and saved presentation. */
class FeatureSmokeActivity : Activity() {
    private var actualAttempt = ""
    private var generatedAttempt = ""
    override fun onCreate(state: Bundle?) {
        super.onCreate(state)
        Thread({
            try {
                val checks = forensics()
                File(filesDir, "feature-smoke.json").writeText(JSONObject().put("passed", true).put("checks", checks)
                    .put("actual_attempt", actualAttempt).put("generated_attempt", generatedAttempt).toString())
            } catch (error: Throwable) {
                File(filesDir, "feature-smoke.json").writeText(JSONObject().put("passed", false).put("error", error.stackTraceToString()).toString())
            }
            runOnUiThread { finish() }
        }, "alfred-feature-smoke").start()
    }
    private fun waitFor(block: () -> Boolean) {
        val until = System.nanoTime() + 120_000_000_000L
        while (!block()) { check(System.nanoTime() < until); Thread.sleep(50) }
    }
    private fun forensics(): Int {
        val jobs = SharedJobs.get(this)
        jobs.awaitReady()
        val staging = InputStore(File(filesDir, "input-snapshots"))
        fun item(name: String) = InputItem(UUID.randomUUID().toString(), Uri.EMPTY, name, name, null, "generated")
        val good = item("noise16.wav"); val bad = item("failed.wav"); val unavailable = item("unsupported.dsf")
        val selection = InputSelection(UUID.randomUUID().toString(), 1, listOf(good, bad, unavailable))
        val inputs = FeatureInputs(selection, mapOf(good.id to ProbeCapability(good.id, "available", "generated", rate = 44100),
            bad.id to ProbeCapability(bad.id, "available", "generated", rate = 44100),
            unavailable.id to ProbeCapability(unavailable.id, "unavailable", "unsupported_codec")),
            { selected, attempt, cancellation ->
                if (selected.id == bad.id) throw InputFailure("input_missing")
                staging.acquire(attempt, null, cancellation) { assets.open("noise16.wav") }
            }, { AutoCloseable {} })
        val record = ForensicsWork.submit(this, inputs, JSONObject().put("kind", "prefix").put("seconds", 1)).get(10, TimeUnit.SECONDS)
        actualAttempt = record.attemptId
        waitFor { jobs.snapshot().first { it.attemptId == record.attemptId }.terminal }
        val final = jobs.snapshot().first { it.attemptId == record.attemptId }
        check(final.state == "completed") { final.toString() }
        val manifest = jobs.results.load(record.attemptId)
        val outcomes = manifest.getJSONArray("outcomes")
        check(outcomes.length() == 3)
        check(outcomes.getJSONObject(0).getString("status") == "analyzed") { outcomes.toString() }
        check(outcomes.getJSONObject(1).getString("diagnostic") == "input_missing")
        check(outcomes.getJSONObject(2).getString("diagnostic") == "unsupported_codec")
        val descriptor = ResultDescriptor.from(manifest.getJSONArray("payloads").getJSONObject(0))
        val bytes = jobs.results.validate(record.attemptId, descriptor).readBytes()
        val doc = ForensicDocument.read(bytes)
        check(doc.status == "available" && doc.measurementStatus == "analyzed")
        val product = doc.value!!
        check(product["metadata"] is Map<*, *> && product["field_aliases"] is Map<*, *>)
        val measurement = product["measurement_report"] as Map<*, *>
        check((measurement["coverage"] as Map<*, *>)["decoded_pcm_sha256"] != null)
        // Synthetic version/abstention/failure fixtures test presentation, never DSP or scoring.
        val fixture = JSONObject(String(bytes, Charsets.UTF_8))
        fixture.put("unknown_future_integer", BigInteger("18446744073709551615")).put("unknown_null", JSONObject.NULL)
        fixture.getJSONObject("reference_assessment").put("status", "unavailable").put("scores", JSONObject.NULL).put("display_summary", "Generated abstention")
        fixture.getJSONObject("measurement_report").put("status", "failed")
        val generated = File(cacheDir, "a06-exact.json").apply { writeText(fixture.toString()) }
        val exactBytes = generated.readBytes()
        val attempt = UUID.randomUUID().toString()
        generatedAttempt = attempt
        jobs.results.reserve(attempt, 4L * 1024 * 1024)
        val exact = jobs.results.import(attempt, generated, "product", "audio-forensic-product-v1")
        val future = jobs.results.import(attempt, generated, "product", "future")
        val diagnostic = File(cacheDir, "a06-artifact.json").apply { writeText("{\"artifact\":{\"status\":\"failed\",\"reason\":\"resource_limit\",\"payload\":null}}") }
        val artifact = jobs.results.import(attempt, diagnostic, "alfred-result", "1")
        jobs.results.commit(JobRecord(UUID.randomUUID().toString(), attempt, "forensics", listOf(good.id), "{}", System.currentTimeMillis(), "completed"), listOf(exact, future, artifact), JSONArray())
        check(jobs.results.validate(attempt, exact).readBytes().contentEquals(exactBytes))
        val saved = ForensicDocument.read(jobs.results.validate(attempt, exact).readBytes())
        val savedValue = saved.value!!
        check(savedValue["unknown_future_integer"] == BigInteger("18446744073709551615"))
        check(savedValue.containsKey("unknown_null") && savedValue["unknown_null"] == null)
        check(saved.assessmentStatus == "unavailable" && saved.measurementStatus == "failed" && saved.summary == "Generated abstention")
        check(jobs.results.dispatch(future, setOf("product" to "audio-forensic-product-v1")) == "unsupported_version")
        val futureSchema = JSONObject(fixture.toString()).put("product_schema_version", "future")
        check(ForensicDocument.read(futureSchema.toString().toByteArray()).status == "unsupported_version")
        val futureMethod = JSONObject(fixture.toString()).apply { getJSONObject("reference_assessment").put("method_id", "future") }
        check(ForensicDocument.read(futureMethod.toString().toByteArray()).summary.startsWith("unsupported_version"))
        val shared = jobs.results.share(this, attempt, exact)
        contentResolver.openInputStream(shared)!!.use { check(it.readBytes().contentEquals(exactBytes)) }
        jobs.results.export(this, attempt, exact, shared)
        contentResolver.openInputStream(shared)!!.use { check(it.readBytes().contentEquals(exactBytes)) }
        check(jobs.results.load(attempt).getJSONArray("payloads").length() == 3)
        check(File(filesDir, "input-snapshots").listFiles().orEmpty().isEmpty())
        check(File(filesDir, "input-probes").listFiles().orEmpty().isEmpty())
        return 18
    }
}
