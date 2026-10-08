package dev.alfred.forensics

import android.content.Context
import dev.alfred.shared.*
import org.json.JSONObject
import org.json.JSONArray
import java.io.File
import java.util.concurrent.CompletableFuture

object ForensicsWork {
    fun submit(app: Context, inputs: FeatureInputs, scope: JSONObject, retryJob: String? = null): CompletableFuture<JobRecord> {
        val options = JSONObject().put("scope", scope).put("tracks", JSONArray(inputs.selection.items.map {
            JSONObject().put("item_id", it.id).put("name", it.name)
        }))
        val frozenScope = scope.toString()
        val work = FeatureWork { context ->
            NativeClient().use { client ->
                context.phase("running")
                for (item in inputs.selection.items) {
                    context.check()
                    val probe = inputs.probes[item.id]
                    if (probe?.status != "available") {
                        context.outcome(item.id, "unavailable", probe?.reason ?: "input_missing")
                        continue
                    }
                    try {
                        context.progress(item.id, "acquiring")
                        inputs.acquire(item, context.record.attemptId, context.cancellation).use { snapshot ->
                            OwnedDirectory.create(File(app.filesDir, "input-probes"), context.record.attemptId).use { output ->
                                val request = nativeFeatureRequest(app, context, inputs, item, snapshot, output, "forensics", JSONObject(frozenScope))
                                val result = context.runNative(request, snapshot, output, client)
                                val adopted = context.adoptNative(result, output)
                                val status = (result["summary"] as? Map<*, *>)?.get("status") as? String ?: "unknown"
                                context.outcome(item.id, status, "payload:${adopted.payload.path}; artifact:${adopted.artifactStatus}")
                            }
                        }
                    } catch (error: InputFailure) {
                        if (error.code in setOf("cancelled", "interrupted", "native_release_unconfirmed", "storage_full")) throw error
                        context.outcome(item.id, "failed", error.code)
                    }
                }
                context.phase("finalizing")
            }
        }
        val jobs = SharedJobs.get(app)
        val reservation = minOf(512L * 1024 * 1024, inputs.selection.items.size * ResultStore.DOCUMENT_LIMIT + 1024 * 1024)
        return if (retryJob == null) jobs.submit("forensics", inputs.selection.items.map { it.id }, options, reservation, work, retainGrants = inputs.retainGrants)
        else jobs.submit("forensics", inputs.selection.items.map { it.id }, options, reservation, work, retryJob, inputs.retainGrants)
    }
}
