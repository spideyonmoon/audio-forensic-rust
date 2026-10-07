package dev.alfred.shared

import android.os.Looper
import android.util.JsonReader
import android.util.JsonToken
import java.io.StringReader
import java.math.BigDecimal
import java.math.BigInteger
import java.nio.ByteBuffer
import java.nio.charset.CodingErrorAction
import java.util.concurrent.ArrayBlockingQueue
import java.util.concurrent.CompletableFuture
import java.util.concurrent.ThreadPoolExecutor
import java.util.concurrent.TimeUnit
import org.json.JSONObject

/** Byte transport only. All consumers, including describe/load, run off main. */
object NativeTransport {
    external fun describe(): ByteArray
    external fun start(request: ByteArray): ByteArray
    external fun poll(handle: Long): ByteArray
    external fun cancel(handle: Long): ByteArray
    external fun close(handle: Long): ByteArray
}

/** Exact numeric tokens; original bytes remain authoritative for persistence/export.
 * Android JSONObject's number coercions are deliberately not used for reading.
 */
object ExactJson {
    fun parse(bytes: ByteArray, limit: Int = 64 * 1024): Any? {
        require(bytes.size <= limit) { "size_limit" }
        val decoder = Charsets.UTF_8.newDecoder()
            .onMalformedInput(CodingErrorAction.REPORT).onUnmappableCharacter(CodingErrorAction.REPORT)
        val text = decoder.decode(ByteBuffer.wrap(bytes)).toString()
        return JsonReader(StringReader(text)).use { reader ->
            reader.isLenient = false
            val value = read(reader, 0)
            require(reader.peek() == JsonToken.END_DOCUMENT) { "invalid_request" }
            value
        }
    }
    private fun read(reader: JsonReader, depth: Int): Any? {
        require(depth <= 128) { "invalid_request" }
        return when (reader.peek()) {
            JsonToken.BEGIN_OBJECT -> {
                val result = linkedMapOf<String, Any?>()
                reader.beginObject()
                while (reader.hasNext()) {
                    val key = reader.nextName()
                    require(!result.containsKey(key)) { "invalid_request" }
                    result[key] = read(reader, depth + 1)
                }
                reader.endObject(); result
            }
            JsonToken.BEGIN_ARRAY -> {
                val result = mutableListOf<Any?>()
                reader.beginArray()
                while (reader.hasNext()) result.add(read(reader, depth + 1))
                reader.endArray(); result
            }
            JsonToken.NUMBER -> reader.nextString().let {
                if (it.any { c -> c == '.' || c == 'e' || c == 'E' }) BigDecimal(it) else BigInteger(it)
            }
            JsonToken.STRING -> reader.nextString()
            JsonToken.BOOLEAN -> reader.nextBoolean()
            JsonToken.NULL -> { reader.nextNull(); null }
            else -> error("invalid_request")
        }
    }
}

/** Bounded control executor is independent of the native DSP worker. A05 owns
 * observation (at most every 250 ms), attempts, snapshot leases and history.
 * After close, wait until describe.active_handle differs before deleting input.
 * Never await a future on main. No token is persisted or reused after restart.
 */
class NativeClient : AutoCloseable {
    private val control = ThreadPoolExecutor(1, 1, 0, TimeUnit.MILLISECONDS,
        ArrayBlockingQueue<Runnable>(8), { r -> Thread(r, "alfred-control") },
        ThreadPoolExecutor.AbortPolicy())
    private fun failure(code: String, attempt: String?): ByteArray = JSONObject()
        .put("ok", false).put("error", JSONObject().put("code", code)
            .put("message", code).put("attempt_id", attempt ?: JSONObject.NULL)
            .put("phase", "transport")).toString().toByteArray(Charsets.UTF_8)
    private fun call(attempt: String? = null, block: () -> ByteArray): CompletableFuture<ByteArray> {
        val future = CompletableFuture<ByteArray>()
        try {
            control.execute {
                try {
                    check(Looper.myLooper() != Looper.getMainLooper())
                    val loaded = NativeBootstrap.load()
                    if (loaded != "Native host v1 loaded") {
                        future.complete(failure(loaded, attempt))
                        return@execute
                    }
                    val bytes = block()
                    ExactJson.parse(bytes) // Validate without rewriting original bytes.
                    future.complete(bytes)
                } catch (_: LinkageError) { future.complete(failure("native_load_failed", attempt))
                } catch (_: Exception) { future.complete(failure("invalid_request", attempt)) }
            }
        } catch (error: java.util.concurrent.RejectedExecutionException) {
            future.complete(failure("busy", attempt))
        }
        return future
    }
    fun describe() = call { NativeTransport.describe() }
    fun start(request: ByteArray): CompletableFuture<ByteArray> {
        if (request.size > 64 * 1024) return CompletableFuture.completedFuture(failure("size_limit", null))
        val owned = request.copyOf()
        val attempt = try { (ExactJson.parse(owned) as? Map<*, *>)?.get("attempt_id") as? String } catch (_: Exception) { null }
        return call(attempt) { NativeTransport.start(owned) }
    }
    fun poll(handle: Long) = call { NativeTransport.poll(handle) }
    fun cancel(handle: Long) = call { NativeTransport.cancel(handle) }
    fun close(handle: Long) = call { NativeTransport.close(handle) }
    override fun close() { control.shutdown() }
}
