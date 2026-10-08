package dev.alfred.shared

import android.content.Context
import android.net.Uri
import android.util.AtomicFile
import androidx.core.content.FileProvider
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.io.FileOutputStream
import java.nio.file.Files
import java.nio.file.StandardCopyOption
import java.security.MessageDigest
import java.util.UUID

internal fun atomicJson(file: File, value: JSONObject) {
    file.parentFile!!.mkdirs()
    val atomic = AtomicFile(file)
    val stream = atomic.startWrite()
    try { stream.write(value.toString().toByteArray(Charsets.UTF_8)); atomic.finishWrite(stream) }
    catch (error: Throwable) { atomic.failWrite(stream); throw error }
}

internal fun readJson(file: File): JSONObject = JSONObject(AtomicFile(file).openRead().use {
    val bytes = it.readNBytesCompat(1024 * 1024 + 1)
    if (bytes.size > 1024 * 1024) throw InputFailure("size_limit")
    String(bytes, Charsets.UTF_8)
})

private fun java.io.InputStream.readNBytesCompat(limit: Int): ByteArray {
    val output = java.io.ByteArrayOutputStream()
    val buffer = ByteArray(8192)
    while (output.size() < limit) { val count = read(buffer, 0, minOf(buffer.size, limit - output.size()))
        if (count < 0) break; if (count == 0) throw InputFailure("io_error"); output.write(buffer, 0, count) }
    return output.toByteArray()
}

data class ResultDescriptor(val path: String, val kind: String, val version: String,
                            val bytes: Long, val sha256: String, val artifact: Boolean = false) {
    fun json() = JSONObject().put("path", path).put("kind", kind).put("version", version)
        .put("bytes", bytes).put("sha256", sha256).put("artifact", artifact).put("status", "available")
    companion object {
        fun from(json: JSONObject) = ResultDescriptor(json.getString("path"), json.getString("kind"),
            json.getString("version"), json.getLong("bytes"), json.getString("sha256"), json.getBoolean("artifact"))
    }
}

/** App-owned bytes only. All I/O is called on a worker; readers/exporters pin jobs.
 * Native output must be released before import. Original bytes are never parsed/reencoded.
 */
