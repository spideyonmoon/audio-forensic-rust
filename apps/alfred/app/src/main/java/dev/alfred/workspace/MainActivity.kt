package dev.alfred.workspace

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
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
                // Session-only selection. A04 supplies durable identity/grants and bounded acquisition.
                var documents by remember { mutableStateOf<List<String>>(emptyList()) }
                var folderSelected by remember { mutableStateOf(false) }
                var notice by remember { mutableStateOf("") }
                var route by remember { mutableStateOf<String?>(null) }
                var nativeStatus by remember { mutableStateOf("Loading native host…") }
                LaunchedEffect(Unit) {
                    nativeStatus = withContext(Dispatchers.IO) { NativeBootstrap.load() }
                }
                val files = rememberLauncherForActivityResult(ActivityResultContracts.OpenMultipleDocuments()) { uris ->
                    if (uris.isNotEmpty()) {
                        route = null
                        folderSelected = false
                        documents = emptyList()
                        if (uris.size > 32) {
                            notice = "Selection limit: choose at most 32 documents. No partial selection retained."
                        } else {
                            documents = uris.map { it.toString() }.distinct()
                            notice = "Contents need verification before any operation can run."
                        }
                    }
                }
                val folder = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocumentTree()) { uri ->
                    if (uri != null) {
                        route = null
                        documents = emptyList()
                        folderSelected = true
                        notice = "Folder selected. Bounded folder listing is not available in this scaffold. Choose documents to explore feature routes."
                    }
                }
                BackHandler(route != null) { route = null }
                Scaffold { insets ->
                    Column(
                        Modifier.fillMaxSize().padding(insets).verticalScroll(rememberScrollState()).padding(20.dp),
                        verticalArrangement = Arrangement.spacedBy(12.dp)
                    ) {
                        Text("Alfred", style = MaterialTheme.typography.headlineLarge)
                        if (route != null) TextButton(onClick = { route = null }) { Text("Back to workspace") }
                        when (route) {
                            FeatureId.FORENSICS.name -> ForensicsScreen(documents.size)
                            FeatureId.SPECTROGRAM.name -> SpectrogramScreen(documents.size)
                            FeatureId.COMPARE.name -> CompareScreen(documents.size)
                            else -> {
                                Text("Audio workspace", style = MaterialTheme.typography.titleLarge)
                                Text("Select audio documents or a folder. Everything stays on this device.")
                                Button(onClick = { files.launch(arrayOf("*/*")) }) { Text("Choose documents") }
                                Button(onClick = { folder.launch(null) }) { Text("Choose folder") }
                                Text(if (folderSelected) "Folder selected" else "${documents.size} selected documents")
                                if (notice.isNotEmpty()) Text(notice)
                                listOf(forensicsOperation, spectrogramOperation, compareOperation).forEach { operation ->
                                    Button(
                                        enabled = operation.acceptsSelectionShape(documents.size),
                                        onClick = { route = operation.id.name }
                                    ) { Text(operation.title) }
                                    if (!operation.acceptsSelectionShape(documents.size)) {
                                        Text("Select ${operation.minimum}–${operation.maximum} documents to open this feature.")
                                    }
                                }
                                Text("Development scaffold · $nativeStatus")
                            }
                        }
                    }
                }
            }
        }
    }
}
