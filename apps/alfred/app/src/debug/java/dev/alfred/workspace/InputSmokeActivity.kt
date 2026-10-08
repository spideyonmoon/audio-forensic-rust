package dev.alfred.workspace

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.os.ParcelFileDescriptor
import android.provider.DocumentsContract
import dev.alfred.shared.*
import org.json.JSONArray
import org.json.JSONObject
import java.io.ByteArrayInputStream
import java.io.File
import java.io.InputStream
import java.security.MessageDigest
import java.util.UUID
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit

/** Runs the actual shared input path against a generated DocumentsProvider. */
class InputSmokeActivity : Activity() {
    private val checks = JSONArray()
    override fun onCreate(state: Bundle?) {
        super.onCreate(state)
        Thread({
            val receipt = File(filesDir, "input-smoke.json")
            try {
                smoke()
                receipt.writeText(JSONObject().put("passed", true).put("checks", checks)
                    .put("api", android.os.Build.VERSION.SDK_INT).toString())
            } catch (error: Throwable) {
                receipt.writeText(JSONObject().put("passed", false).put("checks", checks).put("error", error.stackTraceToString()).toString())
            }
            runOnUiThread { finish() }
        }, "alfred-input-smoke").start()
    }
    private fun uri(id: String) = DocumentsContract.buildDocumentUri(InputTestProvider.AUTHORITY, id)
    private fun picker(vararg ids: String) = PickerResult(ids.map { uri(it) }, Intent.FLAG_GRANT_READ_URI_PERMISSION, false)
    private fun tree(id: String) = PickerResult(listOf(DocumentsContract.buildTreeDocumentUri(InputTestProvider.AUTHORITY, id)),
        Intent.FLAG_GRANT_READ_URI_PERMISSION, true)
    private fun expect(code: String, action: () -> Unit) {
        try { action(); error("Expected $code") } catch (error: InputFailure) { check(error.code == code) { "${error.code} != $code" } }
    }
    private fun smoke() {
        check(java.math.BigInteger.valueOf(Long.MAX_VALUE).checkedPositiveLong() == Long.MAX_VALUE)
        expect("invalid_request") { java.math.BigInteger("18446744073709551615").checkedPositiveLong() }
        val selection = SafSelection(contentResolver)
        try {
            val contract = SafPicker()
            val intent = contract.createIntent(this, Unit)
            check(intent.action == Intent.ACTION_OPEN_DOCUMENT && intent.getBooleanExtra(Intent.EXTRA_ALLOW_MULTIPLE, false))
            check(intent.flags and Intent.FLAG_GRANT_WRITE_URI_PERMISSION == 0)
            val clip = android.content.ClipData.newRawUri("generated", uri("wav"))
            repeat(40) { clip.addItem(android.content.ClipData.Item(uri("wav"))) }
            intent.data = uri("wav"); intent.clipData = clip
            check(contract.parseResult(RESULT_OK, intent)!!.let { it.uris.size == 1 && !it.overflow })
            val single = selection.read(picker("wav", "wav"), 1, InputCancellation())
            check(single.items.size == 1 && single.items.single().grantState == "session_only")
            check(selection.read(picker("wav", "pipe"), 2, InputCancellation()).items.size == 2) // Equal names, distinct documents.
            expect("selection_limit") { selection.read(PickerResult((0..32).map { uri("$it") }, 0, false), 3, InputCancellation()) }
            val small = selection.read(tree("small"), 4, InputCancellation())
            check(small.items.size == 3 && !small.incomplete)
            check(small.items == small.items.sortedWith(compareBy< InputItem > { it.name }.thenBy { it.identity }))
            check(selection.read(tree("candidates"), 5, InputCancellation()).let { it.incomplete && it.items.size == 32 })
            check(selection.read(tree("records"), 6, InputCancellation()).let { it.incomplete && it.items.isEmpty() })
            check(boundedName("🎵".repeat(400)).toByteArray().size == 1024)
            val failedGrant = selection.read(picker("wav").copy(flags = Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION), 7, InputCancellation())
            check(failedGrant.items.single().grantState == "session_only_grant_failed")
            checks.put("single/multiple/dedup/equal-names/folder/nonrecursive/512/32/name-limit/session-grant")
        } finally { selection.close() }

        val directory = File(filesDir, "input-test-store").apply { mkdirs() }
        var free = Long.MAX_VALUE
        val store = InputStore(directory) { free }
        fun acquire(id: String, declared: Long? = null, token: InputCancellation = InputCancellation(), hash: String? = null) =
            store.acquire(UUID.randomUUID().toString(), declared, token, hash) {
                ParcelFileDescriptor.AutoCloseInputStream(contentResolver.openFileDescriptor(uri(id), "r", token.signal)!!)
            }
        val wav = acquire("wav")
        val encodedHash = wav.sha256
        val nativePin = wav.retain()
        wav.close(); wav.close()
        check(wav.file.exists() && wav.state == "in_use")
        expect("busy") { acquire("pipe") }
        expect("busy") { InputStore(directory).acquire(UUID.randomUUID().toString(), null, InputCancellation()) { ByteArrayInputStream(byteArrayOf(1)) } }
        nativePin.close(); nativePin.close()
        check(!wav.file.exists() && wav.state == "released")
        acquire("pipe").use { check(it.sha256 == encodedHash) }
        acquire("wav", 1).use { check(it.bytes > 1 && it.sha256 == encodedHash) } // Actual count is authoritative.
        val changed = acquire("changed")
        val previousHash = changed.sha256
        changed.close(); InputTestProvider.changed = true
        expect("input_changed") { acquire("changed", hash = previousHash) }
        InputTestProvider.changed = false
        expect("permission_denied") { acquire("revoked") }
        expect("input_missing") { acquire("missing-open") }
        free = InputStore.DISK_MARGIN
        expect("storage_full") { acquire("wav", 1000) }
        free = Long.MAX_VALUE
        expect("size_limit") { acquire("large", InputStore.MAX_BYTES + 1) }
        val token = InputCancellation()
        expect("cancelled") {
            store.acquire(UUID.randomUUID().toString(), null, token) {
                object : InputStream() {
                    override fun read() = error("bounded read required")
                    override fun read(b: ByteArray, off: Int, len: Int): Int { token.cancel(); return len }
                }
            }
        }
        expect("storage_full") {
            store.acquire(UUID.randomUUID().toString(), null, InputCancellation()) {
                object : ByteArrayInputStream(ByteArray(512 * 1024)) {
                    override fun read(b: ByteArray, off: Int, len: Int): Int { free = InputStore.DISK_MARGIN; return super.read(b, off, len) }
                }
            }
        }
        free = Long.MAX_VALUE
        check(directory.listFiles()!!.isEmpty())
        val orphan = File(directory, "generated-orphan.partial").apply { writeText("generated") }
        expect("interrupted") { acquire("wav") }
        check(orphan.delete())
        val output = OwnedDirectory.create(File(filesDir, "output-test-store"), UUID.randomUUID().toString())
        File(output.file, "payload").writeText("generated")
        val reader = output.retain()
        output.close(); check(output.file.exists())
        reader.close(); check(!output.file.exists())
        checks.put("seekable/pipe/unknown-size/hash/leases/change/revoked/missing/disk-pressure/cancel/partial-cleanup")

        // Actual 700 MiB copy from an unknown-length, nonseekable pipe. No source
        // file or whole-audio allocation. Independently hash generated zero chunks.
        val digest = MessageDigest.getInstance("SHA-256")
        val zeros = ByteArray(256 * 1024)
        repeat((InputStore.MAX_BYTES / zeros.size).toInt()) { digest.update(zeros) }
        val expected = digest.digest().joinToString("") { "%02x".format(it) }
        acquire("large").use { check(it.bytes == InputStore.MAX_BYTES && it.sha256 == expected) }
        expect("size_limit") { acquire("oversize") }
        check(directory.listFiles()!!.isEmpty())
        checks.put("exact-700-MiB/700-MiB-plus-one-rejected/bounded-buffer/large-cleanup")

        val events = LinkedBlockingQueue<WorkspaceState>()
        val inputs = WorkspaceInput(this) { events.offer(it) }
        fun settled(): WorkspaceState {
            val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(90)
            while (System.nanoTime() < deadline) {
                val event = events.poll(1, TimeUnit.SECONDS) ?: continue
                if (!event.busy) return event
            }
            error("input coordinator timeout")
        }
        try {
            inputs.select(picker("denied"))
            check(settled().notice == "permission_denied")
            inputs.select(picker("missing"))
            check(settled().notice == "input_missing")
            inputs.select(tree("candidates"))
            val bounded = settled()
            val boundedSelection = checkNotNull(bounded.selection)
            check(boundedSelection.incomplete && bounded.probes.isEmpty())
            inputs.confirmFolder(boundedSelection.items.take(2).map { it.id }.toSet())
            check(settled().let { !it.selection!!.incomplete && it.probes.size == 2 })
            inputs.select(picker("wav", "flac", "alac", "aac"))
            val probed = settled()
            check(probed.probes.size == 4) { probed.toString() }
            check(probed.probes.values.count { it.status == "available" } == 3) { probed.toString() }
            val forensics = Operation(FeatureId.FORENSICS, "Forensics", 1, 32)
            val compare = Operation(FeatureId.COMPARE, "Compare", 2, 32)
            check(operationCapability(forensics, probed).state == "available")
            check(operationCapability(compare, probed).state == "unavailable")
            inputs.select(picker("wav", "pipe"))
            val compatible = settled()
            check(operationCapability(compare, compatible).state == "needs_input")
            check(operationCapability(compare, compatible.copy(sameTrack = true)).state == "available")
            inputs.featureInputs()!!.let { access ->
                access.acquire(access.selection.items.first(), UUID.randomUUID().toString(), InputCancellation()).use {
                    check(it.sha256 == encodedHash)
                }
            }
            events.clear()
            inputs.select(picker("large"))
            Thread.sleep(25)
            inputs.select(picker("wav"))
            val latest = settled()
            check(latest.selection!!.items.single().identity.endsWith(":wav")) { latest.toString() }
            check(latest.probes.values.single().status == "available")
            checks.put("actual-codec-ALAC-vs-AAC/mixed-routing/same-track/shared-feature-acquisition/selection-change")
            check(File(filesDir, "input-snapshots").listFiles()!!.isEmpty())
            check(File(filesDir, "input-probes").listFiles()!!.isEmpty())
            checks.put("native-release/snapshot/probe-output-cleanup")
        } finally { inputs.close() }
    }
}
