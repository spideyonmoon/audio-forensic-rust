package dev.alfred.forensics

import dev.alfred.shared.ExactJson

/** Presentation never derives a score. Unknown versions are retained for byte export. */
data class ForensicDocument(val status: String, val source: String, val measurementStatus: String,
                            val assessmentStatus: String, val summary: String, val value: Map<*, *>?) {
    companion object {
        fun read(bytes: ByteArray): ForensicDocument {
            val product = ExactJson.parse(bytes, 64 * 1024 * 1024) as? Map<*, *> ?: error("invalid_payload")
            return from(product)
        }
        fun from(product: Map<*, *>): ForensicDocument {
            if (product["product_schema_version"] != "audio-forensic-product-v1") return ForensicDocument("unsupported_version", "Unknown product", "unknown", "unknown", "Original bytes remain exportable.", null)
            val measurement = product["measurement_report"] as? Map<*, *> ?: error("invalid_payload")
            val assessment = product["reference_assessment"] as? Map<*, *> ?: error("invalid_payload")
            val knownAssessment = assessment["method_id"] == "python-reference-c6ecce2-v1" && assessment["assessment_version"]?.toString() == "1"
            return ForensicDocument("available", measurement["source"]?.toString() ?: "Unknown source",
                measurement["status"]?.toString() ?: "unknown", assessment["status"]?.toString() ?: "unknown",
                if (knownAssessment) assessment["display_summary"] as? String ?: "Reference summary unavailable"
                else "unsupported_version: reference interpretation; inspect raw fields and export original bytes.", product)
        }
    }
}
