#!/usr/bin/env python3
from __future__ import annotations

import contextlib
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest import mock

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("mutation_check", HERE / "mutation-check.py")
mutation_check = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(mutation_check)

SOURCE = "alpha GOOD omega\nsecond GOOD line\n"
# A stand-in for a test suite: red when the marker BAD is present, a build error when SYNTAX is.
PROBE = (
    "import pathlib, sys\n"
    "text = pathlib.Path('sample.txt').read_text()\n"
    "if 'SYNTAX' in text:\n"
    "    print('error: could not compile `sample`'); sys.exit(101)\n"
    "if 'BAD' in text:\n"
    "    print('test sample::checks ... FAILED'); sys.exit(1)\n"
    "print('test sample::checks ... ok')\n"
)


def edit(old, new, count=1, file="sample.txt"):
    return {"file": file, "old": old, "new": new, "count": count}


class MutationCheckTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name) / "tree"
        self.root.mkdir()
        self.sample = self.root / "sample.txt"
        self.sample.write_bytes(SOURCE.encode())
        self.original_hash = hashlib.sha256(SOURCE.encode()).hexdigest()
        self.evidence = Path(self.directory.name) / "evidence"
        self.command = [sys.executable, "-c", PROBE]

    def run_check(self, spec, **kwargs):
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()) as errors:
            results, status = mutation_check.check(self.root, spec, self.command, self.evidence, **kwargs)
        return results, status, errors.getvalue()

    def assert_original(self):
        self.assertEqual(hashlib.sha256(self.sample.read_bytes()).hexdigest(), self.original_hash)

    def test_killed_survived_and_compile_failure_are_told_apart(self):
        spec = [
            {"name": "m-killed", "description": "makes the marker appear", "edits": [edit("alpha GOOD", "alpha BAD")]},
            {"name": "m-survived", "description": "a change no test notices", "edits": [edit("omega", "OMEGA")]},
            {"name": "m-compile", "description": "does not build", "edits": [edit("second GOOD", "SYNTAX")]},
        ]
        results, status, _ = self.run_check(spec)
        self.assertEqual(status, 0)
        self.assertEqual({r["name"]: r["status"] for r in results}, {
            "m-killed": "killed",
            "m-survived": "survived",
            "m-compile": "compile-failure (does not count)",
        })
        self.assertEqual(results[0]["failed_tests"], ["sample::checks"])
        self.assertTrue(all(r["restored_exact"] for r in results))
        self.assert_original()

    def test_evidence_logs_and_hashes_are_written(self):
        results, _, _ = self.run_check([{"name": "m1", "description": "", "edits": [edit("alpha GOOD", "alpha BAD")]}])
        self.assertIn("FAILED", (self.evidence / "m1.log").read_text())
        recorded = json.loads((self.evidence / "results.json").read_text())["results"][0]
        self.assertEqual(recorded["files"]["sample.txt"]["sha256_before"], self.original_hash)
        self.assertEqual(recorded["files"]["sample.txt"]["sha256_after"], self.original_hash)
        self.assertEqual(results[0]["status"], "killed")

    def test_the_mutation_is_visible_to_the_command_while_it_runs(self):
        seen = []
        real = mutation_check.run_command

        def spy(command, root, timeout):
            seen.append((root / "sample.txt").read_text())
            return real(command, root, timeout)

        with mock.patch.object(mutation_check, "run_command", side_effect=spy):
            self.run_check([{"name": "m1", "description": "", "edits": [edit("alpha GOOD", "alpha BAD")]}],
                           skip_baseline=True)
        self.assertEqual(seen, ["alpha BAD omega\nsecond GOOD line\n"])

    def test_a_count_mismatch_runs_nothing_and_touches_nothing(self):
        spec = [{"name": "m1", "description": "", "edits": [edit("GOOD", "BAD", count=1)]}]  # occurs twice
        with mock.patch.object(mutation_check, "run_command") as run:
            results, status, errors = self.run_check(spec)
        self.assertEqual((results, status), ([], 2))
        self.assertIn("expected 1 occurrence(s)", errors)
        self.assertIn("found 2", errors)
        run.assert_not_called()
        self.assert_original()

    def test_multi_occurrence_counts_are_applied_to_every_match(self):
        spec = [{"name": "m1", "description": "", "edits": [edit("GOOD", "BAD", count=2)]}]
        results, status, _ = self.run_check(spec)
        self.assertEqual((status, results[0]["status"]), (0, "killed"))
        self.assert_original()

    def test_a_red_unmutated_run_is_refused(self):
        self.sample.write_bytes(b"already BAD\n")
        results, status, errors = self.run_check([{"name": "m1", "description": "", "edits": [edit("already", "still")]}])
        self.assertEqual((results, status), ([], 2))
        self.assertIn("unmutated tests do not pass", errors)

    def test_files_are_restored_when_the_run_is_interrupted(self):
        with mock.patch.object(mutation_check, "run_command", side_effect=KeyboardInterrupt), \
                self.assertRaises(KeyboardInterrupt):
            self.run_check([{"name": "m1", "description": "", "edits": [edit("alpha GOOD", "alpha BAD")]}],
                           skip_baseline=True)
        self.assert_original()

    def test_files_are_restored_when_the_command_cannot_start(self):
        self.command = ["/nonexistent/test-runner"]
        with self.assertRaises(OSError):
            self.run_check([{"name": "m1", "description": "", "edits": [edit("alpha GOOD", "alpha BAD")]}],
                           skip_baseline=True)
        self.assert_original()

    def test_a_file_that_does_not_restore_fails_the_run(self):
        real = mutation_check.write_bytes
        writes = []

        def apply_then_forget(path, data):
            writes.append(path)
            if len(writes) == 1:
                real(path, data)  # the mutation is applied; the restoring write is lost

        spec = [{"name": "m1", "description": "", "edits": [edit("omega", "OMEGA")]}]
        with mock.patch.object(mutation_check, "write_bytes", side_effect=apply_then_forget):
            results, status, errors = self.run_check(spec, skip_baseline=True)
        self.assertEqual(status, 1)
        self.assertFalse(results[0]["restored_exact"])
        self.assertIn("did not restore", errors)

    def test_a_hanging_mutant_counts_as_killed(self):
        self.command = [sys.executable, "-c", "import time; time.sleep(60)"]
        results, status, _ = self.run_check(
            [{"name": "m1", "description": "", "edits": [edit("omega", "OMEGA")]}],
            skip_baseline=True, timeout=0.5)
        self.assertEqual((status, results[0]["status"], results[0]["timed_out"]), (0, "killed", True))
        self.assert_original()

    def test_the_cli_uses_the_default_evidence_directory_and_reports_a_summary(self):
        spec_path = Path(self.directory.name) / "spec.json"
        spec_path.write_text(json.dumps([
            {"name": "m1", "description": "", "edits": [edit("alpha GOOD", "alpha BAD")]},
            {"name": "m2", "description": "", "edits": [edit("omega", "OMEGA")]}]))
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            status = mutation_check.main([str(spec_path), "--root", str(self.root),
                                          "--command", f"{sys.executable} -c \"{PROBE}\""])
        self.assertEqual(status, 0)
        self.assertIn("1 killed, 1 survived, 0 compile-failure", output.getvalue())
        self.assertTrue((self.root / ".evidence" / "mutations" / "results.json").is_file())
        self.assert_original()

    def test_validate_checks_the_spec_without_running(self):
        spec_path = Path(self.directory.name) / "spec.json"
        spec_path.write_text(json.dumps([{"name": "m1", "description": "", "edits": [edit("GOOD", "BAD", count=5)]}]))
        with mock.patch.object(mutation_check, "run_command") as run, \
                contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(mutation_check.main([str(spec_path), "--root", str(self.root), "--validate"]), 2)
        run.assert_not_called()

    def test_bad_specs_are_rejected(self):
        bad = [
            [],
            [{"name": "has space", "edits": [edit("a", "b")]}],
            [{"name": "ok", "edits": []}],
            [{"name": "ok", "edits": [edit("a", "b", file="../outside.txt")]}],
            [{"name": "ok", "edits": [edit("same", "same")]}],
            [{"name": "ok", "edits": [edit("a", "b", count=0)]}],
            [{"name": "dup", "edits": [edit("a", "b")]}, {"name": "dup", "edits": [edit("a", "b")]}],
        ]
        for spec in bad:
            with self.subTest(spec=spec):
                path = Path(self.directory.name) / "bad.json"
                path.write_text(json.dumps(spec))
                with self.assertRaises(mutation_check.SpecError):
                    mutation_check.load_spec(path)

    def test_the_default_evidence_directory_is_gitignored(self):
        ignored = (HERE.parent / ".gitignore").read_text().splitlines()
        self.assertIn(".evidence/", ignored)


if __name__ == "__main__":
    unittest.main()
