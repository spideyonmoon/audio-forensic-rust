"""Generated-only manifest/lineage/intake failure controls; Python stdlib."""
import copy
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location("ingest_corpus", Path(__file__).resolve().parents[1]
                                              / "scripts/ingest_corpus.py")
intake = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(intake)


class CorpusIntakeTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
        (self.root / "a.wav").write_bytes(b"generated-audio-a")
        (self.root / "b.wav").write_bytes(b"generated-audio-b")
        (self.root / "notes.txt").write_text("Generated controls only; no source claim.\n", encoding="utf-8")
        self.binary = self.root / "engine.exe"
        self.binary.write_bytes(b"generated fake executable identity")
        self.path = self.root / "manifest.json"
        self.entries = [dict(id="a", group_id="group-a", split="development", file="a.wav",
                             history="documented", notes="notes.txt")]

    def manifest(self):
        self.path.write_text(json.dumps(dict(version=1, root=".", entries=self.entries)), encoding="utf-8")
        return self.path

    def second(self, **changes):
        self.entries.append(dict(self.entries[0], id="b", file="b.wav", **changes))

    def report(self, status="analyzed", pcm="1" * 64):
        return [dict(status=status, ancestry_verdict="INCONCLUSIVE", evidence_index=None,
                     engine_version="0.18.0", schema_version="0.18.0", policy_version="observations-only-v18",
                     stream={}, diagnostics=[], coverage=dict(start_seconds=0, reached_end=True,
                     requested_max_seconds=None, analysis_passes=2, decoded_pcm_sha256=pcm,
                     hash_sample_encoding="s32le_msb_aligned", analyzed_frames=1600))]

    def process(self, report=None, exit_code=0):
        return subprocess.CompletedProcess([], exit_code,
            json.dumps(report if report is not None else self.report()).encode(), b"generated log")

    def reject(self, message):
        with self.assertRaisesRegex(intake.IntakeError, message):
            intake.load_manifest(self.manifest())

    def test_original_snapshot_has_hashes_and_notes_without_truth_labels(self):
        _, prepared, groups = intake.load_manifest(self.manifest())
        record = prepared[0][0]
        self.assertEqual(groups, {"group-a": "development"})
        self.assertEqual(record["notes_text"], (self.root / "notes.txt").read_text())
        self.assertEqual(record["file_fingerprint"], intake.fingerprint(self.root / "a.wav"))
        self.assertIsNone(record["parent_id"])
        self.assertNotIn("ancestry_label", record)

    def test_group_cannot_cross_splits(self):
        self.second(split="locked_test")
        self.reject("crosses splits")

    def test_uncertain_histories_require_challenge(self):
        for history in ("claimed", "unknown"):
            self.entries[0]["history"] = history
            self.reject("belong in challenge")
        self.entries[0]["split"] = "challenge"
        intake.load_manifest(self.manifest())

    def test_duplicate_ids_paths_and_file_hashes_are_rejected(self):
        self.second(group_id="group-b")
        self.entries[1]["id"] = "a"
        self.reject("Duplicate id")
        self.entries[1]["id"] = "b"
        self.entries[1]["file"] = "a.wav"
        self.reject("Duplicate file path")
        self.entries[1]["file"] = "b.wav"
        (self.root / "b.wav").write_bytes((self.root / "a.wav").read_bytes())
        self.reject("Identical files")

    def test_parent_recipe_and_group_are_required(self):
        self.second(parent_id="missing", recipe=[dict(tool="generated", version="1", arguments=[])])
        self.reject("Missing parent")
        self.entries[1]["parent_id"] = "a"
        self.entries[1]["group_id"] = "group-b"
        self.reject("share a group")
        self.entries[1]["group_id"] = "group-a"
        intake.load_manifest(self.manifest())
        self.entries[1]["recipe"] = []
        self.reject("require a parent and recipe")

    def test_lineage_cycles_and_history_upgrades_are_rejected(self):
        recipe = [dict(tool="generated", version="1", arguments=["copy"])]
        self.second(parent_id="a", recipe=recipe)
        self.entries[0].update(parent_id="b", recipe=recipe)
        self.reject("Cyclic")
        self.entries[0].update(parent_id=None, recipe=[], history="unknown", split="challenge")
        self.entries[1]["split"] = "challenge"
        self.reject("cannot upgrade")

    def test_strict_schema_and_json(self):
        self.entries[0]["split_typo"] = "validation"
        self.reject("Unexpected")
        for data in ('{"version":1,"version":1}', '{"value":NaN}', '{"value":1e999}'):
            with self.assertRaises(intake.IntakeError):
                intake.parse_json(data)

    def test_paths_and_provenance_notes_are_checked(self):
        self.entries[0]["file"] = str(self.root / "a.wav")
        self.reject("relative")
        with tempfile.NamedTemporaryFile(dir=self.root.parent, suffix=".wav", delete=False) as handle:
            outside = Path(handle.name)
        self.entries[0]["file"] = "../" + outside.name
        self.addCleanup(outside.unlink)
        self.reject("outside root")
        self.entries[0]["file"] = "a.wav"
        (self.root / "notes.txt").write_text(" ", encoding="utf-8")
        self.reject("nonempty")

    def test_report_policy_prefix_and_exit_are_enforced(self):
        good = self.report()
        intake.validate_report(good, 0)
        for key, value, message in (("ancestry_verdict", "STRONG_INDICATORS", "policy"),
                                    ("evidence_index", 0, "policy"), ("policy_version", "scored", "policy")):
            bad = copy.deepcopy(good)
            bad[0][key] = value
            with self.assertRaisesRegex(intake.IntakeError, message):
                intake.validate_report(bad, 0)
        bad = copy.deepcopy(good)
        bad[0]["coverage"]["requested_max_seconds"] = 1
        with self.assertRaisesRegex(intake.IntakeError, "full-file"):
            intake.validate_report(bad, 0)
        with self.assertRaisesRegex(intake.IntakeError, "exit/status"):
            intake.validate_report(good, 1)
        for field, value in (("status", []), ("coverage", {})):
            bad = copy.deepcopy(good)
            bad[0][field] = value
            with self.assertRaises(intake.IntakeError):
                intake.validate_report(bad, 0)

    def test_intake_preserves_explicit_unsupported_and_continues(self):
        self.second()
        output = self.root / "result"
        with patch.object(intake.subprocess, "run", side_effect=[self.process(self.report("unsupported"), 1),
                                                                self.process()]):
            summary = intake.ingest(self.manifest(), self.binary, output, 5)
        self.assertFalse(summary["passed"])
        self.assertEqual([r["status"] for r in summary["records"]], ["unsupported", "analyzed"])
        self.assertFalse(summary["accuracy_evaluated"])
        self.assertTrue((output / "manifest-snapshot.json").exists())

    def test_decoded_duplicates_across_groups_fail(self):
        self.second(group_id="group-b")
        with patch.object(intake.subprocess, "run", return_value=self.process()):
            summary = intake.ingest(self.manifest(), self.binary, self.root / "result", 5)
        self.assertFalse(summary["passed"])
        self.assertIn("Identical decoded PCM", summary["records"][1]["error"])

    def test_locked_groups_are_reserved_without_analysis(self):
        self.entries[0]["split"] = "locked_test"
        with patch.object(intake.subprocess, "run") as run:
            summary = intake.ingest(self.manifest(), self.binary, self.root / "result", 5)
        run.assert_not_called()
        self.assertTrue(summary["passed"])
        self.assertEqual(summary["records"][0]["status"], "reserved")
        self.assertNotIn("coverage", summary["records"][0])

    def test_source_mutation_fails_and_is_not_repaired(self):
        def mutate(*args, **kwargs):
            (self.root / "a.wav").write_bytes(b"deliberate generated mutation")
            return self.process()
        with patch.object(intake.subprocess, "run", side_effect=mutate):
            summary = intake.ingest(self.manifest(), self.binary, self.root / "result", 5)
        self.assertFalse(summary["passed"])
        self.assertIn("Source changed", summary["records"][0]["error"])
        self.assertEqual((self.root / "a.wav").read_bytes(), b"deliberate generated mutation")

    def test_manifest_notes_and_binary_mutations_fail_the_final_receipt(self):
        def mutate(*args, **kwargs):
            (self.root / "notes.txt").write_text("changed generated notes")
            self.path.write_text("changed generated manifest")
            self.binary.write_bytes(b"changed generated binary")
            return self.process()
        with patch.object(intake.subprocess, "run", side_effect=mutate):
            summary = intake.ingest(self.manifest(), self.binary, self.root / "result", 5)
        self.assertFalse(summary["passed"])
        self.assertEqual(len(summary["errors"]), 3)

    def test_timeout_and_malformed_reports_remain_failures(self):
        for index, response in enumerate((subprocess.TimeoutExpired("generated", 5),
                                          subprocess.CompletedProcess([], 0, b"broken-json", b""))):
            with patch.object(intake.subprocess, "run", side_effect=[response]):
                summary = intake.ingest(self.manifest(), self.binary, self.root / str(index), 5)
            self.assertFalse(summary["passed"])
            self.assertEqual(summary["records"][0]["status"], "intake_failed")

    def test_results_are_never_overwritten(self):
        manifest = self.manifest()
        output = self.root / "result"
        with patch.object(intake.subprocess, "run", return_value=self.process()):
            self.assertTrue(intake.ingest(manifest, self.binary, output, 5)["passed"])
            before = (output / "summary.json").read_bytes()
            with self.assertRaises(FileExistsError):
                intake.ingest(manifest, self.binary, output, 5)
        self.assertEqual((output / "summary.json").read_bytes(), before)


if __name__ == "__main__":
    unittest.main()
