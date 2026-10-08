package dev.alfred.shared

import java.io.File
import java.io.FileOutputStream
import java.io.InputStream
import java.nio.file.Files
import java.nio.file.StandardCopyOption
import java.security.MessageDigest
import java.util.UUID
import java.util.concurrent.atomic.AtomicBoolean

class InputFailure(val code: String) : Exception(code)

class InputCancellation {
    private val cancelled = AtomicBoolean(false)
    val signal = android.os.CancellationSignal()
    fun cancel() { cancelled.set(true); signal.cancel() }
    fun check() { if (cancelled.get()) throw InputFailure("cancelled") }
}

/** Counted ownership for snapshots and unadopted probe output directories.
 * A native pin is released only after terminal leases_released or active_handle exit.
 */
class OwnedInput internal constructor(
    val id: String, val attemptId: String, val file: File, val bytes: Long,
    val sha256: String, private val deleted: () -> Unit
) : AutoCloseable {
    private var references = 1
    val state: String @Synchronized get() = when { references == 0 -> "released"; references == 1 && !ownerReleased.get() -> "ready"; else -> "in_use" }
    @Synchronized fun retain(): AutoCloseable {
        check(references > 0)
        references++
        val released = AtomicBoolean(false)
        return AutoCloseable { if (released.compareAndSet(false, true)) release() }
    }
    private val ownerReleased = AtomicBoolean(false)
    override fun close() { if (ownerReleased.compareAndSet(false, true)) release() }
    @Synchronized private fun release() {
        check(references > 0)
        references--
        if (references == 0) deleted()
    }
}

/** Output readers/exporters use the same counted ownership rule as native work.
 * Only a freshly created private directory is eligible for this lease.
 */
class OwnedDirectory private constructor(val file: File, parent: File, attempt: String) : AutoCloseable {
    private val owner = OwnedInput(UUID.randomUUID().toString(), attempt, file, 0, "") {
        check(file.canonicalFile.parentFile == parent.canonicalFile)
        if (!file.deleteRecursively()) throw InputFailure("io_error")
    }
    fun retain() = owner.retain()
    override fun close() = owner.close()
    companion object {
        fun create(parent: File, attempt: String): OwnedDirectory {
            if (!parent.isDirectory && !parent.mkdirs()) throw InputFailure("storage_full")
            return OwnedDirectory(Files.createTempDirectory(parent.toPath(), "probe-").toFile(), parent, attempt)
        }
    }
}

/** One snapshot at a time; never holds encoded audio in memory. A05 will add
 * shared reservations/history/orphan recovery around this ownership primitive.
 */
class InputStore(private val directory: File, private val freeBytes: () -> Long = { directory.usableSpace }) {
    companion object {
        const val MAX_BYTES = 700L * 1024 * 1024
        const val STAGING_QUOTA = 768L * 1024 * 1024
        const val DISK_MARGIN = 256L * 1024 * 1024
        const val PROBE_OUTPUT = 64L * 1024 * 1024
        const val BUFFER_BYTES = 256 * 1024
        private val activeDirectories = mutableSetOf<String>()
        @Volatile internal var resultReservations = 0L
    }
    private var occupied = false
    private var slotKey: String? = null
    @Synchronized private fun claim() {
        if (occupied) throw InputFailure("busy")
        val key = directory.canonicalPath
        synchronized(activeDirectories) {
            if (key in activeDirectories) throw InputFailure("busy")
            // A recreated controller must not stage beside an old worker or an
            // interrupted process's orphan. A05 owns orphan recovery/deletion.
            if (directory.isDirectory && Files.newDirectoryStream(directory.toPath()).use { it.iterator().hasNext() }) {
                throw InputFailure("interrupted")
            }
            activeDirectories.add(key)
            slotKey = key
            occupied = true
        }
    }
    @Synchronized private fun releaseSlot() {
        synchronized(activeDirectories) { slotKey?.let { activeDirectories.remove(it) } }
        slotKey = null; occupied = false
    }
    fun admitWrite(bytes: Long) {
        if (freeBytes() < DISK_MARGIN + maxOf(PROBE_OUTPUT, resultReservations) + bytes) throw InputFailure("storage_full")
    }
    fun acquire(attempt: String, declaredBytes: Long?, cancellation: InputCancellation,
                expectedHash: String? = null, open: () -> InputStream): OwnedInput {
        claim()
        var partial: File? = null
        var ready: File? = null
        try {
            cancellation.check()
            if (declaredBytes != null && declaredBytes > MAX_BYTES) throw InputFailure("size_limit")
            if (!directory.isDirectory && !directory.mkdirs()) throw InputFailure("storage_full")
            admitWrite(declaredBytes?.coerceAtLeast(0) ?: BUFFER_BYTES.toLong())
            val id = UUID.randomUUID().toString()
            partial = File(directory, "$id.partial")
            // create-new is required even though UUID collisions are unlikely.
            Files.createFile(partial.toPath())
            val digest = MessageDigest.getInstance("SHA-256")
            var count = 0L
            open().use { source ->
                FileOutputStream(partial).use { output ->
                    val buffer = ByteArray(BUFFER_BYTES)
                    while (true) {
                        cancellation.check()
                        val read = source.read(buffer)
                        cancellation.check()
                        if (read < 0) break
                        if (read == 0) throw InputFailure("io_error")
                        if (count + read > MAX_BYTES || count + read > STAGING_QUOTA) throw InputFailure("size_limit")
                        admitWrite(read.toLong())
                        output.write(buffer, 0, read)
                        digest.update(buffer, 0, read)
                        count += read
                    }
                    output.fd.sync()
                }
            }
            cancellation.check()
            val hash = digest.digest().joinToString("") { "%02x".format(it) }
            if (expectedHash != null && hash != expectedHash) throw InputFailure("input_changed")
            ready = File(directory, "$id.ready")
            Files.move(partial.toPath(), ready.toPath(), StandardCopyOption.ATOMIC_MOVE)
            if (!ready.setReadOnly()) throw InputFailure("io_error")
            val ownedFile = ready
            return OwnedInput(id, attempt, ownedFile, count, hash) {
                if (!ownedFile.delete()) throw InputFailure("io_error")
                releaseSlot()
            }
        } catch (error: Exception) {
            partial?.delete()
            ready?.delete()
            releaseSlot()
            throw when (error) {
                is InputFailure -> error
                is SecurityException -> InputFailure("permission_denied")
                is java.io.FileNotFoundException -> InputFailure("input_missing")
                is android.os.OperationCanceledException -> InputFailure("cancelled")
                else -> InputFailure(if (freeBytes() < DISK_MARGIN + PROBE_OUTPUT) "storage_full" else "io_error")
            }
        }
    }
}
