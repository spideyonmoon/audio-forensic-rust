package dev.alfred.compare

import android.app.ActivityManager
import android.content.Context
import dev.alfred.shared.*
import org.json.JSONArray
import org.json.JSONObject
import java.io.File

/** One negotiated live scope; the existing Rust comparator owns every ranking rule. */
object CompareWork {
    data class SavedProduct(val attempt: String, val descriptor: ResultDescriptor) {
        val id get() = descriptor.path.substringBefore('.')
    }

    fun savedProducts(store: ResultStore): List<SavedProduct> = store.entries().reversed().flatMap { entry ->
        val payloads = entry.getJSONArray("payloads")
        (0 until payloads.length()).map { ResultDescriptor.from(payloads.getJSONObject(it)) }
            .filter { it.kind == "product" }.map { SavedProduct(entry.getString("attempt_id"), it) }
    }

    /** Admission pins prevent eviction while queued. Copy original bytes so the
     * resulting comparison remains self-contained after its source is deleted. */
    fun submitSaved(app: Context, products: List<SavedProduct>, sameTrack: Boolean) = run {
        require(sameTrack && products.size in 2..32 && products.map { it.id }.distinct().size == products.size)
        if (products.any { it.descriptor.bytes !in 0..ResultStore.DOCUMENT_LIMIT } || products.sumOf { it.descriptor.bytes } > 128L * 1024 * 1024) throw InputFailure("resource_limit")
        val frozen = products.toList()
        val jobs = SharedJobs.get(app)
        val options = JSONObject().put("same_track_asserted", true).put("input_mode", "saved")
            .put("scope", "original saved coverage; Rust checks compatibility")
            .put("tracks", JSONArray(frozen.map { JSONObject().put("attempt_id", it.attempt).put("payload", it.descriptor.json()) }))
        jobs.submit("compare", frozen.map { it.id }, options,
            minOf(512L * 1024 * 1024, frozen.sumOf { it.descriptor.bytes } + ResultStore.DOCUMENT_LIMIT + 1024 * 1024),
            FeatureWork { context ->
                context.phase("running")
                val copied = frozen.map { input ->
                    context.check()
                    val source = context.results.validate(input.attempt, input.descriptor)
                    context.persist(source, input.descriptor.kind, input.descriptor.version)
                }
                compare(app, context, NativeClient(), copied, java.util.UUID.randomUUID().toString(), 1,
                    JSONObject().put("kind", "full"))
                context.phase("finalizing")
            }, retainGrants = {
                val pins = mutableListOf<AutoCloseable>()
                try {
                    if (frozen.any { it.descriptor.version != "audio-forensic-product-v1" }) throw InputFailure("unsupported_version")
                    if (frozen.any { it.descriptor.bytes < 0 } || frozen.sumOf { it.descriptor.bytes } > 128L * 1024 * 1024) throw InputFailure("resource_limit")
                    frozen.map { it.attempt }.distinct().forEach { pins.add(jobs.results.pin(it)) }
                    frozen.forEach { jobs.results.validate(it.attempt, it.descriptor) }
                    AutoCloseable { pins.forEach { it.close() } }
                } catch (error: Throwable) { pins.forEach { it.close() }; throw error }
            })
    }

    private fun compare(app: Context, context: JobContext, client: NativeClient, products: List<ResultDescriptor>,
                        selection: String, generation: Long, scope: JSONObject) {
        client.use {
            OwnedDirectory.create(File(app.filesDir, "input-probes"), context.record.attemptId).use { output ->
                val memory = ActivityManager.MemoryInfo()
                (app.getSystemService(Context.ACTIVITY_SERVICE) as ActivityManager).getMemoryInfo(memory)
                val saved = JSONArray(products.map { descriptor ->
                    JSONObject().put("root", context.results.validate(context.record.attemptId, descriptor).parentFile!!.absolutePath)
                        .put("payload", JSONObject().put("path", descriptor.path).put("kind", descriptor.kind).put("version", descriptor.version).put("bytes", descriptor.bytes).put("sha256", descriptor.sha256))
                })
                val request = JSONObject().put("version", 1).put("kind", "feature").put("feature", "compare")
                    .put("job_id", context.record.jobId).put("attempt_id", context.record.attemptId)
                    .put("selection_id", selection).put("generation", generation)
                    .put("item_ids", JSONArray(context.record.items)).put("leases", JSONArray()).put("saved", saved)
                    .put("same_track_asserted", true).put("scope", scope).put("deadline_ms", 1200000)
                    .put("output_dir", output.file.absolutePath).put("available_memory_bytes", memory.availMem)
                    .put("low_memory", memory.lowMemory).put("reserved_output_bytes", ResultStore.DOCUMENT_LIMIT)
                val result = context.runNative(request, null, output, client, savedPins = listOf(context.results.pin(context.record.attemptId)))
                context.adoptNative(result, output)
            }
        }
    }
    fun prefixBudget(inputs: FeatureInputs): Long? = inputs.selection.items.map { item ->
        val probe = inputs.probes[item.id] ?: return null
        val rate = probe.rate?.takeIf { it > 0 } ?: return null
        minOf(180L, 8_640_000L / rate, probe.frames?.divide(java.math.BigInteger.valueOf(rate))?.min(java.math.BigInteger.valueOf(180))?.toLong() ?: 180L)
    }.minOrNull()

