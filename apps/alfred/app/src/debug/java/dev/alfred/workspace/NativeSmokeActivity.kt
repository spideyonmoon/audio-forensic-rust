package dev.alfred.workspace

import android.app.Activity
import android.os.Bundle
import dev.alfred.shared.ExactJson
import dev.alfred.shared.NativeBootstrap
import dev.alfred.shared.NativeTransport
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.math.BigInteger
import java.security.MessageDigest
import java.util.UUID

/** Explicit adb-only generated-input harness; never shipped in release builds. */
class NativeSmokeActivity : Activity() {
    override fun onCreate(state: Bundle?) {
        super.onCreate(state)
        Thread({
            val receipt = File(filesDir, "native-smoke.json")
            try {
                val checks = smoke()
                receipt.writeText(JSONObject().put("passed", true).put("checks", checks)
                    .put("api", android.os.Build.VERSION.SDK_INT).put("abi", android.os.Build.SUPPORTED_ABIS[0]).toString())
            } catch (error: Throwable) {
                receipt.writeText(JSONObject().put("passed", false).put("error", error.toString()).toString())
            }
            runOnUiThread { finish() }
        }, "alfred-jni-smoke").start()
    }
    @Suppress("UNCHECKED_CAST")
    private fun obj(value: Any?) = value as Map<String, Any?>
    private fun decode(bytes: ByteArray) = obj(ExactJson.parse(bytes))
    private fun ok(bytes: ByteArray): Map<String, Any?> {
        val response = decode(bytes)
        check(response["ok"] == true) { response.toString() }
        return obj(response["value"])
    }
    private fun BigInteger.asHandle(): Long {
        check(signum() > 0 && bitLength() <= 63) { "invalid native handle" }
        return toLong()
    }
    private fun hash(bytes: ByteArray) = MessageDigest.getInstance("SHA-256").digest(bytes).joinToString("") { "%02x".format(it) }
    private fun request(input: File, kind: String = "feature", feature: String? = "forensics", png: Boolean = false): ByteArray {
        val attempt = UUID.randomUUID().toString()
        return JSONObject().put("version", 1).put("kind", kind).put("job_id", UUID.randomUUID())
            .put("attempt_id", attempt).put("selection_id", UUID.randomUUID()).put("generation", BigInteger("18446744073709551615"))
            .put("item_ids", JSONArray().put(UUID.randomUUID())).put("feature", feature ?: JSONObject.NULL)
            .put("leases", JSONArray().put(JSONObject().put("id", UUID.randomUUID()).put("attempt_id", attempt)
                .put("path", input.absolutePath).put("bytes", input.length()).put("sha256", hash(input.readBytes())).put("name", input.name)))
            .put("scope", JSONObject().put("kind", "full")).put("deadline_ms", 120000)
            .put("output_dir", File(filesDir, "native-results").apply { mkdirs() }.absolutePath)
            .put("available_memory_bytes", 2147483648L).put("low_memory", false)
            .put("reserved_output_bytes", 134217728).put("png", png).toString().toByteArray()
    }
    private fun finish(handle: Long): Map<String, Any?> {
        val deadline = System.nanoTime() + 120_000_000_000L
        while (System.nanoTime() < deadline) {
            val p = ok(NativeTransport.poll(handle))
            check(p["generation"] == BigInteger("18446744073709551615"))
            if (p["terminal"] != null) {
                check(p["leases_released"] == true)
                check(ok(NativeTransport.poll(handle)) == p) // Repeatable payload descriptor.
                ok(NativeTransport.cancel(handle)); ok(NativeTransport.cancel(handle))
                ok(NativeTransport.close(handle)); ok(NativeTransport.close(handle))
                check(obj(decode(NativeTransport.poll(handle))["error"])["code"] == "stale_handle")
                val terminal = obj(p["terminal"])
                check(terminal["status"] == "completed") { terminal.toString() }
                return obj(terminal["result"])
            }
            Thread.sleep(250)
        }
        error("native worker timeout")
    }
    private fun payload(result: Map<String, Any?>): Map<String, Any?> {
        val descriptor = obj(result["payload"])
        val path = File(File(filesDir, "native-results/${result["directory"]}"), descriptor["path"] as String)
        val bytes = path.readBytes()
        check(hash(bytes) == descriptor["sha256"])
        check(BigInteger.valueOf(bytes.size.toLong()) == descriptor["bytes"])
        return obj(ExactJson.parse(bytes, 64*1024*1024))
    }
    private fun smoke(): Int {
        check(NativeBootstrap.load() == "Native host v1 loaded")
        check(ok(NativeTransport.describe())["version"] == BigInteger.ONE)
        check(obj(decode(NativeTransport.start(ByteArray(65537)))["error"])["code"] == "size_limit")
        check(obj(decode(NativeTransport.start("{".toByteArray()))["error"])["code"] == "invalid_request")
        val roundTrip = obj(ExactJson.parse("{\"n\":18446744073709551615,\"x\":null,\"future\":{\"v\":1.25}}".toByteArray()))
        check(roundTrip["n"] == BigInteger("18446744073709551615") && roundTrip.containsKey("x") && roundTrip["x"] == null)
        val controls = listOf("noise16.wav" to "1981c82a788cf0be394f6c6326e6e7576a788076b51e7f7834286cada2b0f0e4",
            "noise16.flac" to "1981c82a788cf0be394f6c6326e6e7576a788076b51e7f7834286cada2b0f0e4",
            "alac/8000-16-1-tail.m4a" to "8211292db8b4da19fb6b60bc1b1896c20a8108dbd3eb59f5ee9b7ee9c2790f89")
        for ((name, expected) in controls) {
            val input = File(filesDir, name.substringAfterLast('/'))
            assets.open(name).use { source -> input.outputStream().use { source.copyTo(it) } }
            val req = request(input, png = true)
            val h = (ok(NativeTransport.start(req))["handle"] as BigInteger).asHandle()
            check(obj(decode(NativeTransport.start(req))["error"])["code"] == "busy")
            val result = finish(h)
            val product = payload(result)
            val measurement = obj(product["measurement_report"])
            check(measurement["status"] == "analyzed")
            check(obj(measurement["coverage"])["decoded_pcm_sha256"] == expected)
            val artifact = obj(result["artifact"])
            check(artifact["status"] == "available") { artifact.toString() }
            val png = File(File(filesDir, "native-results/${result["directory"]}"), obj(artifact["payload"])["path"] as String).readBytes()
            check(png.take(8) == listOf<Byte>(-119,80,78,71,13,10,26,10))
            val dimensions = java.nio.ByteBuffer.wrap(png, 16, 8)
            check(dimensions.int == 2560 && dimensions.int == 1440)
        }
        val input = File(filesDir,"noise16.wav")
        val probe = finish((ok(NativeTransport.start(request(input,"probe",null)))["handle"] as BigInteger).asHandle())
        check(payload(probe)["metadata_version"] == BigInteger.ONE)
        val spec = finish((ok(NativeTransport.start(request(input,feature="spectrogram",png=true)))["handle"] as BigInteger).asHandle())
        check(obj(payload(spec)["measurement"])["status"] == "analyzed")
        // AAC-in-M4A codec marker mutation follows the existing core rejection control.
        val aac = File(filesDir, "unsupported.m4a")
        val encoded = File(filesDir, "8000-16-1-tail.m4a").readBytes()
        val marker = "alac".toByteArray()
        val offset = (0..encoded.size - 4).first { i -> encoded.copyOfRange(i, i + 4).contentEquals(marker) }
        "mp4a".toByteArray().copyInto(encoded, offset)
        aac.writeBytes(encoded)
        val dsd = File(filesDir, "unsupported.dsf").apply { writeBytes("DSD     ".toByteArray()) }
        for (unsupported in listOf(aac, dsd)) {
            val p = payload(finish((ok(NativeTransport.start(request(unsupported)))["handle"] as BigInteger).asHandle()))
            check(obj(p["measurement_report"])["status"] == "unsupported")
        }
        val wave = input.readBytes()
        val info = java.io.ByteArrayOutputStream().apply {
            write("INFO".toByteArray())
            repeat(10) {
                write("ICMT".toByteArray())
                write(java.nio.ByteBuffer.allocate(4).order(java.nio.ByteOrder.LITTLE_ENDIAN).putInt(12000).array())
                write(ByteArray(12000) { 120 })
            }
        }.toByteArray()
        val tagged = java.io.ByteArrayOutputStream().apply {
            write(wave, 0, 12); write("LIST".toByteArray())
            write(java.nio.ByteBuffer.allocate(4).order(java.nio.ByteOrder.LITTLE_ENDIAN).putInt(info.size).array())
            write(info); write(wave, 12, wave.size - 12)
        }.toByteArray()
        java.nio.ByteBuffer.wrap(tagged, 4, 4).order(java.nio.ByteOrder.LITTLE_ENDIAN).putInt(tagged.size - 8)
        val taggedFile = File(filesDir, "large-metadata.wav").apply { writeBytes(tagged) }
        val metadata = finish((ok(NativeTransport.start(request(taggedFile,"probe",null)))["handle"] as BigInteger).asHandle())
        check((obj(metadata["payload"])["bytes"] as BigInteger) > BigInteger.valueOf(65536))
        check(obj(payload(metadata)["text_limits"])["retained_text_utf8_bytes"] == BigInteger.valueOf(120040))
        val cancelHandle = (ok(NativeTransport.start(request(input)))["handle"] as BigInteger).asHandle()
        ok(NativeTransport.cancel(cancelHandle)); ok(NativeTransport.close(cancelHandle))
        val releaseDeadline = System.nanoTime() + 10_000_000_000L
        while (ok(NativeTransport.describe())["active_handle"] != null) {
            check(System.nanoTime() < releaseDeadline)
            Thread.sleep(250)
        }
        return 16
    }
}
