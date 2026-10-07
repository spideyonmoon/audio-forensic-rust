//! P05 offline saved-result interpreter; P07 owns product CLI workflows.
use audio_forensic::reference_assessment::assess_reference;
use audio_forensic::reference_inputs::ReferenceAnalysis;
fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: read_reference_assessment SAVED_REFERENCE_JSON");
    let input: ReferenceAnalysis =
        serde_json::from_str(&std::fs::read_to_string(path).expect("read JSON"))
            .expect("reference JSON");
    println!(
        "{}",
        serde_json::to_string_pretty(&assess_reference(&input)).expect("serialize")
    );
}
