package dev.alfred.compare

import androidx.compose.material3.Button
import androidx.compose.material3.Checkbox
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.platform.LocalContext
import dev.alfred.shared.*
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONObject

val compareOperation = Operation(FeatureId.COMPARE, "Audio Compare", 2, 32)

@Composable
fun CompareScreen(inputs: FeatureInputs?, sameTrack: Boolean, jobs: SharedJobs, records: List<JobRecord>, onExport: ResultAction, onShare: ResultAction) {
    val app = LocalContext.current.applicationContext
    val coroutine = rememberCoroutineScope()
    var seconds by rememberSaveable(inputs?.selection?.id) { mutableStateOf(inputs?.let { CompareWork.prefixBudget(it) }?.toString() ?: "") }
    var notice by remember { mutableStateOf("") }
    var submitting by remember { mutableStateOf(false) }
    Text("Audio Compare")
    Text("Reference-method comparison of variants of one track. The Rust method checks version, domain and actual coverage before ranking. Its tuple is uncalibrated; it is not a perceptual quality metric. Exact ties retain input order; incompatible/unavailable results have no winner.")
    Text(notice)
    if (inputs != null && sameTrack && inputs.selection.items.size in 2..32) {
        val budget = CompareWork.prefixBudget(inputs) ?: 0
        val prefix = seconds.toLongOrNull()
        val fullFits = inputs.selection.items.all { (inputs.probes[it.id]?.rate ?: Long.MAX_VALUE) <= 48000 }
        Text("${inputs.selection.items.size} declared same-track variants · common prefix budget: $budget seconds")
        OutlinedTextField(seconds, onValueChange = { seconds = it }, label = { Text("Common prefix seconds (whole number)") })
        fun start(scope: JSONObject) {
            submitting = true
            coroutine.launch {
                try { val record = withContext(Dispatchers.IO) { CompareWork.submit(app, inputs, scope, sameTrack).get() }; notice = "${record.state} · ${record.attemptId}" }
                catch (error: Exception) { notice = resultFailure(error) }
                finally { submitting = false }
            }
        }
        Button(enabled = !submitting && prefix != null && prefix > 0 && prefix <= budget,
            onClick = { start(JSONObject().put("kind", "prefix").put("seconds", prefix)) }) { Text("Compare all at the same prefix") }
        Button(enabled = !submitting && fullFits, onClick = { start(JSONObject().put("kind", "full")) }) { Text("Compare all at full scope") }
        Text("One scope applies unchanged to every reacquired snapshot. A shorter actual EOF remains shorter in its report. Missing input yields an explicit host failure and preserves already collected products.")
    } else Text("Select 2–32 supported variants and declare their same-track relationship in the workspace. Saved comparisons remain available.")
    var saved by remember { mutableStateOf<List<CompareWork.SavedProduct>>(emptyList()) }
    var selected by rememberSaveable { mutableStateOf<List<String>>(emptyList()) }
    var savedSameTrack by rememberSaveable { mutableStateOf(false) }
    var page by rememberSaveable { mutableStateOf(0) }
    LaunchedEffect(records) {
        try { saved = withContext(Dispatchers.IO) { jobs.awaitReady(); CompareWork.savedProducts(jobs.results) } }
        catch (error: Exception) { notice = resultFailure(error) }
    }
    Text("Compare saved products")
    Text("Original coverage is preserved. Rust decides compatibility; no saved report is rescoped. Selection order is the stable tie order.")
    saved.drop(page * 24).take(24).forEach { input ->
        val supported = input.descriptor.version == "audio-forensic-product-v1"
        Checkbox(input.id in selected, enabled = supported && !submitting && (input.id in selected || selected.size < 32), onCheckedChange = { checked ->
            selected = if (checked) selected + input.id else selected - input.id
        })
        Text("${input.attempt} · ${input.descriptor.path} · ${input.descriptor.bytes} bytes · ${if (supported) "product v1" else "unsupported_version"}")
    }
    if (page > 0) Button(onClick = { page-- }) { Text("Previous saved products") }
    if ((page + 1) * 24 < saved.size) Button(onClick = { page++ }) { Text("Next saved products") }
    Checkbox(savedSameTrack, onCheckedChange = { savedSameTrack = it })
    Text("These saved products are variants of the same track · ${selected.size} selected")
    Button(enabled = !submitting && savedSameTrack && selected.size in 2..32, onClick = {
        val expectedCount = selected.size
        val asserted = savedSameTrack
        val chosen = selected.mapNotNull { id -> saved.firstOrNull { it.id == id } }
        submitting = true
        coroutine.launch {
            try { val record = withContext(Dispatchers.IO) {
                if (chosen.size != expectedCount) throw InputFailure("input_missing")
                CompareWork.submitSaved(app, chosen, asserted).get()
            }; notice = "${record.state} · ${record.attemptId}" }
            catch (error: Exception) { notice = resultFailure(error) }
            finally { submitting = false }
        }
    }) { Text("Compare selected saved products") }
    ResultBrowser("compare", jobs, records, setOf("comparison" to "audio-forensic-comparison-v1", "product" to "audio-forensic-product-v1", "alfred-result" to "1"), onExport, onShare) { descriptor, value ->
        if (descriptor.kind == "comparison" && value is Map<*, *>) {
            if (value["comparison_schema_version"] != "audio-forensic-comparison-v1" || value["method_id"] != "python-c6ecce2-reference-tuple-v1") Text("unsupported_version · inspect/export original fields")
            else {
                Text("Comparison: ${value["status"] ?: "unknown"}")
                val winner = (value["ranking"] as? List<*>)?.mapNotNull { it as? Map<*, *> }?.firstOrNull { it["winner"] == true }?.get("source")
                Text("Winner reported by Rust (stable order breaks ties): ${winner ?: "none"}")
                Text("Inspect criteria, caveats and ordered ranking below for the exact tuple, missing fields, null ranks and compatibility result.")
            }
        }
    }
}
