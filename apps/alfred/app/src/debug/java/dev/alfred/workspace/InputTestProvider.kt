package dev.alfred.workspace

import android.database.Cursor
import android.database.MatrixCursor
import android.os.CancellationSignal
import android.os.ParcelFileDescriptor
import android.provider.DocumentsContract.Document
import android.provider.DocumentsContract.Root
import android.provider.DocumentsProvider
import java.io.File
import java.io.FileNotFoundException

/** Generated-only provider, absent from release. Includes pipe/unknown-length,
 * changing, missing and revoked inputs and finite folder enumeration controls.
 */
class InputTestProvider : DocumentsProvider() {
    companion object {
        const val AUTHORITY = "dev.alfred.workspace.debug.inputs"
        @Volatile var changed = false
    }
    override fun onCreate() = true
    override fun queryRoots(projection: Array<out String>?): Cursor {
        val columns = projection ?: arrayOf(Root.COLUMN_ROOT_ID, Root.COLUMN_DOCUMENT_ID, Root.COLUMN_TITLE, Root.COLUMN_FLAGS)
        return MatrixCursor(columns).apply { newRow().apply {
            columns.forEach { column -> add(column, when (column) {
                Root.COLUMN_ROOT_ID -> "generated"
                Root.COLUMN_DOCUMENT_ID -> "small"
                Root.COLUMN_TITLE -> "Alfred generated inputs"
                Root.COLUMN_FLAGS -> Root.FLAG_SUPPORTS_IS_CHILD or Root.FLAG_LOCAL_ONLY
                else -> null
            }) }
        } }
    }
    private fun columns(projection: Array<out String>?) = projection ?: arrayOf(Document.COLUMN_DOCUMENT_ID,
        Document.COLUMN_DISPLAY_NAME, Document.COLUMN_SIZE, Document.COLUMN_MIME_TYPE, Document.COLUMN_FLAGS)
    private fun row(cursor: MatrixCursor, id: String, directory: Boolean = false) {
        val name = when (id) { "wav", "pipe", "changed" -> "same.wav"; "flac" -> "same.flac"; "alac", "aac" -> "$id.m4a"; else -> "$id.wav" }
        cursor.newRow().apply {
            cursor.columnNames.forEach { column -> add(column, when (column) {
                Document.COLUMN_DOCUMENT_ID -> id
                Document.COLUMN_DISPLAY_NAME -> if (directory) id else name
                Document.COLUMN_MIME_TYPE -> if (directory) Document.MIME_TYPE_DIR else "audio/x-wav"
                Document.COLUMN_SIZE -> if (id == "pipe" || id == "large" || id == "oversize") null else if (directory) null else fixture(id).length()
                Document.COLUMN_FLAGS -> 0
                else -> null
            }) }
        }
    }
    override fun queryDocument(documentId: String, projection: Array<out String>?): Cursor {
        if (documentId == "denied") throw SecurityException("generated permission denial")
        if (documentId == "missing") throw FileNotFoundException("generated missing input")
        return MatrixCursor(columns(projection)).apply { row(this, documentId, documentId in setOf("small", "candidates", "records")) }
    }
    override fun queryChildDocuments(parentDocumentId: String, projection: Array<out String>?, sortOrder: String?): Cursor =
        MatrixCursor(columns(projection)).apply {
            when (parentDocumentId) {
                "small" -> { row(this, "wav"); row(this, "pipe"); row(this, "flac"); row(this, "subfolder", true) }
                "candidates" -> repeat(40) { row(this, "candidate-$it") }
                "records" -> repeat(600) { row(this, "directory-$it", true) }
                else -> throw FileNotFoundException("generated missing folder")
            }
        }
    override fun isChildDocument(parentDocumentId: String, documentId: String) = parentDocumentId in setOf("small", "candidates", "records")
    private fun fixture(id: String): File {
        val file = File(context!!.cacheDir, "provider-$id")
        if (!file.exists()) {
            val asset = when (id) { "alac", "aac" -> "alac/8000-16-1-tail.m4a"; "flac" -> "noise16.flac"; else -> "noise16.wav" }
            context!!.assets.open(asset).use { source -> file.outputStream().use { source.copyTo(it) } }
            if (id == "aac") {
                val bytes = file.readBytes()
                val marker = "alac".toByteArray()
                val offset = (0..bytes.size - 4).first { i -> bytes.copyOfRange(i, i + 4).contentEquals(marker) }
                "mp4a".toByteArray().copyInto(bytes, offset); file.writeBytes(bytes)
            }
        }
        return file
    }
    override fun openDocument(documentId: String, mode: String, signal: CancellationSignal?): ParcelFileDescriptor {
        require(mode == "r")
        signal?.throwIfCanceled()
        if (documentId == "revoked") throw SecurityException("generated revoked grant")
        if (documentId == "missing-open") throw FileNotFoundException("generated missing input")
        if (documentId in setOf("pipe", "large", "oversize")) {
            val pipe = ParcelFileDescriptor.createPipe()
            Thread({
                try {
                    ParcelFileDescriptor.AutoCloseOutputStream(pipe[1]).use { output ->
                        if (documentId == "pipe") fixture("wav").inputStream().use { it.copyTo(output) }
                        else {
                            val buffer = ByteArray(256 * 1024)
                            var left = 700L * 1024 * 1024 + if (documentId == "oversize") 1 else 0
                            while (left > 0) {
                                signal?.throwIfCanceled()
                                val count = minOf(left, buffer.size.toLong()).toInt()
                                output.write(buffer, 0, count); left -= count
                            }
                        }
                    }
                } catch (_: Exception) { try { pipe[1].close() } catch (_: Exception) { } }
            }, "generated-provider").start()
            return pipe[0]
        }
        val file = fixture(documentId)
        if (documentId == "changed" && changed) {
            // Same encoded length, different bytes after capability negotiation.
            val replacement = File(context!!.cacheDir, "provider-changed-second")
            replacement.writeBytes(file.readBytes().also { it[it.lastIndex] = (it.last() + 1).toByte() })
            return ParcelFileDescriptor.open(replacement, ParcelFileDescriptor.MODE_READ_ONLY)
        }
        return ParcelFileDescriptor.open(file, ParcelFileDescriptor.MODE_READ_ONLY)
    }
}
