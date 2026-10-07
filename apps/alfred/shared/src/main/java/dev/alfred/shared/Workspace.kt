package dev.alfred.shared

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

enum class FeatureId { FORENSICS, SPECTROGRAM, COMPARE }

data class Operation(val id: FeatureId, val title: String, val minimum: Int, val maximum: Int) {
    fun acceptsSelectionShape(count: Int): Boolean = count in minimum..maximum
}

/** A02 navigation only: no codec claim, persisted URI grant, source lease or job. */
@Composable
fun FeatureScaffold(title: String, detail: String, selectionCount: Int) {
    Column(Modifier.padding(20.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Text(title)
        Text("$selectionCount selected documents · contents unverified")
        Text(detail)
        Text("Analysis is not available in this scaffold. Selection acquisition, native jobs and results arrive in the following integration packets.")
    }
}

/** Only the bootstrap exists in A02. A03 adds the versioned worker transport. */
object NativeBootstrap {
    private external fun nativeHostVersion(): Int
    fun load(): String = try {
        System.loadLibrary("alfred_native")
        if (nativeHostVersion() == 1) "Native host v1 loaded" else "unsupported_version"
    } catch (_: LinkageError) {
        "native_load_failed"
    } catch (_: SecurityException) {
        "native_load_failed"
    }
}
