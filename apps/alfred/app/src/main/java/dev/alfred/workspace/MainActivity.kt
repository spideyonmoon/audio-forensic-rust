package dev.alfred.workspace

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.Checkbox
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import dev.alfred.compare.CompareScreen
import dev.alfred.compare.compareOperation
import dev.alfred.forensics.ForensicsScreen
import dev.alfred.forensics.forensicsOperation
import dev.alfred.shared.FeatureId
import dev.alfred.shared.NativeBootstrap
import dev.alfred.shared.SafPicker
import dev.alfred.shared.WorkspaceInput
import dev.alfred.shared.WorkspaceState
import dev.alfred.shared.operationCapability
import dev.alfred.spectrogram.SpectrogramScreen
import dev.alfred.spectrogram.spectrogramOperation
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.coroutines.delay
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.ViewModelProvider
import dev.alfred.shared.SharedJobs
import dev.alfred.shared.JobRecord
import androidx.activity.result.contract.ActivityResultContracts
import android.Manifest
import android.os.Build
import android.content.pm.PackageManager
import dev.alfred.shared.ResultTransfer
import dev.alfred.shared.ResultDescriptor
import dev.alfred.shared.ResultDestination
import dev.alfred.shared.ResultAction
import dev.alfred.shared.resultFailure
import kotlinx.coroutines.launch

