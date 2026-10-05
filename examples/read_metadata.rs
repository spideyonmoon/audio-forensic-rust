//! No-decode metadata, or an explicitly requested native-loudness audit.
use audio_forensic::{
    AnalysisOptions, CancellationToken, analyze_path,
    metadata::{audit_metadata_replaygain, read_metadata_path},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("usage: read_metadata PATH [--audit [MAX_SECONDS]]")?;
    let audit = args.next();
    if audit.as_deref().is_some_and(|arg| arg != "--audit") {
        return Err("expected --audit".into());
    }
    let options = AnalysisOptions {
        max_seconds: args.next().map(|v| v.parse()).transpose()?,
        ..Default::default()
    };
    if args.next().is_some() {
        return Err("too many arguments".into());
    }
    let cancel = CancellationToken::default();
    let metadata = read_metadata_path(&path, &options, &cancel);
    if audit.is_some() {
        let measured = analyze_path(&path, &options, &cancel);
        let audit = audit_metadata_replaygain(&metadata, &measured);
        println!(
            "{}",
            serde_json::to_string_pretty(
                &serde_json::json!({"metadata": metadata, "replaygain_audit": audit})
            )?
        );
    } else {
        println!("{}", serde_json::to_string_pretty(&metadata)?);
    }
    Ok(())
}
