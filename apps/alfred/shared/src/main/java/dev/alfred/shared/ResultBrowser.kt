package dev.alfred.shared

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.Button
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.rememberSaveable
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

typealias ResultAction = (String, ResultDescriptor) -> Unit

/** One document in memory; navigation pages every field and long text. */
@Composable
fun ExactFields(value: Any?) {
    var path by remember(value) { mutableStateOf<List<String>>(emptyList()) }
    var page by remember(value, path) { mutableStateOf(0) }
    var current = value
    for (part in path) current = when (val node = current) {
        is Map<*, *> -> node[part]
        is List<*> -> node[part.toInt()]
        else -> null
    }
    Text(if (path.isEmpty()) "All report fields" else path.joinToString(" / "))
    if (path.isNotEmpty()) TextButton(onClick = { path = path.dropLast(1) }) { Text("Up one field") }
    val entries = when (val node = current) {
        is Map<*, *> -> node.entries.asSequence().drop(page * 24).take(24).map { it.key.toString() to it.value }.toList()
        is List<*> -> (page * 24 until minOf(node.size, (page + 1) * 24)).map { it.toString() to node[it] }
        else -> emptyList()
    }
    if (current is Map<*, *> || current is List<*>) {
        val size = when (val node = current) { is Map<*, *> -> node.size; is List<*> -> node.size; else -> 0 }
        Text("$size fields · page ${page + 1}")
        entries.forEach { (key, child) ->
            TextButton(onClick = { path = path + key }) {
                val preview = when (child) { null -> "null · unavailable"; is Map<*, *> -> "${child.size} fields"; is List<*> -> "${child.size} entries"; else -> child.toString().take(100) }
                Text("$key: $preview")
            }
        }
        if ((page + 1) * 24 < size) Button(onClick = { page++ }) { Text("Next fields") }
    } else {
        val text = current?.toString() ?: "null · unavailable (not zero)"
        SelectionContainer { Text(text.drop(page * 2000).take(2000)) }
        if ((page + 1) * 2000 < text.length) Button(onClick = { page++ }) { Text("Next text") }
    }
    if (page > 0) TextButton(onClick = { page-- }) { Text("Previous page") }
}

@Composable
fun ResultBrowser(feature: String, jobs: SharedJobs, records: List<JobRecord>, supported: Set<Pair<String, String>>,
                  onExport: ResultAction, onShare: ResultAction,
                  artifactPreview: @Composable (String, ResultDescriptor) -> Unit = { _, _ -> },
                  summary: @Composable (ResultDescriptor, Any?) -> Unit = { _, _ -> }) {
    var entries by remember { mutableStateOf<List<org.json.JSONObject>>(emptyList()) }
    var selected by rememberSaveable(feature) { mutableStateOf<String?>(null) }
    var descriptorIndex by rememberSaveable(selected) { mutableStateOf(0) }
    var revision by remember { mutableStateOf(0) }
    var notice by remember { mutableStateOf("") }
    var descriptors by remember { mutableStateOf<List<ResultDescriptor>>(emptyList()) }
    var document by remember { mutableStateOf<Any?>(null) }
    var loadState by remember { mutableStateOf("loading") }
    val scope = rememberCoroutineScope()
    LaunchedEffect(feature, records, revision) {
        try {
            entries = withContext(Dispatchers.IO) {
                jobs.awaitReady(); jobs.results.entries().filter { it.getString("feature_id") == feature }.reversed()
            }
        } catch (error: Exception) { notice = resultFailure(error) }
    }
    Text("Saved $feature results")
    if (notice.isNotEmpty()) Text(notice)
    records.filter { it.feature == feature && it.terminal }.reversed().forEach { Text("${it.state} · ${it.error ?: ""} · attempt ${it.attemptId}") }
    entries.forEach { entry ->
        val attempt = entry.getString("attempt_id")
        TextButton(onClick = { selected = attempt }) { Text("Open ${entry.getString("state")} · $attempt") }
    }
    LaunchedEffect(selected, descriptorIndex, revision) {
        document = null; descriptors = emptyList(); loadState = "loading"
        val attempt = selected ?: return@LaunchedEffect
        try {
            val loaded = withContext(Dispatchers.IO) {
                jobs.results.pin(attempt).use {
                    val manifest = jobs.results.load(attempt)
                    val payloads = manifest.getJSONArray("payloads")
                    val list = (0 until payloads.length()).map { ResultDescriptor.from(payloads.getJSONObject(it)) }
                    val descriptor = list.getOrNull(descriptorIndex)
                    var state = descriptor?.let { jobs.results.dispatch(it, supported) } ?: "no_payload"
                    val value = if (descriptor != null && state == "available" && !descriptor.artifact) {
                        try {
                            val runtime = Runtime.getRuntime()
                            val heapFree = runtime.maxMemory() - runtime.totalMemory() + runtime.freeMemory()
                            if (heapFree < descriptor.bytes * 8 + 16L * 1024 * 1024) throw InputFailure("resource_limit")
                            ExactJson.parse(jobs.results.validate(attempt, descriptor).readBytes(), 64 * 1024 * 1024)
                        } catch (error: Exception) { state = resultFailure(error); null }
                    } else null
                    Triple(list, state, value) to "${manifest.getJSONArray("outcomes")} · saved scope/tracks: ${manifest.optJSONObject("options") ?: "unavailable in older history"}"
                }
            }
            descriptors = loaded.first.first; loadState = loaded.first.second; document = loaded.first.third
            notice = "Track outcomes: ${loaded.second}"
        } catch (error: Exception) { loadState = resultFailure(error) }
    }
    selected?.let { attempt ->
        Text("Attempt $attempt · $loadState")
        descriptors.forEachIndexed { index, descriptor ->
            TextButton(onClick = { descriptorIndex = index }) { Text("${index + 1}: ${descriptor.kind} · ${descriptor.version} · ${descriptor.bytes} bytes") }
        }
        descriptors.getOrNull(descriptorIndex)?.let { descriptor ->
            Button(onClick = { onExport(attempt, descriptor) }) { Text("Export original ${descriptor.kind}") }
            Button(onClick = { onShare(attempt, descriptor) }) { Text("Share ${descriptor.kind}") }
            if (loadState == "available" && descriptor.artifact) artifactPreview(attempt, descriptor)
            if (loadState == "available" && document != null) {
                summary(descriptor, document)
                key(attempt, descriptor.path) { Column { ExactFields(document) } }
            } else if (loadState == "unsupported_version") Text("unsupported_version · original bytes remain exportable")
        }
        Button(onClick = {
            scope.launch {
                try { withContext(Dispatchers.IO) { jobs.results.delete(attempt) }; selected = null; revision++; notice = "Deleted local result" }
                catch (error: Exception) { notice = resultFailure(error) }
            }
        }) { Text("Delete local result") }
    }
}
