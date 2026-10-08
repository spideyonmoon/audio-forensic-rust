package dev.alfred.forensics

import androidx.compose.material3.Button
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.ui.platform.LocalContext
import dev.alfred.shared.*
import org.json.JSONObject
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

val forensicsOperation = Operation(FeatureId.FORENSICS, "Audio Forensics", 1, 32)

@Composable
fun ForensicsScreen(inputs: FeatureInputs?, jobs: SharedJobs, records: List<JobRecord>,
                    onExport: ResultAction, onShare: ResultAction, onRoute: (FeatureId) -> Unit) {
    val app = LocalContext.current.applicationContext
    val coroutine = rememberCoroutineScope()
    var notice by remember { mutableStateOf("") }
    var submitting by remember { mutableStateOf(false) }
    Text("Audio Forensics")
    Text("Read-only metadata, native measurements and the qualified Python reference method. Scores are uncalibrated; they are not probabilities or proof of authenticity, source or sound quality. Native ancestry remains INCONCLUSIVE; evidence index is unavailable.")
    if (notice.isNotEmpty()) Text(notice)
    if (inputs != null) {
        val prefix = inputs.selection.items.mapNotNull { inputs.probes[it.id]?.rate }.maxOrNull()?.let { minOf(180L, 8_640_000L / it) }
        fun start(scope: JSONObject, retry: String? = null) {
            submitting = true
            coroutine.launch {
                try { val record = withContext(Dispatchers.IO) { ForensicsWork.submit(app, inputs, scope, retry).get() }; notice = "${record.state} · ${record.attemptId}" }
                catch (error: Exception) { notice = resultFailure(error) }
                finally { submitting = false }
            }
        }
        Text("${inputs.selection.items.size} tracks · each result retains its own status, scope, channels, units and caveats.")
        Button(enabled = !submitting, onClick = { start(JSONObject().put("kind", "full")) }) { Text("Analyze full scope") }
        if (prefix != null && prefix > 0) Button(enabled = !submitting, onClick = { start(JSONObject().put("kind", "prefix").put("seconds", prefix)) }) { Text("Analyze first $prefix seconds") }
        Text("Full-scope resource admission may reject high-rate input. Choose a prefix explicitly; its result does not describe the entire track.")
        records.lastOrNull { it.feature == "forensics" && it.terminal && it.items == inputs.selection.items.map { item -> item.id } }?.let { previous ->
            Button(enabled = !submitting, onClick = { start(JSONObject(previous.options).getJSONObject("scope"), previous.jobId) }) { Text("Retry with fresh input") }
        }
        Button(onClick = { onRoute(FeatureId.SPECTROGRAM) }) { Text("Open Spectrogram workflow") }
        Button(onClick = { onRoute(FeatureId.COMPARE) }) { Text("Open Audio Compare workflow") }
    } else Text("Select documents in the workspace to start a new analysis. Saved results remain available.")
    ResultBrowser("forensics", jobs, records, setOf("product" to "audio-forensic-product-v1", "alfred-result" to "1"), onExport, onShare) { descriptor, value ->
        if (descriptor.kind == "product" && value is Map<*, *>) {
            val doc = runCatching { ForensicDocument.from(value) }.getOrNull()
            if (doc == null) Text("invalid_payload · inspect raw fields or export original bytes")
            else {
                Text("${doc.status} · ${doc.source} · ${doc.measurementStatus} · reference ${doc.assessmentStatus}")
                Text(doc.summary)
            }
            Text("Expand metadata for all raw tags/declarations; measurement_report for native channels, scopes and detector caveats; reference_assessment for scores, missing inputs, candidates, rule traces and deviations. Every saved field is accessible below.")
        }
    }
}
