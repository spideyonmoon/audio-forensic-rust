package dev.alfred.compare

import androidx.compose.runtime.Composable
import dev.alfred.shared.FeatureId
import dev.alfred.shared.FeatureScaffold
import dev.alfred.shared.Operation
import dev.alfred.shared.FeatureInputs

val compareOperation = Operation(FeatureId.COMPARE, "Audio Compare", 2, 32)

@Composable
fun CompareScreen(inputs: FeatureInputs) = FeatureScaffold(
    "Audio Compare", "Compare same-track variants after a shared scope and compatibility check.", inputs.selection.items.size
)
