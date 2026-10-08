package dev.alfred.shared

import android.app.Activity
import android.content.ClipData
import android.content.Context
import android.content.Intent
import android.net.Uri
import androidx.activity.result.contract.ActivityResultContract
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.util.concurrent.CompletableFuture
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean

class ResultDestination : ActivityResultContract<String, Uri?>() {
    override fun createIntent(context: Context, input: String) = Intent(Intent.ACTION_CREATE_DOCUMENT)
        .addCategory(Intent.CATEGORY_OPENABLE).setType(if (input.endsWith(".png", true)) "image/png" else if (input.endsWith(".json", true)) "application/json" else "application/octet-stream")
        .putExtra(Intent.EXTRA_TITLE, boundedName(input))
    override fun parseResult(resultCode: Int, intent: Intent?) = if (resultCode == Activity.RESULT_OK) intent?.data else null
}

/** One transfer and the last 32 attempt receipts. A failed export leaves history intact. */
class ResultTransfer(context: Context, private val store: ResultStore) {
    private val app = context.applicationContext
    private val executor = Executors.newSingleThreadExecutor { Thread(it, "alfred-export") }
    private val occupied = AtomicBoolean(false)
    private val receipt = File(app.filesDir, "export-attempts.json")
    private fun <T> transfer(attempt: String, action: () -> T): CompletableFuture<T> {
        val future = CompletableFuture<T>()
        if (!occupied.compareAndSet(false, true)) { future.completeExceptionally(InputFailure("busy")); return future }
        executor.execute {
            var status = "completed"
            try { future.complete(action()) }
            catch (error: Throwable) { status = if (error is InputFailure) error.code else "io_error"; future.completeExceptionally(InputFailure(status)) }
            finally {
                try {
                    val old = if (receipt.exists()) readJson(receipt).getJSONArray("attempts") else JSONArray()
                    val kept = JSONArray()
                    for (i in maxOf(0, old.length() - 31) until old.length()) kept.put(old.getJSONObject(i))
                    kept.put(JSONObject().put("attempt_id", attempt).put("status", status).put("time_ms", System.currentTimeMillis()))
                    atomicJson(receipt, JSONObject().put("version", 1).put("attempts", kept))
                } finally { occupied.set(false) }
            }
        }
        return future
    }
    fun export(attempt: String, descriptor: ResultDescriptor, destination: Uri) =
        transfer(attempt) { store.export(app, attempt, descriptor, destination) }
    fun share(attempt: String, descriptor: ResultDescriptor) = transfer(attempt) {
        val uri = store.share(app, attempt, descriptor)
        Intent(Intent.ACTION_SEND).setType(if (descriptor.kind == "png") "image/png" else if (descriptor.kind in setOf("product", "spectrogram", "comparison", "alfred-result")) "application/json" else "application/octet-stream").putExtra(Intent.EXTRA_STREAM, uri)
            .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION).apply { clipData = ClipData.newRawUri("Alfred result", uri) }
    }
}
