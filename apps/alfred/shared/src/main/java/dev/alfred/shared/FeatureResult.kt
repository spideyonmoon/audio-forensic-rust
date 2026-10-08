package dev.alfred.shared

import android.app.ActivityManager
import android.content.Context
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.math.BigInteger
import java.security.MessageDigest

/** Platform request construction and byte adoption only; feature policy stays in Rust. */
fun nativeFeatureRequest(app: Context, context: JobContext, inputs: FeatureInputs, item: InputItem,
                         snapshot: OwnedInput, output: OwnedDirectory, feature: String,
                         scope: JSONObject, png: Boolean = false, preset: String = "publication"): JSONObject {
    val memory = ActivityManager.MemoryInfo()
    (app.getSystemService(Context.ACTIVITY_SERVICE) as ActivityManager).getMemoryInfo(memory)
    return JSONObject().put("version", 1).put("kind", "feature").put("feature", feature)
        .put("job_id", context.record.jobId).put("attempt_id", context.record.attemptId)
        .put("selection_id", inputs.selection.id).put("generation", inputs.selection.generation)
        .put("item_ids", JSONArray().put(item.id))
        .put("leases", JSONArray().put(JSONObject().put("id", snapshot.id).put("attempt_id", snapshot.attemptId)
            .put("path", snapshot.file.absolutePath).put("bytes", snapshot.bytes).put("sha256", snapshot.sha256).put("name", item.name)))
        .put("scope", scope).put("deadline_ms", 1200000).put("output_dir", output.file.absolutePath)
        .put("available_memory_bytes", memory.availMem).put("low_memory", memory.lowMemory)
        .put("reserved_output_bytes", ResultStore.DOCUMENT_LIMIT).put("png", png).put("preset", preset)
}

data class AdoptedResult(val payload: ResultDescriptor, val artifact: ResultDescriptor?, val artifactStatus: String)

/** Called after runNative confirms release. Validate every native path/hash before adoption. */
fun JobContext.adoptNative(result: Map<String, Any?>, output: OwnedDirectory, keepManifest: Boolean = true): AdoptedResult {
    if (result["directory"] != record.attemptId) throw InputFailure("invalid_payload")
    val directory = File(output.file, record.attemptId)
    if (directory.canonicalFile.parentFile != output.file.canonicalFile) throw InputFailure("invalid_payload")
    fun adopt(value: Any?, artifact: Boolean = false): ResultDescriptor {
        val descriptor = value as? Map<*, *> ?: throw InputFailure("invalid_payload")
        val path = descriptor["path"] as? String ?: throw InputFailure("invalid_payload")
        if (path.contains('/') || path.contains('\\') || path in setOf(".", "..") || File(path).isAbsolute) throw InputFailure("invalid_payload")
        val file = File(directory, path)
        val bytes = descriptor["bytes"] as? BigInteger ?: throw InputFailure("invalid_payload")
        if (file.canonicalFile.parentFile != directory.canonicalFile || !file.isFile || bytes != BigInteger.valueOf(file.length()) || file.length() > ResultStore.DOCUMENT_LIMIT) throw InputFailure("invalid_payload")
        val digest = MessageDigest.getInstance("SHA-256")
        file.inputStream().use { source ->
            val buffer = ByteArray(InputStore.BUFFER_BYTES)
            while (true) { check(); val count = source.read(buffer); if (count < 0) break; digest.update(buffer, 0, count) }
        }
        if (digest.digest().joinToString("") { "%02x".format(it) } != descriptor["sha256"]) throw InputFailure("invalid_payload")
        return persist(file, descriptor["kind"] as String, descriptor["version"] as String, artifact)
    }
    val payload = adopt(result["payload"])
    // Keep the native manifest unchanged: source/item binding and independent PNG diagnostics.
    if (keepManifest) adopt(result["manifest"])
    val image = result["artifact"] as? Map<*, *>
    val status = image?.get("status") as? String ?: "not_requested"
    val artifact = if (status == "available") adopt(image?.get("payload"), true) else null
    return AdoptedResult(payload, artifact, status)
}

fun resultFailure(error: Throwable): String {
    val cause = if (error is java.util.concurrent.CompletionException || error is java.util.concurrent.ExecutionException) error.cause ?: error else error
    return if (cause is InputFailure) cause.code else "io_error"
}