class ResultStore(private val root: File, private val freeBytes: () -> Long = { root.usableSpace },
                  private val maxJobs: Int = 32, private val quota: Long = 512L * 1024 * 1024,
                  private val reservationChanged: (Long) -> Unit = {}) {
    companion object { const val DOCUMENT_LIMIT = 64L * 1024 * 1024 }
    private val leases = mutableMapOf<String, Int>()
    private val reserved = mutableMapOf<String, Long>()
    private fun directory(attempt: String): File {
        UUID.fromString(attempt)
        val dir = File(root, attempt)
        if (dir.canonicalFile.parentFile != root.canonicalFile) throw InputFailure("invalid_request")
        return dir
    }
    private fun owned(attempt: String, path: String): File {
        val dir = directory(attempt)
        if (File(path).isAbsolute || path.contains('/') || path.contains('\\') || path == "." || path == "..") throw InputFailure("invalid_request")
        val file = File(dir, path)
        if (file.canonicalFile.parentFile != dir.canonicalFile) throw InputFailure("invalid_request")
        return file
    }
    @Synchronized fun pin(attempt: String): AutoCloseable {
        directory(attempt)
        leases[attempt] = (leases[attempt] ?: 0) + 1
        var closed = false
        return AutoCloseable { synchronized(this) {
            if (!closed) { closed = true; val count = leases.getValue(attempt) - 1
                if (count == 0) leases.remove(attempt) else leases[attempt] = count }
        } }
    }
    private fun history() = root.listFiles().orEmpty().filter { it.isDirectory && File(it, "manifest.json").isFile }
        .sortedBy { readJson(File(it, "manifest.json")).getLong("created_ms") }
    private fun used() = root.walkTopDown().filter { it.isFile }.sumOf { it.length() }
    @Synchronized fun reserve(attempt: String, bytes: Long) {
        if (!root.isDirectory && !root.mkdirs()) throw InputFailure("storage_full")
        if (bytes < 0 || bytes > quota) throw InputFailure("storage_full")
        val dir = directory(attempt)
        while (used() + reserved.filterKeys { it != attempt }.values.sum() + bytes > quota ||
            history().count { it.name != attempt } >= maxJobs) {
            val oldest = history().firstOrNull { it.name != attempt && it.name !in leases && it.name !in reserved }
                ?: throw InputFailure("storage_full")
            delete(oldest.name)
        }
        if (freeBytes() < InputStore.DISK_MARGIN + reserved.filterKeys { it != attempt }.values.sum() + bytes) throw InputFailure("storage_full")
        reserved[attempt] = bytes
        if (!dir.isDirectory && !dir.mkdir()) throw InputFailure("storage_full")
        reservationChanged(reserved.values.sum())
    }
    @Synchronized fun import(attempt: String, source: File, kind: String, version: String, artifact: Boolean = false): ResultDescriptor {
        val size = source.length()
        if (!source.isFile || size > DOCUMENT_LIMIT) throw InputFailure("size_limit")
        val remaining = reserved[attempt] ?: throw InputFailure("storage_full")
        if (size > remaining) throw InputFailure("storage_full")
        val path = UUID.randomUUID().toString() + ".payload"
        val partial = owned(attempt, "$path.partial")
        Files.createFile(partial.toPath())
        val digest = MessageDigest.getInstance("SHA-256")
        try {
            var count = 0L
            source.inputStream().use { input -> FileOutputStream(partial).use { output ->
                val buffer = ByteArray(InputStore.BUFFER_BYTES)
                while (true) {
                    val read = input.read(buffer); if (read < 0) break
                    count += read
                    if (read == 0 || count > size || freeBytes() < InputStore.DISK_MARGIN + reserved.values.sum() - count + read) throw InputFailure("storage_full")
                    output.write(buffer, 0, read); digest.update(buffer, 0, read)
                }
                if (count != size) throw InputFailure("input_changed")
                output.fd.sync()
            } }
            Files.move(partial.toPath(), owned(attempt, path).toPath(), StandardCopyOption.ATOMIC_MOVE)
            reserved[attempt] = remaining - size
            reservationChanged(reserved.values.sum())
            return ResultDescriptor(path, kind, version, size, digest.digest().joinToString("") { "%02x".format(it) }, artifact)
        } finally { partial.delete() }
    }
    @Synchronized fun commit(job: JobRecord, descriptors: List<ResultDescriptor>, outcomes: JSONArray) {
        require(descriptors.size <= 64 && outcomes.length() <= 32)
        descriptors.forEach { validate(job.attemptId, it) }
        val manifest = JSONObject().put("version", 1).put("job_id", job.jobId).put("attempt_id", job.attemptId)
            .put("feature_id", job.feature).put("state", job.state).put("created_ms", job.createdMs)
            .put("options", JSONObject(job.options))
            .put("payloads", JSONArray(descriptors.map { it.json() })).put("outcomes", outcomes)
        val manifestBytes = manifest.toString().toByteArray(Charsets.UTF_8).size.toLong()
        if ((reserved[job.attemptId] ?: 0) < manifestBytes) reserve(job.attemptId, manifestBytes)
        atomicJson(File(directory(job.attemptId), "manifest.json"), manifest)
        reserved.remove(job.attemptId)
        reservationChanged(reserved.values.sum())
    }
    @Synchronized fun releaseReservation(attempt: String) { reserved.remove(attempt); reservationChanged(reserved.values.sum()) }
    @Synchronized fun validate(attempt: String, descriptor: ResultDescriptor): File {
        val file = owned(attempt, descriptor.path)
        if (!file.isFile || file.length() != descriptor.bytes || descriptor.bytes > DOCUMENT_LIMIT) throw InputFailure("invalid_payload")
        val digest = MessageDigest.getInstance("SHA-256")
        file.inputStream().use { input -> val buffer = ByteArray(InputStore.BUFFER_BYTES)
            while (true) { val count = input.read(buffer); if (count < 0) break; digest.update(buffer, 0, count) } }
        if (digest.digest().joinToString("") { "%02x".format(it) } != descriptor.sha256) throw InputFailure("invalid_payload")
        return file
    }
    fun dispatch(descriptor: ResultDescriptor, supported: Set<Pair<String, String>>) =
        if (descriptor.kind to descriptor.version in supported) "available" else "unsupported_version"
    @Synchronized fun entries(): List<JSONObject> = history().map { readJson(File(it, "manifest.json")) }
    @Synchronized fun load(attempt: String): JSONObject {
        val json = readJson(File(directory(attempt), "manifest.json"))
        if (json.getInt("version") != 1) throw InputFailure("unsupported_version")
        if (json.getString("attempt_id") != attempt) throw InputFailure("invalid_payload")
        val payloads = json.getJSONArray("payloads")
        if (payloads.length() > 64 || json.getJSONArray("outcomes").length() > 32) throw InputFailure("size_limit")
        for (index in 0 until payloads.length()) validate(attempt, ResultDescriptor.from(payloads.getJSONObject(index)))
        return json
    }
    @Synchronized fun delete(attempt: String) {
        if (attempt in leases || attempt in reserved) throw InputFailure("busy")
        if (!directory(attempt).deleteRecursively()) throw InputFailure("io_error")
    }
    @Synchronized fun recover() {
        check(leases.isEmpty() && reserved.isEmpty())
        if (!root.isDirectory && !root.mkdirs()) throw InputFailure("storage_full")
        root.listFiles().orEmpty().forEach { dir ->
            if (!dir.isDirectory) throw InputFailure("invalid_payload")
            UUID.fromString(dir.name)
            val manifest = File(dir, "manifest.json")
            if (manifest.exists() || File(dir, "manifest.json.bak").exists()) {
                val json = readJson(manifest)
                val payloads = json.getJSONArray("payloads")
                val paths = (0 until payloads.length()).map { payloads.getJSONObject(it).getString("path") }.toSet()
                dir.listFiles().orEmpty().filter { it.name !in paths && it.name != "manifest.json" }.forEach { it.delete() }
            } else if (!dir.deleteRecursively()) throw InputFailure("io_error")
        }
    }
    fun export(context: Context, attempt: String, descriptor: ResultDescriptor, destination: Uri) {
        pin(attempt).use {
            val file = validate(attempt, descriptor)
            context.contentResolver.openOutputStream(destination, "w")?.use { output -> file.inputStream().use { it.copyTo(output, InputStore.BUFFER_BYTES) } }
                ?: throw InputFailure("io_error")
        }
    }
    /** Share an independent, bounded expiring copy; FileProvider exposes only this cache. */
    fun share(context: Context, attempt: String, descriptor: ResultDescriptor): Uri {
        pin(attempt).use {
            val source = validate(attempt, descriptor)
            val shares = File(context.cacheDir, "result-shares").apply { mkdirs() }
            synchronized(shares.canonicalPath.intern()) {
                shares.listFiles().orEmpty().filter { System.currentTimeMillis() - it.lastModified() > 10 * 60 * 1000 }.forEach { it.delete() }
                if (shares.listFiles().orEmpty().size >= 32 || shares.listFiles().orEmpty().sumOf { it.length() } + source.length() > 128L * 1024 * 1024) throw InputFailure("storage_full")
                if (shares.usableSpace < InputStore.DISK_MARGIN + source.length()) throw InputFailure("storage_full")
                val copy = File(shares, UUID.randomUUID().toString() + ".payload")
                Files.copy(source.toPath(), copy.toPath())
                return FileProvider.getUriForFile(context, context.packageName + ".results", copy)
            }
        }
    }
}
