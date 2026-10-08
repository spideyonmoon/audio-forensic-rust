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
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
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

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            MaterialTheme {
                var workspace by remember { mutableStateOf(WorkspaceState()) }
                var checked by remember { mutableStateOf<Set<String>>(emptySet()) }
                var route by remember { mutableStateOf<String?>(null) }
                var nativeStatus by remember { mutableStateOf("Loading native host…") }
                val inputs = remember { WorkspaceInput(applicationContext) { workspace = it } }
                DisposableEffect(inputs) { onDispose { inputs.close() } }
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
                        if (route != null) TextButton(onClick = { route = null }) { Text("Back to workspace") }
                        val count = workspace.selection?.items?.size ?: 0
                        when (route) {
                            FeatureId.FORENSICS.name -> inputs.featureInputs()?.let { ForensicsScreen(it) }
                            FeatureId.SPECTROGRAM.name -> inputs.featureInputs()?.let { SpectrogramScreen(it) }
                            FeatureId.COMPARE.name -> inputs.featureInputs()?.let { CompareScreen(it) }
                            else -> {
                                Text("Audio workspace", style = MaterialTheme.typography.titleLarge)
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
                                Text("Input workspace · $nativeStatus")
                            }
                        }
                    }
                }
            }
        }
    }
}
