package dev.alfred.forensics

import androidx.compose.runtime.Composable
import dev.alfred.shared.FeatureId
import dev.alfred.shared.FeatureScaffold
import dev.alfred.shared.Operation

val forensicsOperation = Operation(FeatureId.FORENSICS, "Audio Forensics", 1, 32)

@Composable
fun ForensicsScreen(count: Int) = FeatureScaffold(
    "Audio Forensics", "Measurements, complete metadata and qualified reference interpretations.", count
)
