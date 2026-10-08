package dev.alfred.spectrogram

import android.graphics.Bitmap
import android.graphics.BitmapFactory
import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.asImageBitmap
import dev.alfred.shared.*
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/** Display a scaled engine PNG; never redraw calibration or export this preview. */
@Composable
fun SpectrogramPreview(store: ResultStore, attempt: String, descriptor: ResultDescriptor) {
    var bitmap by remember(attempt, descriptor) { mutableStateOf<Bitmap?>(null) }
    var status by remember(attempt, descriptor) { mutableStateOf("Loading PNG preview…") }
    LaunchedEffect(attempt, descriptor) {
        try {
            bitmap = withContext(Dispatchers.IO) {
                store.pin(attempt).use {
                    val file = store.validate(attempt, descriptor)
                    val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
                    file.inputStream().use { stream -> BitmapFactory.decodeStream(stream, null, bounds) }
                    if (bounds.outWidth !in 1..3840 || bounds.outHeight !in 1..2160) throw InputFailure("invalid_payload")
                    val options = BitmapFactory.Options().apply {
                        inSampleSize = 1
                        while (bounds.outWidth / inSampleSize > 1024) inSampleSize *= 2
                    }
                    file.inputStream().use { stream -> BitmapFactory.decodeStream(stream, null, options) } ?: throw InputFailure("invalid_payload")
                }
            }
            status = "Scaled viewing preview. Export/share retains the chosen Rust PNG preset, title, axes, calibration and PCM binding."
        } catch (error: Exception) { status = resultFailure(error) }
    }
    Text(status)
    bitmap?.let { Image(it.asImageBitmap(), contentDescription = "Calibrated Rust spectrogram PNG", modifier = Modifier.fillMaxWidth()) }
}
