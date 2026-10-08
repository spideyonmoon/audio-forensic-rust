package dev.alfred.shared

import android.app.Activity
import android.content.ContentResolver
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.provider.DocumentsContract
import android.provider.OpenableColumns
import androidx.activity.result.contract.ActivityResultContract
import java.util.UUID

data class PickerResult(val uris: List<Uri>, val flags: Int, val tree: Boolean, val overflow: Boolean = false)

fun documentIdentity(uri: Uri): String = if (uri.scheme == "content" && "document" in uri.pathSegments) {
    "${uri.authority}:${DocumentsContract.getDocumentId(uri)}"
} else uri.toString()

/** Preserve returned flags: taking a persisted grant is conditional on the offer. */
class SafPicker(private val tree: Boolean = false, private val multiple: Boolean = true) : ActivityResultContract<Unit, PickerResult?>() {
    override fun createIntent(context: Context, input: Unit) = Intent(
        if (tree) Intent.ACTION_OPEN_DOCUMENT_TREE else Intent.ACTION_OPEN_DOCUMENT
    ).apply {
        addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION)
        if (!tree) { type = "*/*"; addCategory(Intent.CATEGORY_OPENABLE); putExtra(Intent.EXTRA_ALLOW_MULTIPLE, multiple) }
    }
    override fun parseResult(resultCode: Int, intent: Intent?): PickerResult? {
        if (resultCode != Activity.RESULT_OK || intent == null) return null
        val uris = linkedMapOf<String, Uri>()
        intent.data?.let { uris[documentIdentity(it)] = it }
        intent.clipData?.let { clip ->
            // Collapse identities while reading; retain only the first 33 unique
            // items so over-limit selections fail without an unbounded allocation.
            for (i in 0 until clip.itemCount) {
                val uri = clip.getItemAt(i).uri
                uris.putIfAbsent(documentIdentity(uri), uri)
                if (uris.size > 32) break
            }
        }
        return PickerResult(uris.values.toList(), intent.flags, tree, uris.size > 32)
    }
}

data class InputItem(val id: String, val uri: Uri, val identity: String, val name: String,
                     val declaredBytes: Long?, val grantState: String)
data class InputSelection(val id: String, val generation: Long, val items: List<InputItem>, val incomplete: Boolean = false)

fun boundedName(value: String): String {
    val result = StringBuilder()
    var bytes = 0
    var offset = 0
    while (offset < value.length) {
        val point = value.codePointAt(offset)
        val text = String(Character.toChars(point))
        val size = text.toByteArray(Charsets.UTF_8).size
        if (bytes + size > 1024) break
        result.append(text); bytes += size; offset += Character.charCount(point)
    }
    return result.toString()
}

/** A grant batch owns only newly taken grants. Existing persisted user grants
 * are never revoked by selection replacement. Release after provider copy exits.
 */
class SafSelection(private val resolver: ContentResolver) : AutoCloseable {
    companion object {
        private val ownedGrants = mutableMapOf<Uri, Int>()
        private var ledger: java.io.File? = null
        private fun journal() { ledger?.let { atomicJson(it, org.json.JSONObject().put("version", 1)
            .put("uris", org.json.JSONArray(ownedGrants.keys.map { uri -> uri.toString() }))) } }
        internal fun recoverGrants(context: Context) = synchronized(ownedGrants) {
            check(ownedGrants.isEmpty())
            ledger = java.io.File(context.filesDir, "owned-input-grants.json")
            val file = ledger!!
            if (file.exists() || java.io.File(file.path + ".bak").exists()) {
                val json = readJson(file)
                if (json.getInt("version") != 1) throw InputFailure("unsupported_version")
                val uris = json.getJSONArray("uris")
                for (i in 0 until uris.length()) try {
                    context.contentResolver.releasePersistableUriPermission(Uri.parse(uris.getString(i)), Intent.FLAG_GRANT_READ_URI_PERMISSION)
                } catch (_: SecurityException) { }
            }
            journal()
        }
    }
    private val taken = mutableSetOf<Uri>()
    private var closed = false
    private fun grantIdentity(uri: Uri): Uri = if ("tree" in uri.pathSegments) {
        DocumentsContract.buildTreeDocumentUri(uri.authority, DocumentsContract.getTreeDocumentId(uri))
    } else uri
    private fun grant(uri: Uri, flags: Int): String {
        val key = grantIdentity(uri)
        synchronized(ownedGrants) {
            if (key in taken) return "persisted"
            if (key in ownedGrants) {
                ownedGrants[key] = ownedGrants.getValue(key) + 1; taken.add(key); return "persisted"
            }
            if (resolver.persistedUriPermissions.any { it.uri == key && it.isReadPermission }) return "persisted"
        if (flags and Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION == 0 || flags and Intent.FLAG_GRANT_READ_URI_PERMISSION == 0) return "session_only"
        return try {
            ownedGrants[key] = 1; journal() // Record ownership intent before taking a new grant.
            resolver.takePersistableUriPermission(key, Intent.FLAG_GRANT_READ_URI_PERMISSION)
            taken.add(key); ownedGrants[key] = 1; "persisted"
        } catch (_: SecurityException) { ownedGrants.remove(key); journal(); "session_only_grant_failed" }
        }
    }
    @Suppress("UNUSED_PARAMETER")
    fun adoptFrom(previous: SafSelection?, result: PickerResult) {
        // read() already retained registered app-owned grants before old release.
    }
    fun retain(): AutoCloseable = synchronized(ownedGrants) {
        check(!closed)
        val keys = taken.toList()
        keys.forEach { ownedGrants[it] = ownedGrants.getValue(it) + 1 }
        var released = false
        AutoCloseable { synchronized(ownedGrants) { if (!released) { released = true; release(keys) } } }
    }
    private fun release(keys: List<Uri>) {
        keys.forEach { key ->
            val count = ownedGrants.getValue(key) - 1
            if (count == 0) {
                try { resolver.releasePersistableUriPermission(key, Intent.FLAG_GRANT_READ_URI_PERMISSION) } catch (_: SecurityException) { }
                ownedGrants.remove(key)
                journal()
            } else ownedGrants[key] = count
        }
    }
    fun identity(uri: Uri): String = documentIdentity(uri)