class WorkspaceModel(application: android.app.Application) : AndroidViewModel(application) {
    val workspace = mutableStateOf(WorkspaceState())
    val jobs = SharedJobs.get(application)
    val transfers = ResultTransfer(application, jobs.results)
    val inputs = WorkspaceInput(application) { workspace.value = it }
    override fun onCleared() { inputs.close() }
}

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        val model = ViewModelProvider(this)[WorkspaceModel::class.java]
        setContent {
            MaterialTheme {
                val workspace by model.workspace
                var checked by remember { mutableStateOf<Set<String>>(emptySet()) }
                var route by rememberSaveable { mutableStateOf<String?>(null) }
                var transferNotice by remember { mutableStateOf("") }
                var exportAttempt by rememberSaveable { mutableStateOf<String?>(null) }
                var exportDescriptor by rememberSaveable { mutableStateOf<String?>(null) }
                val coroutine = rememberCoroutineScope()
                val destination = rememberLauncherForActivityResult(ResultDestination()) { uri ->
                    val attempt = exportAttempt
                    val descriptor = exportDescriptor
                    exportAttempt = null; exportDescriptor = null
                    if (uri != null && attempt != null && descriptor != null) coroutine.launch {
                        try { withContext(Dispatchers.IO) { model.transfers.export(attempt, ResultDescriptor.from(org.json.JSONObject(descriptor)), uri).get() }; transferNotice = "Export completed" }
                        catch (error: Exception) { transferNotice = "Export failed: ${resultFailure(error)} · local result retained" }
                    }
                }
                val export: ResultAction = { attempt, descriptor ->
                    exportAttempt = attempt; exportDescriptor = descriptor.json().toString()
                    destination.launch("alfred-${descriptor.kind}-${descriptor.path.substringBefore('.')}.${if (descriptor.kind == "png") "png" else "json"}")
                }
                val share: ResultAction = { attempt, descriptor -> coroutine.launch {
                    try { val intent = withContext(Dispatchers.IO) { model.transfers.share(attempt, descriptor).get() }; startActivity(android.content.Intent.createChooser(intent, "Share Alfred result")); transferNotice = "Share copy prepared" }
                    catch (error: Exception) { transferNotice = "Share failed: ${resultFailure(error)} · local result retained" }
                } }
                var nativeStatus by remember { mutableStateOf("Loading native host…") }
                val inputs = model.inputs
                var jobs by remember { mutableStateOf<List<JobRecord>>(emptyList()) }
                var progress by remember { mutableStateOf<dev.alfred.shared.JobProgress?>(null) }
                var notificationAllowed by remember { mutableStateOf(Build.VERSION.SDK_INT < 33 || checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) == PackageManager.PERMISSION_GRANTED) }
                val notifications = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { notificationAllowed = it }
                LaunchedEffect(model.jobs) {
                    while (true) {
                        if (lifecycle.currentState.isAtLeast(Lifecycle.State.STARTED)) {
                            jobs = model.jobs.snapshot(); progress = model.jobs.progressSnapshot()
                        }
                        delay(250)
                    }
                }
                LaunchedEffect(Unit) { nativeStatus = withContext(Dispatchers.IO) { NativeBootstrap.load() } }
                val files = rememberLauncherForActivityResult(SafPicker()) { result ->
                    if (result != null) { route = null; checked = emptySet(); inputs.select(result) }
                }
                val single = rememberLauncherForActivityResult(SafPicker(multiple = false)) { result ->
                    if (result != null) { route = null; checked = emptySet(); inputs.select(result) }
                }
                val folder = rememberLauncherForActivityResult(SafPicker(tree = true)) { result ->
                    if (result != null) { route = null; checked = emptySet(); inputs.select(result) }
                }
                BackHandler(route != null) { route = null }
                Scaffold { insets ->
                    Column(Modifier.fillMaxSize().padding(insets).verticalScroll(rememberScrollState()).padding(20.dp),
                        verticalArrangement = Arrangement.spacedBy(12.dp)) {
                        Text("Alfred", style = MaterialTheme.typography.headlineLarge)
                        if (transferNotice.isNotEmpty()) Text(transferNotice)
                        if (model.jobs.releaseUnconfirmed()) Text("Native release could not be confirmed. Work is blocked to protect its files; force-stop Alfred before reopening it.")
                        if (!notificationAllowed) {
                            Text("Notifications are denied. Started operations can continue; return here to cancel and inspect progress.")
                            Button(onClick = { if (Build.VERSION.SDK_INT >= 33) notifications.launch(Manifest.permission.POST_NOTIFICATIONS) }) { Text("Allow job notifications") }
                        }
                        jobs.filter { !it.terminal }.forEach { job ->
                            Text("${job.feature} · ${job.state}${if (job.cancelRequested) " · cancellation requested" else ""}")
                            if (progress?.attemptId == job.attemptId) Text("${progress?.phase} · pass ${progress?.pass ?: "unknown"} · ${progress?.frames ?: "unknown"} frames / ${progress?.expectedFrames ?: "unknown"}")
                            Button(onClick = { model.jobs.cancel(job.attemptId) }) { Text("Cancel operation") }
                        }
                        if (route != null) TextButton(onClick = { route = null }) { Text("Back to workspace") }
                        val count = workspace.selection?.items?.size ?: 0
                        when (route) {
                            FeatureId.FORENSICS.name -> ForensicsScreen(inputs.featureInputs(), model.jobs, jobs, export, share) { feature ->
                                val operation = when (feature) { FeatureId.FORENSICS -> forensicsOperation; FeatureId.SPECTROGRAM -> spectrogramOperation; FeatureId.COMPARE -> compareOperation }
                                val capability = operationCapability(operation, workspace)
                                if (capability.state == "available") route = feature.name else transferNotice = capability.reason
                            }
                            FeatureId.SPECTROGRAM.name -> SpectrogramScreen(inputs.featureInputs(), model.jobs, jobs, export, share)
                            FeatureId.COMPARE.name -> CompareScreen(inputs.featureInputs(), workspace.sameTrack, model.jobs, jobs, export, share)
                            else -> {
                                Text("Audio workspace", style = MaterialTheme.typography.titleLarge)
                                TextButton(onClick = { route = FeatureId.FORENSICS.name }) { Text("Forensics history") }
                                Text("Select audio documents or a folder. Everything stays on this device.")
                                Button(onClick = { single.launch(Unit) }) { Text("Choose one document") }
                                Button(onClick = { files.launch(Unit) }) { Text("Choose documents") }
                                Button(onClick = { folder.launch(Unit) }) { Text("Choose folder") }
                                Text("$count selected documents")
                                if (workspace.notice.isNotEmpty()) Text(workspace.notice)
                                if (workspace.busy) Button(onClick = { inputs.cancel() }) { Text("Cancel input checks") }
                                workspace.selection?.items?.forEach { item ->
                                    Row {
                                        if (workspace.selection?.incomplete == true) Checkbox(item.id in checked, onCheckedChange = { value ->
                                            checked = if (value) checked + item.id else checked - item.id
                                        })
                                        val probe = workspace.probes[item.id]
                                        Text("${item.name} · ${item.declaredBytes?.let { "$it bytes" } ?: "unknown length"} · ${item.grantState}\n${probe?.reason ?: "needs_input"}")
                                    }
                                }
                                if (workspace.selection?.incomplete == true) Button(
                                    enabled = checked.size in 1..31, onClick = { inputs.confirmFolder(checked) }
                                ) { Text("Use selected documents") }
                                if (count >= 2) Row {
                                    Checkbox(workspace.sameTrack, onCheckedChange = { inputs.assertSameTrack(it) })
                                    Text("These are variants of the same track")
                                }
                                listOf(forensicsOperation, spectrogramOperation, compareOperation).forEach { operation ->
                                    val capability = operationCapability(operation, workspace)
                                    Button(enabled = capability.state == "available" && !workspace.busy,
                                        onClick = { route = operation.id.name }) { Text(operation.title) }
                                    Text("${capability.state}: ${capability.reason}")
                                    if (capability.state == "available" && operation.id == FeatureId.COMPARE) {
                                        Text("A live comparison will use one agreed scope for all tracks; common prefix budget: ${capability.prefixSeconds ?: 0} seconds. Choose the scope when starting the Compare workflow.")
                                    }
                                }
                                TextButton(onClick = { route = FeatureId.SPECTROGRAM.name }) { Text("Spectrogram history") }
                                TextButton(onClick = { route = FeatureId.COMPARE.name }) { Text("Compare history") }
                                Text("Input workspace · $nativeStatus")
                            }
                        }
                    }
                }
            }
        }
    }
}
