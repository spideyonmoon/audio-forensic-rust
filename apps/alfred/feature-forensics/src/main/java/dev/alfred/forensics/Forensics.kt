package dev.alfred.forensics

import androidx.compose.runtime.Composable
import dev.alfred.shared.FeatureId
import dev.alfred.shared.FeatureScaffold
import dev.alfred.shared.Operation
import dev.alfred.shared.FeatureInputs

val forensicsOperation = Operation(FeatureId.FORENSICS, "Audio Forensics", 1, 32)

@Composable
fun ForensicsScreen(inputs: FeatureInputs) = FeatureScaffold(
    "Audio Forensics", "Measurements, complete metadata and qualified reference interpretations.", inputs.selection.items.size
)