    fun read(result: PickerResult, generation: Long, cancel: InputCancellation): InputSelection {
        if (result.overflow) throw InputFailure("selection_limit")
        val items = linkedMapOf<String, InputItem>()
        var incomplete = false
        if (result.tree) {
            val tree = result.uris.singleOrNull() ?: throw InputFailure("invalid_request")
            val grant = grant(tree, result.flags)
            val parent = DocumentsContract.getTreeDocumentId(tree)
            val children = DocumentsContract.buildChildDocumentsUriUsingTree(tree, parent)
            val projection = arrayOf(DocumentsContract.Document.COLUMN_DOCUMENT_ID, DocumentsContract.Document.COLUMN_DISPLAY_NAME,
                DocumentsContract.Document.COLUMN_SIZE, DocumentsContract.Document.COLUMN_MIME_TYPE)
            resolver.query(children, projection, null, null, null, cancel.signal)?.use { cursor ->
                var records = 0
                while (cursor.moveToNext()) {
                    cancel.check(); records++
                    val id = cursor.getString(0)
                    val name = boundedName(cursor.getString(1) ?: "Document")
                    val mime = cursor.getString(3) ?: ""
                    if (mime != DocumentsContract.Document.MIME_TYPE_DIR &&
                        (mime.startsWith("audio/") || name.substringAfterLast('.', "").lowercase() in setOf("flac", "wav", "wave", "m4a", "alac", "aac", "mp3", "ogg", "opus", "aiff", "dsf", "dff"))) {
                        val uri = DocumentsContract.buildDocumentUriUsingTree(tree, id)
                        val identity = "${tree.authority}:$id"
                        items.putIfAbsent(identity, InputItem(UUID.randomUUID().toString(), uri, identity, name,
                            if (cursor.isNull(2)) null else cursor.getLong(2).takeIf { it >= 0 }, grant))
                    }
                    if (records >= 512 || items.size >= 32) { incomplete = true; break }
                }
            } ?: throw InputFailure("input_missing")
        } else {
            // Collapse document identity before enforcing the direct-selection cap.
            val unique = result.uris.distinctBy { identity(it) }
            if (unique.size > 32) throw InputFailure("selection_limit")
            for (uri in unique) {
                cancel.check()
                val grant = grant(uri, result.flags)
                resolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE), null, null, null, cancel.signal)?.use { cursor ->
                    if (!cursor.moveToFirst()) throw InputFailure("input_missing")
                    val nameColumn = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME)
                    val sizeColumn = cursor.getColumnIndex(OpenableColumns.SIZE)
                    val name = boundedName(if (nameColumn >= 0) cursor.getString(nameColumn) ?: "Document" else "Document")
                    val size = if (sizeColumn < 0 || cursor.isNull(sizeColumn)) null else cursor.getLong(sizeColumn).takeIf { it >= 0 }
                    val identity = identity(uri)
                    items[identity] = InputItem(UUID.randomUUID().toString(), uri, identity, name, size, grant)
                } ?: throw InputFailure("input_missing")
            }
        }
        cancel.check()
        val ordered = if (result.tree) items.values.sortedWith(compareBy<InputItem> { it.name }.thenBy { it.identity }) else items.values.toList()
        return InputSelection(UUID.randomUUID().toString(), generation, ordered, incomplete)
    }
    override fun close() {
        synchronized(ownedGrants) { if (!closed) { closed = true; release(taken.toList()); taken.clear() } }
    }
}
