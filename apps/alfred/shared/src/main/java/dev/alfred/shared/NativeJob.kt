package dev.alfred.shared

import org.json.JSONObject
import java.math.BigInteger

/** Blocking adapter helper for the shared worker only. Return requires actual
 * native release, including cancellation/close. Tokens never enter the journal. */
fun JobContext.runNative(request: JSONObject, snapshot: OwnedInput?, output: OwnedDirectory,
                         client: NativeClient, savedPins: List<AutoCloseable> = emptyList(),
                         onStarted: (Long) -> Unit = {}): Map<String, Any?> {
    @Suppress("UNCHECKED_CAST")
    fun objectValue(value: Any?) = value as Map<String, Any?>
    fun response(bytes: ByteArray): Map<String, Any?> {
        val envelope = objectValue(ExactJson.parse(bytes))
        if (envelope["ok"] != true) throw InputFailure(objectValue(envelope["error"])["code"] as String)
        return objectValue(envelope["value"])
    }
    require(request.getString("attempt_id") == record.attemptId && (snapshot == null || snapshot.attemptId == record.attemptId))
    val sourcePin = snapshot?.retain()
    val outputPin = output.retain()
    var handle: Long? = null
    var released = true
    try {
        check()
        handle = (response(client.start(request.toString().toByteArray(Charsets.UTF_8)).get())["handle"] as BigInteger).checkedPositiveLong()
        released = false
        onStarted(handle)
        while (true) {
            check()
            val polled = response(client.poll(handle).get())
            if (polled["attempt_id"] != record.attemptId) throw InputFailure("stale_handle")
            val event = polled["progress"] as? Map<*, *>
            if (event != null) progress(request.getJSONArray("item_ids").optString(0), event["phase"]?.toString() ?: "running",
                (event["pass"] as? BigInteger)?.toInt(), event["frames"]?.toString(), event["expected_frames"]?.toString())
            val terminal = polled["terminal"] as? Map<*, *>
            if (terminal != null) {
                released = polled["leases_released"] == true
                if (!released) throw InputFailure("io_error")
                if (terminal["status"] != "completed") throw InputFailure(
                    (terminal["error"] as? Map<*, *>)?.get("code") as? String
                        ?: terminal["status"] as? String ?: "host_error")
                return objectValue(terminal["result"])
            }
            Thread.sleep(250)
        }
    } finally {
        try {
            handle?.let {
                response(client.close(it).get()) // Native close requests cooperative cancellation.
                while (!released) {
                    val active = response(client.describe().get())["active_handle"] as? BigInteger
                    if (active?.toLong() != it) released = true else Thread.sleep(250)
                }
            }
        } catch (error: Throwable) {
            if (!released) throw InputFailure("native_release_unconfirmed")
            throw error
        } finally {
            // Preserve pins on transport failure rather than remove a live source.
            if (released) { sourcePin?.close(); outputPin.close(); savedPins.forEach { it.close() } }
        }
    }
}