    fun submit(app: Context, inputs: FeatureInputs, scope: JSONObject, sameTrack: Boolean) = run {
        require(sameTrack && inputs.selection.items.size in 2..32)
        require(inputs.selection.items.all { inputs.probes[it.id]?.status == "available" })
        if (scope.getString("kind") == "prefix") {
            val seconds = scope.getLong("seconds")
            require(seconds > 0 && seconds <= (prefixBudget(inputs) ?: 0) && scope.getDouble("seconds") == seconds.toDouble())
        } else require(scope.getString("kind") == "full" && inputs.selection.items.all { (inputs.probes[it.id]?.rate ?: Long.MAX_VALUE) <= 48000 })
        val frozenScope = scope.toString()
        val options = JSONObject().put("scope", scope).put("same_track_asserted", true)
            .put("tracks", JSONArray(inputs.selection.items.map { JSONObject().put("item_id", it.id).put("name", it.name).put("encoded_sha256", inputs.probes[it.id]?.hash) }))
        SharedJobs.get(app).submit("compare", inputs.selection.items.map { it.id }, options,
            minOf(512L * 1024 * 1024, (inputs.selection.items.size + 1) * ResultStore.DOCUMENT_LIMIT + 1024 * 1024),
            FeatureWork { context ->
                NativeClient().use { client ->
                    context.phase("running")
                    val products = mutableListOf<ResultDescriptor>()
                    var bytes = 0L
                    for (item in inputs.selection.items) {
                        context.check(); context.progress(item.id, "acquiring")
                        try {
                            inputs.acquire(item, context.record.attemptId, context.cancellation).use { snapshot ->
                                OwnedDirectory.create(File(app.filesDir, "input-probes"), context.record.attemptId).use { output ->
                                    val request = nativeFeatureRequest(app, context, inputs, item, snapshot, output, "forensics", JSONObject(frozenScope))
                                    val result = context.runNative(request, snapshot, output, client)
                                    val product = context.adoptNative(result, output, keepManifest = false).payload
                                    products.add(product); bytes += product.bytes
                                    context.outcome(item.id, (result["summary"] as? Map<*, *>)?.get("status")?.toString() ?: "unknown", "payload:${product.path}")
                                    if (bytes > 128L * 1024 * 1024) throw InputFailure("resource_limit")
                                }
                            }
                        } catch (error: InputFailure) {
                            if (error.code in setOf("cancelled", "interrupted", "native_release_unconfirmed", "storage_full", "resource_limit")) throw error
                            context.outcome(item.id, "failed", error.code)
                        }
                    }
                    // Missing host inputs cannot be turned into invented Rust product reports.
                    if (products.size != inputs.selection.items.size) throw InputFailure("input_unavailable")
                    OwnedDirectory.create(File(app.filesDir, "input-probes"), context.record.attemptId).use { output ->
                        val memory = ActivityManager.MemoryInfo()
                        (app.getSystemService(Context.ACTIVITY_SERVICE) as ActivityManager).getMemoryInfo(memory)
                        val saved = JSONArray(products.map { descriptor ->
                            JSONObject().put("root", context.results.validate(context.record.attemptId, descriptor).parentFile!!.absolutePath)
                                .put("payload", JSONObject().put("path", descriptor.path).put("kind", descriptor.kind).put("version", descriptor.version).put("bytes", descriptor.bytes).put("sha256", descriptor.sha256))
                        })
                        val request = JSONObject().put("version", 1).put("kind", "feature").put("feature", "compare")
                            .put("job_id", context.record.jobId).put("attempt_id", context.record.attemptId)
                            .put("selection_id", inputs.selection.id).put("generation", inputs.selection.generation)
                            .put("item_ids", JSONArray(context.record.items)).put("leases", JSONArray()).put("saved", saved)
                            .put("same_track_asserted", true).put("scope", JSONObject(frozenScope)).put("deadline_ms", 1200000)
                            .put("output_dir", output.file.absolutePath).put("available_memory_bytes", memory.availMem)
                            .put("low_memory", memory.lowMemory).put("reserved_output_bytes", ResultStore.DOCUMENT_LIMIT)
                        val inputPin = context.results.pin(context.record.attemptId)
                        val result = context.runNative(request, null, output, client, savedPins = listOf(inputPin))
                        context.adoptNative(result, output)
                    }
                    context.phase("finalizing")
                }
            }, retainGrants = inputs.retainGrants)
    }
}
