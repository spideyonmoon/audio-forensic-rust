//! Offline freeze/run utility. See EVALUATION_VALIDATION.md for the label contract.
use audio_forensic::{
    AnalysisReport,
    evaluation::{EvaluationManifest, EvaluationSession, FrozenEvaluation, Split},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::{
    env,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

type Error = Box<dyn std::error::Error>;
const MAX_DOCUMENT_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RunnerPlan {
    runner_sha256: String,
    evaluation: FrozenEvaluation,
}

fn runner_hash() -> Result<String, Error> {
    let mut file = File::open(env::current_exe()?)?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn read<T: DeserializeOwned>(path: &Path) -> Result<T, Error> {
    let mut bytes = vec![];
    File::open(path)?
        .take(MAX_DOCUMENT_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_DOCUMENT_BYTES {
        return Err("JSON input exceeds 64 MiB".into());
    }
    Ok(serde_json::from_slice(&bytes)?)
}

fn save_new(path: &Path, value: &impl Serialize) -> Result<(), Error> {
    let bytes = serde_json::to_vec_pretty(value)?;
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(&bytes)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(())
}

fn run(args: &[std::ffi::OsString]) -> Result<(), Error> {
    match args {
        [mode, manifest, frozen] if mode == "freeze" => {
            let manifest: EvaluationManifest = read(Path::new(manifest))?;
            let plan = RunnerPlan { runner_sha256: runner_hash()?, evaluation: FrozenEvaluation::freeze(manifest)? };
            save_new(Path::new(frozen), &plan)?;
        }
        [mode, frozen, split, directory, output] if mode == "run" => {
            let split = match split.to_str() {
                Some("development") => Split::Development,
                Some("validation") => Split::Validation,
                Some("locked_test") => Split::LockedTest,
                Some("challenge") => Split::Challenge,
                _ => return Err("split must be development, validation, locked_test or challenge".into()),
            };
            let runner_plan: RunnerPlan = read(Path::new(frozen))?;
            if runner_plan.runner_sha256 != runner_hash()? { return Err("runner executable differs from frozen fingerprint".into()); }
            let plan = runner_plan.evaluation;
            let mut session = EvaluationSession::new(&plan, split)?;
            let root = fs::canonicalize(directory)?;
            for case in plan.manifest.cases.iter().filter(|c| c.split == split) {
                // IDs are validated before paths are built. Check containment
                // after resolving links, and never inspect unselected reports.
                let path = fs::canonicalize(root.join(format!("{}.json", case.id)))?;
                if !path.starts_with(&root) { return Err("report path escapes report directory".into()); }
                let report: AnalysisReport = read(&path)?;
                session.submit(&case.id, &report)?;
            }
            save_new(&PathBuf::from(output), &session.finish()?)?;
        }
        _ => return Err("usage: evaluate_reports freeze <manifest.json> <new-plan.json> | run <plan.json> <split> <report-directory> <new-summary.json>".into()),
    }
    Ok(())
}

fn main() -> ExitCode {
    match run(&env::args_os().skip(1).collect::<Vec<_>>()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use audio_forensic::{
        AnalysisOptions, CancellationToken, analyze_path,
        evaluation::{Codec, EvaluationCase, Provenance, StageLabel},
    };

    #[test]
    fn freeze_and_run_reads_only_selected_reports_and_preserves_outputs() {
        let temp = tempfile::tempdir().unwrap();
        let report = analyze_path(
            Path::new("tests/fixtures/noise16.wav"),
            &AnalysisOptions::default(),
            &CancellationToken::default(),
        );
        let hash = report.coverage.as_ref().unwrap().decoded_pcm_sha256.clone();
        let case = EvaluationCase {
            id: "dev".into(),
            source_group: "source".into(),
            split: Split::Development,
            codec: Codec::Aac,
            label: StageLabel::Absent,
            provenance: Provenance::GeneratedControl,
            processing: "native".into(),
            evidence_ref: "generated fixture receipt".into(),
            evidence_sha256: "a".repeat(64),
            expected_pcm_sha256: Some(hash),
        };
        let mut locked = case.clone();
        locked.id = "unseen".into();
        locked.source_group = "reserved".into();
        locked.split = Split::LockedTest;
        locked.expected_pcm_sha256 = Some("b".repeat(64));
        let manifest = EvaluationManifest {
            manifest_version: 1,
            report_engine_version: report.engine_version.clone(),
            cases: vec![case, locked],
        };
        let input = temp.path().join("manifest.json");
        let plan = temp.path().join("plan.json");
        let output = temp.path().join("summary.json");
        save_new(&input, &manifest).unwrap();
        save_new(&temp.path().join("dev.json"), &report).unwrap();
        // No locked report exists, even while freezing the complete manifest.
        let freeze_args = vec![
            "freeze".into(),
            input.into_os_string(),
            plan.clone().into_os_string(),
        ];
        run(&freeze_args).unwrap();
        let plan_before = fs::read(&plan).unwrap();
        assert!(run(&freeze_args).is_err());
        assert_eq!(fs::read(&plan).unwrap(), plan_before);
        let run_args = vec![
            "run".into(),
            plan.into_os_string(),
            "development".into(),
            temp.path().as_os_str().into(),
            output.clone().into_os_string(),
        ];
        run(&run_args).unwrap();
        let before = fs::read(&output).unwrap();
        let summary: serde_json::Value = serde_json::from_slice(&before).unwrap();
        assert_eq!(summary["cases"].as_array().unwrap().len(), 1);
        assert_eq!(summary["cases"][0]["case_id"], "dev");
        assert!(run(&run_args).is_err());
        assert_eq!(fs::read(output).unwrap(), before);
        let mut changed: RunnerPlan = read(Path::new(&run_args[1])).unwrap();
        changed.runner_sha256 = "0".repeat(64);
        let changed_path = temp.path().join("changed-plan.json");
        save_new(&changed_path, &changed).unwrap();
        let mut changed_args = run_args;
        changed_args[1] = changed_path.into_os_string();
        changed_args[4] = temp.path().join("must-not-exist.json").into_os_string();
        assert!(
            run(&changed_args)
                .unwrap_err()
                .to_string()
                .contains("executable differs")
        );
        assert!(!Path::new(&changed_args[4]).exists());
    }
}
