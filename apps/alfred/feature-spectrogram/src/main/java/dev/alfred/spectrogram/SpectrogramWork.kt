package dev.alfred.spectrogram

import android.content.Context
import dev.alfred.shared.*
import org.json.JSONObject
import java.io.File

object SpectrogramWork {
    fun submit(app: Context, inputs: FeatureInputs, scope: JSONObject, preset: String) = run {
        require(inputs.selection.items.size == 1 && preset in setOf("standard", "publication", "large"))
        val item = inputs.selection.items.single()
        require(inputs.probes[item.id]?.status == "available")
        val options = JSONObject().put("scope", scope).put("preset", preset).put("title", item.name)
        val frozenScope = scope.toString()
        SharedJobs.get(app).submit("spectrogram", listOf(item.id), options, 128L * 1024 * 1024,
            FeatureWork { context ->
                NativeClient().use { client ->
                    context.phase("running")
                    context.progress(item.id, "acquiring")
                    inputs.acquire(item, context.record.attemptId, context.cancellation).use { snapshot ->
                        OwnedDirectory.create(File(app.filesDir, "input-probes"), context.record.attemptId).use { output ->
                            val request = nativeFeatureRequest(app, context, inputs, item, snapshot, output, "spectrogram", JSONObject(frozenScope), true, preset)
                            val result = context.runNative(request, snapshot, output, client)
                            val saved = context.adoptNative(result, output)
                            context.outcome(item.id, (result["summary"] as? Map<*, *>)?.get("status")?.toString() ?: "unknown", "PNG:${saved.artifactStatus}; payload:${saved.payload.path}")
                        }
                    }
                    context.phase("finalizing")
                }
            }, retainGrants = inputs.retainGrants)
    }
}
