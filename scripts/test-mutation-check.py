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
    "print('   Compiling wasm-bindgen-test v0.3.50')\n"
    "if 'SYNTAX' in text:\n"
    "    print('error: could not compile `sample`'); sys.exit(101)\n"
    "if 'BAD' in text:\n"
    "    print('test sample::checks ... FAILED'); sys.exit(1)\n"
    "print('test sample::checks ... ok')\n"
)


def edit(old, new, count=1, file="sample.txt"):
    return {"file": file, "old": old, "new": new, "count": count}


# Trimmed literal excerpts from real wasm failure logs. Tests never read .evidence.
# (A) run-wasm-tests summary shape: `first failure [...]` + `FAILED <name>` lines.
# From .evidence/004-recovery/checker/relation/004-M1-matrix-tab-chrome-restored.log.
WASM_M1_FIRST_FAILURE = (
    "first failure [Rust panic/assertion]: "
    "presentation::layout_relation_highlight_tests::the_relations_tab_is_gone_and_facts_live_beside_the_fields "
    "at web/src/presentation/layout_relation_highlight_tests.rs:208:5 — no Inspector tab bar remains; "
    "follow-on symptom: 15 later test(s) hit 'cannot modify the panic hook from a panicking thread' "
    "in shared WASM state (re-run after fixing first failure)"
)
WASM_M1_OUTPUT = """\
--- presentation::layout_relation_highlight_tests:: ---
first failure [Rust panic/assertion]: presentation::layout_relation_highlight_tests::the_relations_tab_is_gone_and_facts_live_beside_the_fields at web/src/presentation/layout_relation_highlight_tests.rs:208:5 — no Inspector tab bar remains; follow-on symptom: 15 later test(s) hit 'cannot modify the panic hook from a panicking thread' in shared WASM state (re-run after fixing first failure)
run-wasm-tests: executed 16, failed 16 (filters: presentation::layout_relation_highlight_tests::)
  FAILED presentation::layout_relation_highlight_tests::a_key_removed_while_highlighted_leaves_nothing_highlighted
  FAILED presentation::layout_relation_highlight_tests::a_removed_anchor_key_leaves_its_follows_reference_inert
  FAILED presentation::layout_relation_highlight_tests::anchored_follows_navigates_to_its_key_without_writing
  FAILED presentation::layout_relation_highlight_tests::bent_key_explains_its_cumulative_shift_and_opens_its_row
  FAILED presentation::layout_relation_highlight_tests::breadcrumb_carries_chip_selects_its_column_without_writing
  FAILED presentation::layout_relation_highlight_tests::changing_the_selection_clears_the_highlight
  FAILED presentation::layout_relation_highlight_tests::hidden_reference_names_its_layer_and_still_navigates
  FAILED presentation::layout_relation_highlight_tests::hovering_a_breadcrumb_chip_highlights_only_its_keys
  FAILED presentation::layout_relation_highlight_tests::hovering_an_aggregate_part_highlights_only_its_key
  FAILED presentation::layout_relation_highlight_tests::hovering_an_inherited_column_highlights_only_its_keys
  FAILED presentation::layout_relation_highlight_tests::hovering_follows_highlights_the_anchor_key
  FAILED presentation::layout_relation_highlight_tests::keyboard_focus_highlights_and_blur_clears
  FAILED presentation::layout_relation_highlight_tests::navigating_a_reference_releases_its_highlight
  FAILED presentation::layout_relation_highlight_tests::selecting_parts_in_two_matrices_explains_the_single_context_chain
  FAILED presentation::layout_relation_highlight_tests::split_navigation_keeps_a_present_focus_label_on_the_same_half
  FAILED presentation::layout_relation_highlight_tests::the_relations_tab_is_gone_and_facts_live_beside_the_fields
"""
WASM_M1_EXPECTED = [
    "presentation::layout_relation_highlight_tests::a_key_removed_while_highlighted_leaves_nothing_highlighted",
    "presentation::layout_relation_highlight_tests::a_removed_anchor_key_leaves_its_follows_reference_inert",
    "presentation::layout_relation_highlight_tests::anchored_follows_navigates_to_its_key_without_writing",
    "presentation::layout_relation_highlight_tests::bent_key_explains_its_cumulative_shift_and_opens_its_row",
    "presentation::layout_relation_highlight_tests::breadcrumb_carries_chip_selects_its_column_without_writing",
    "presentation::layout_relation_highlight_tests::changing_the_selection_clears_the_highlight",
    "presentation::layout_relation_highlight_tests::hidden_reference_names_its_layer_and_still_navigates",
    "presentation::layout_relation_highlight_tests::hovering_a_breadcrumb_chip_highlights_only_its_keys",
    "presentation::layout_relation_highlight_tests::hovering_an_aggregate_part_highlights_only_its_key",
    "presentation::layout_relation_highlight_tests::hovering_an_inherited_column_highlights_only_its_keys",
    "presentation::layout_relation_highlight_tests::hovering_follows_highlights_the_anchor_key",
    "presentation::layout_relation_highlight_tests::keyboard_focus_highlights_and_blur_clears",
    "presentation::layout_relation_highlight_tests::navigating_a_reference_releases_its_highlight",
    "presentation::layout_relation_highlight_tests::selecting_parts_in_two_matrices_explains_the_single_context_chain",
    "presentation::layout_relation_highlight_tests::split_navigation_keeps_a_present_focus_label_on_the_same_half",
    "presentation::layout_relation_highlight_tests::the_relations_tab_is_gone_and_facts_live_beside_the_fields",
]
# (B) direct wasm cargo-test runner shape: `test <name> ... FAIL` + panicked/assertion.
# From .evidence/004-recovery/checker/component-followup/004-M6-component-relationships-suppressed.log.
WASM_M6_TEST = "presentation::layout_component_inspector_tests::mounted_component_facts_live_beside_the_fields_without_tabs"
WASM_M6_OUTPUT = """\
running 1 test
test presentation::layout_component_inspector_tests::mounted_component_facts_live_beside_the_fields_without_tabs ... FAIL

failures:

---- presentation::layout_component_inspector_tests::mounted_component_facts_live_beside_the_fields_without_tabs output ----
    error output:
        panicked at web/src/presentation/layout_component_inspector_tests.rs:2366:5:
        assertion `left == right` failed: the Relationships section renders beside the fields
          left: Some("")
         right: Some("Relationships")

test result: FAILED. 0 passed; 1 failed; 0 ignored; 227 filtered out; finished in 0.35s

console output:
    Invoking test: presentation::layout_component_inspector_tests::mounted_component_facts_live_beside_the_fields_without_tabs
    panicked at web/src/presentation/layout_component_inspector_tests.rs:2366:5:
    assertion `left == right` failed: the Relationships section renders beside the fields
      left: Some("")
     right: Some("Relationships")
        at http://127.0.0.1:35839/wasm-bindgen-test:1979:25
"""


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
        self.assertIsNone(results[0].get("first_failure"))
        self.assertIsNone(results[1].get("first_failure"))
        self.assertIsNone(results[2].get("first_failure"))
        self.assertTrue(all(r["restored_exact"] for r in results))
        self.assert_original()

    def test_native_logs_do_not_claim_wasm_failures_or_collapse_binary_results(self):
        for cargo_line in ("Downloaded wasm-bindgen-test v0.3.50",
                           "Running wasm-bindgen-test v0.3.50", "Dirty wasm-bindgen-test v0.3.50"):
            with self.subTest(cargo_line=cargo_line):
                output = f"{cargo_line}\ntest sample::checks ... FAILED\ntest sample::checks ... FAILED\n"
                self.assertIsNone(mutation_check.first_failure(output))
        with self.subTest(case="two native binaries with the same test name"):
            output = "test sample::checks ... FAILED\ntest sample::checks ... FAILED\n"
            self.assertEqual(mutation_check.classify(1, output, False),
                             ("killed", ["sample::checks", "sample::checks"]))

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

    def test_locked_runs_hold_the_build_lock_fd_inside_the_command(self):
        spec_path = Path(self.directory.name) / "spec.json"
        spec_path.write_text(json.dumps([
            {"name": "m1", "description": "", "edits": [edit("omega", "OMEGA")]}]))
        probe = ("import os, sys; fd = int(os.environ['BOARDSTUDIO_BUILD_LOCK_FD']); "
                 "os.fstat(fd); print('LOCK-FD-OK', fd)")
        home = Path(self.directory.name) / "home"
        with mock.patch.dict(mutation_check.os.environ, {"HOME": str(home)}), \
                contextlib.redirect_stdout(io.StringIO()):
            status = mutation_check.main([str(spec_path), "--root", str(self.root), "--locked",
                                          "--evidence-dir", str(self.evidence),
                                          "--command", f"{sys.executable} -c \"{probe}\""])
        self.assertEqual(status, 0)
        self.assertTrue((home / ".cache" / "tmp" / "boardstudio-build.lock").exists())
        logs = "".join(path.read_text() for path in self.evidence.glob("*.log"))
        self.assertIn("LOCK-FD-OK 9", logs)
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

    def test_wasm_run_summary_keeps_all_failed_names_and_first_cause(self):
        (self.root / "fixture.txt").write_text(WASM_M1_OUTPUT, encoding="utf-8")
        self.command = [sys.executable, "-c",
                        "import sys; sys.stdout.write(open('fixture.txt', encoding='utf-8').read()); sys.exit(1)"]
        results, status, _ = self.run_check(
            [{"name": "m-wasm", "description": "", "edits": [edit("omega", "OMEGA")]}],
            skip_baseline=True)
        self.assertEqual(status, 0)
        result = results[0]
        self.assertEqual(result["status"], "killed")
        self.assertEqual(result["failed_tests"], WASM_M1_EXPECTED)
        self.assertEqual(result.get("first_failure"), WASM_M1_FIRST_FAILURE)
        self.assertFalse(result["timed_out"])
        self.assertTrue(result["restored_exact"])
        self.assert_original()

    def test_wasm_direct_runner_keeps_fail_name_and_first_cause(self):
        (self.root / "fixture.txt").write_text(WASM_M6_OUTPUT, encoding="utf-8")
        self.command = [sys.executable, "-c",
                        "import sys; sys.stdout.write(open('fixture.txt', encoding='utf-8').read()); sys.exit(1)"]
        results, status, _ = self.run_check(
            [{"name": "m-wasm", "description": "", "edits": [edit("omega", "OMEGA")]}],
            skip_baseline=True)
        self.assertEqual(status, 0)
        result = results[0]
        self.assertEqual(result["status"], "killed")
        self.assertEqual(result["failed_tests"], [WASM_M6_TEST])
        cause = result.get("first_failure")
        self.assertIn(WASM_M6_TEST, cause)
        self.assertIn("web/src/presentation/layout_component_inspector_tests.rs:2366:5", cause)
        self.assertIn("the Relationships section renders beside the fields", cause)
        self.assertFalse(result["timed_out"])
        self.assertTrue(result["restored_exact"])
        self.assert_original()

    def test_python_failed_footer_is_not_a_failed_test_name(self):
        # Checker RX1: `FAILED (failures=8)` must not become `["(failures=8)"]`.
        output = WASM_M1_OUTPUT + "FAILED (failures=8)\n"
        (self.root / "fixture.txt").write_text(output, encoding="utf-8")
        self.command = [sys.executable, "-c",
                        "import sys; sys.stdout.write(open('fixture.txt', encoding='utf-8').read()); sys.exit(1)"]
        results, status, _ = self.run_check(
            [{"name": "m-wasm", "description": "", "edits": [edit("omega", "OMEGA")]}],
            skip_baseline=True)
        self.assertEqual(status, 0)
        result = results[0]
        self.assertEqual(result["status"], "killed")
        self.assertEqual(result["failed_tests"], WASM_M1_EXPECTED)
        self.assertEqual(result.get("first_failure"), WASM_M1_FIRST_FAILURE)
        self.assertFalse(result["timed_out"])
        self.assertTrue(result["restored_exact"])
        self.assert_original()


if __name__ == "__main__":
    unittest.main()
