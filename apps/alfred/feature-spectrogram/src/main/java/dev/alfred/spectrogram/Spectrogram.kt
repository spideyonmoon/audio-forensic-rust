package dev.alfred.spectrogram

import androidx.compose.runtime.Composable
import dev.alfred.shared.FeatureId
import dev.alfred.shared.FeatureScaffold
import dev.alfred.shared.Operation
import dev.alfred.shared.FeatureInputs

val spectrogramOperation = Operation(FeatureId.SPECTROGRAM, "Spectrogram", 1, 1)

@Composable
fun SpectrogramScreen(inputs: FeatureInputs) = FeatureScaffold(
    "Spectrogram", "Independent spectral viewing and PNG export using the existing core renderer.", inputs.selection.items.size
)
