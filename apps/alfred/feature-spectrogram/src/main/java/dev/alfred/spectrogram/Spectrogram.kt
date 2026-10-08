package dev.alfred.spectrogram

import androidx.compose.material3.Button
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.platform.LocalContext
import dev.alfred.shared.*
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONObject

val spectrogramOperation = Operation(FeatureId.SPECTROGRAM, "Spectrogram", 1, 1)

@Composable
fun SpectrogramScreen(inputs: FeatureInputs?, jobs: SharedJobs, records: List<JobRecord>, onExport: ResultAction, onShare: ResultAction) {
    val app = LocalContext.current.applicationContext
    val coroutine = rememberCoroutineScope()
    var preset by rememberSaveable { mutableStateOf("publication") }
    var notice by remember { mutableStateOf("") }
    var submitting by remember { mutableStateOf(false) }
    Text("Spectrogram")
    Text("Independent bounded P06 viewing and Rust PNG export. Native channels and stereo mid, hash, intervals and presentation warnings remain in the saved wrapper and canvas.")
    Text(notice)
    if (inputs != null && inputs.selection.items.size == 1) {
        Text("Title: ${inputs.selection.items.single().name}")
        listOf("standard" to "1600 × 900", "publication" to "2560 × 1440", "large" to "3840 × 2160").forEach { (value, label) ->
            Button(onClick = { preset = value }) { Text("$label · $value${if (preset == value) " · selected" else ""}") }
        }
        fun start(scope: JSONObject) {
            submitting = true
            val selectedPreset = preset
            coroutine.launch {
                try { val record = withContext(Dispatchers.IO) { SpectrogramWork.submit(app, inputs, scope, selectedPreset).get() }; notice = "${record.state} · ${record.attemptId}" }
                catch (error: Exception) { notice = resultFailure(error) }
                finally { submitting = false }
            }
        }
        Button(enabled = !submitting, onClick = { start(JSONObject().put("kind", "full")) }) { Text("Create full-scope spectrogram") }
        Button(enabled = !submitting, onClick = { start(JSONObject().put("kind", "prefix").put("seconds", 180)) }) { Text("Create first 180 seconds") }
        Text("A render failure keeps its successful measurements/wrapper and records a separate PNG diagnostic. The chosen export size is never lowered automatically.")
    } else Text("Select one supported track in the workspace to create a spectrogram. Saved results remain available.")
    ResultBrowser("spectrogram", jobs, records, setOf("spectrogram" to "1", "alfred-result" to "1", "png" to "1"), onExport, onShare,
        artifactPreview = { attempt, descriptor -> if (descriptor.kind == "png") SpectrogramPreview(jobs.results, attempt, descriptor) }) { descriptor, value ->
        if (descriptor.kind == "spectrogram" && value is Map<*, *>) {
            Text("Measurement: ${(value["measurement"] as? Map<*, *>)?.get("status") ?: "unknown"}")
            if (value["presentation"] == null) Text("Presentation/encoded bitrate unavailable in this older wrapper; no bitrate is inferred.")
            Text("Inspect spectrogram coverage/hash, channel and mid warnings, axes, optional presentation and separate artifact diagnostics below. Select PNG to view the Rust canvas.")
        }
    }
}
