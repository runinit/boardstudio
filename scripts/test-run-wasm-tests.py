#!/usr/bin/env python3
"""Tests for run-wasm-tests.py using a fake runner (no Chrome, no cargo)."""

import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import re
import hashlib
import subprocess
import tempfile
import unittest
from urllib.error import HTTPError, URLError
from urllib.request import urlopen
from unittest.mock import patch

SCRIPT = Path(__file__).with_name("run-wasm-tests.py")
spec = importlib.util.spec_from_file_location("run_wasm_tests", SCRIPT)
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


class RunWasmTestsTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="run-wasm-tests-")
        self.dir = Path(self.temp.name)
        self.log = self.dir / "calls.log"
        self.known = self.dir / "known.json"
        self.known.write_text(json.dumps([{"test": "known_broken", "reason": "RF-033"}]))
        self._saved = (runner.KNOWN_FAILURES, os.environ.get(runner.RUNNER_ENV))
        runner.KNOWN_FAILURES = self.known
        # Minimal repo so #[path] scanning and --root work.
        (self.dir / "web/src/presentation").mkdir(parents=True)

    def tearDown(self):
        runner.KNOWN_FAILURES = self._saved[0]
        if self._saved[1] is None:
            os.environ.pop(runner.RUNNER_ENV, None)
        else:
            os.environ[runner.RUNNER_ENV] = self._saved[1]
        self.temp.cleanup()

    def fake(self, body):
        script = self.dir / "fake-runner"
        names = re.findall(r"test ([A-Za-z0-9_:]+) \.\.\. (?:ok|FAILED|FAIL)", body)
        names += re.findall(r"Invoking test: ([A-Za-z0-9_:]+)", body)
        if "--list" not in body:
            names = names or ["unrelated::placeholder"]
            listing = " ".join(f"'{name}: test'" for name in sorted(set(names)))
            body = f'if [ "$1" = --list ]; then printf "%s\\n" {listing}; exit 0; fi\n{body}'
        script.write_text(f"#!/bin/sh\necho \"$@\" >> '{self.log}'\n{body}\n")
        script.chmod(0o755)
        os.environ[runner.RUNNER_ENV] = str(script)

    def main(self, *argv):
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = runner.main(["--root", str(self.dir), *argv])
        return code, out.getvalue(), err.getvalue()

    def test_file_to_module_filter_mapping(self):
        f = runner.module_filter
        self.assertEqual(f("web/src/presentation/keymap/binding_editor.rs"), "presentation::keymap::binding_editor::")
        self.assertEqual(f("web/src/presentation/keymap/mod.rs"), "presentation::keymap::")
        self.assertEqual(f("web/src/presentation/panels.rs"), "presentation::panels::")
        self.assertEqual(f("web/src/runtime.rs"), "runtime::")
        self.assertEqual(f("web/src/main.rs"), "")
        with self.assertRaises(runner.RunnerError):
            f("docs/readme.md")

    def test_path_attribute_modules_follow_their_declaring_module(self):
        (self.dir / "web/src/main.rs").write_text("mod presentation;\n")
        (self.dir / "web/src/presentation.rs").write_text("mod panels;\n")
        (self.dir / "web/src/presentation/panels.rs").write_text(
            '#[cfg(test)]\n#[path = "panels_scroll_tests.rs"]\nmod scroll_tests;\n')
        (self.dir / "web/src/presentation/panels_scroll_tests.rs").write_text("")
        filters = runner.filters_for_files(["web/src/presentation/panels_scroll_tests.rs"], self.dir)
        self.assertEqual(filters, ["presentation::panels::scroll_tests::"])

    def test_path_attribute_modules_ignore_aliases_inactive_for_wasm(self):
        (self.dir / "web/src/main.rs").write_text(
            '#[cfg(target_arch = "wasm32")]\nmod presentation;\n'
            '#[cfg(all(test, not(target_arch = "wasm32")))]\n'
            '#[path = "presentation/case_display.rs"]\nmod case_display;\n')
        (self.dir / "web/src/presentation.rs").write_text("mod case_display;\n")
        (self.dir / "web/src/presentation/case_display.rs").write_text("")
        filters = runner.filters_for_files(["web/src/presentation/case_display.rs"], self.dir)
        self.assertEqual(filters, ["presentation::case_display::"])

    def test_path_attribute_modules_accept_dot_root(self):
        (self.dir / "web/src/main.rs").write_text(
            '#[cfg(target_arch = "wasm32")]\n'
            '#[path = "presentation/case_display.rs"]\nmod case_display;\n')
        (self.dir / "web/src/presentation/case_display.rs").write_text("")
        original = Path.cwd()
        os.chdir(self.dir)
        try:
            modules = runner.path_attr_modules(".")
        finally:
            os.chdir(original)
        self.assertEqual(modules, {
            "web/src/presentation/case_display.rs": ("web/src/main.rs", "case_display")
        })

    def test_nested_filters_collapse_into_their_prefix(self):
        filters = runner.filters_for_files([
            "web/src/presentation/keymap/binding_editor.rs",
            "web/src/presentation/keymap/mod.rs",
            "web/src/presentation/layout_camera.rs",
        ], self.dir)
        self.assertEqual(filters, ["presentation::keymap::", "presentation::layout_camera::"])

    def test_files_mode_runs_each_filter_and_passes(self):
        self.fake("echo 'test presentation::layout_camera::tests::a ... ok'")
        code, out, _ = self.main("--files", "web/src/presentation/layout_camera.rs")
        self.assertEqual(code, 0, out)
        self.assertIn("executed 1, failed 0", out)
        self.assertEqual(self.log.read_text().splitlines(), ["--list", "presentation::layout_camera::tests::"])

    def test_files_mode_expands_parent_prefix_to_isolated_matching_modules(self):
        self.fake(
            'if [ "$1" = --list ]; then printf "%s\\n" '
            '"cad_presentation::mounted_tests::mounted_case_panel_starts_once: test" '
            '"presentation::panels::scroll_tests::compact_case_inspector: test" '
            '"presentation::mechanical_settings::contextual_layer_tests::selected_plate: test"; exit 0; fi\n'
            'case "$1" in presentation::panels::scroll_tests::) '
            'echo "test ${1}compact_case_inspector ... ok";; '
            'presentation::mechanical_settings::contextual_layer_tests::) '
            'echo "test ${1}selected_plate ... ok";; esac')
        code, out, err = self.main("--files", "web/src/presentation.rs")
        self.assertEqual(code, 0, err)
        self.assertIn("executed 2, failed 0", out)
        calls = self.log.read_text().splitlines()
        self.assertEqual(calls, [
            "--list",
            "presentation::mechanical_settings::contextual_layer_tests::",
            "presentation::panels::scroll_tests::",
        ])

    def test_files_mode_preserves_zero_test_failure_when_listing_has_no_match(self):
        self.fake(
            'if [ "$1" = --list ]; then echo "cad_presentation::other::test: test"; exit 0; fi\n'
            'echo "running 0 tests"')
        code, _, err = self.main("--files", "web/src/presentation/unused.rs")
        self.assertEqual(code, 1)
        self.assertIn("zero tests executed for filter presentation::unused::", err)
        self.assertEqual(self.log.read_text().splitlines(), ["--list"])

    def test_files_mode_rejects_nonzero_partial_test_listing(self):
        self.fake(
            'if [ "$1" = --list ]; then echo "presentation::panels::a: test"; exit 1; fi\n'
            'echo "test presentation::panels::a ... ok"')
        code, _, err = self.main("--files", "web/src/presentation.rs")
        self.assertEqual(code, 2)
        self.assertIn("could not list wasm tests", err)

    def test_files_mode_rejects_listed_tests_omitted_after_known_failure(self):
        self.fake(
            'if [ "$1" = --list ]; then printf "%s\\n" '
            '"presentation::x::tests::known_broken: test" '
            '"presentation::x::tests::not_run: test"; exit 0; fi\n'
            'echo "test ${1}known_broken ... FAILED"; exit 1')
        code, _, err = self.main("--files", "web/src/presentation/x.rs")
        self.assertEqual(code, 1)
        self.assertIn("listed tests were not completed", err)
        self.assertIn("presentation::x::tests::not_run", err)

    def test_files_mode_rejects_substring_collisions_outside_selected_prefix(self):
        self.fake(
            'if [ "$1" = --list ]; then printf "%s\\n" '
            '"presentation::x::tests::known_broken: test" '
            '"cad_presentation::x::tests::known_broken: test"; exit 0; fi\n'
            'echo "test presentation::x::tests::known_broken ... FAILED"\n'
            'echo "test cad_presentation::x::tests::known_broken ... FAILED"; exit 1')
        code, _, err = self.main("--files", "web/src/presentation/x.rs")
        self.assertEqual(code, 1)
        self.assertIn("unlisted tests matched filter", err)
        self.assertIn("cad_presentation::x::tests::known_broken", err)

    def test_real_failure_fails_the_run_and_is_named(self):
        self.fake("echo 'test presentation::x::tests::good ... ok'; echo 'test presentation::x::tests::bad ... FAIL'; echo 'failure detail: expected marker'; exit 1")
        code, out, err = self.main("--files", "web/src/presentation/x.rs")
        self.assertEqual(code, 1)
        self.assertIn("executed 2, failed 1", out)
        self.assertIn("FAILED presentation::x::tests::bad", out)
        self.assertIn("failure detail: expected marker", err)

    def test_known_failure_is_tolerated(self):
        self.fake("echo 'test presentation::x::tests::good ... ok'; echo 'test presentation::x::tests::known_broken ... FAIL'; exit 1")
        code, out, _ = self.main("--files", "web/src/presentation/x.rs")
        self.assertEqual(code, 0, out)
        self.assertIn("failed 0", out)

    def test_known_failure_that_now_passes_prints_a_note_but_succeeds(self):
        self.fake("echo 'test presentation::x::tests::known_broken ... ok'")
        code, out, _ = self.main("--files", "web/src/presentation/x.rs")
        self.assertEqual(code, 0, out)
        self.assertIn("known failure now passes", out)
        self.assertIn("known_broken", out)

    def test_zero_executed_tests_for_a_filter_fails(self):
        self.fake("echo 'running 0 tests'; echo 'test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out'")
        code, _, err = self.main("--files", "web/src/presentation/x.rs")
        self.assertEqual(code, 1)
        self.assertIn("zero tests executed for filter presentation::x::", err)

    def test_zero_executed_does_not_widen_to_an_unrelated_parent_module(self):
        self.fake('if [ "$1" = --list ]; then printf "%s\\n" "presentation::a::b::sibling::t: test"; exit 0; fi\n'
                  'echo "test presentation::a::b::sibling::t ... ok"')
        code, out, err = self.main("--files", "web/src/presentation/a/b/c.rs")
        self.assertEqual(code, 1, out)
        self.assertIn("no direct module or reviewed owner mapping; fail closed", err)
        self.assertEqual(self.log.read_text().split(), ["--list"])

    def test_invoked_but_never_reported_test_counts_as_failed(self):
        self.fake("echo '    Invoking test: presentation::x::tests::hangs'; exit 1")
        code, out, err = self.main("--files", "web/src/presentation/x.rs")
        self.assertEqual(code, 1)
        self.assertIn("presentation::x::tests::hangs", err)
        self.assertIn("incomplete execution", err)

    def test_allowlisted_invoked_but_unreported_test_still_fails(self):
        self.fake("echo '    Invoking test: presentation::x::tests::known_broken'; exit 1")
        code, out, err = self.main("--files", "web/src/presentation/x.rs")
        self.assertEqual(code, 1, out)
        self.assertIn("incomplete", err.lower())

    def test_build_error_without_failing_test_fails(self):
        self.fake("echo 'test presentation::x::tests::good ... ok'; echo 'error: build exploded' >&2; exit 1")
        code, _, err = self.main("--files", "web/src/presentation/x.rs")
        self.assertEqual(code, 1)
        self.assertIn("without a failing test", err)

    def test_all_mode_lists_tests_and_runs_one_filter_per_module(self):
        self.fake(
            'if [ "$1" = --list ]; then printf "%s\\n" "a::m1::t1: test" "a::m1::t2: test" "b::m2::t3: test"; exit 0; fi\n'
            'case "$1" in a::m1::) echo "test a::m1::t1 ... ok"; echo "test a::m1::t2 ... ok";; '
            'b::m2::) echo "test b::m2::t3 ... ok";; esac'
        )
        code, out, _ = self.main("--all")
        self.assertEqual(code, 0, out)
        calls = self.log.read_text().split("\n")
        self.assertIn("--list", calls[0])
        self.assertEqual(sorted(c for c in calls[1:] if c), ["a::m1::", "b::m2::"])
        self.assertIn("executed 3", out)

    def test_all_mode_rejects_listed_tests_missing_from_terminal_outcomes(self):
        self.fake(
            'if [ "$1" = --list ]; then printf "%s\\n" '
            '"a::m1::t1: test" "a::m1::t2: test" "b::m2::t3: test"; exit 0; fi\n'
            'case "$1" in a::m1::) echo "test a::m1::t1 ... ok";; '
            'b::m2::) echo "test b::m2::t3 ... ok";; esac'
        )
        code, _, err = self.main("--all")
        self.assertEqual(code, 1)
        self.assertIn("listed tests were not completed", err)
        self.assertIn("a::m1::t2", err)

    def test_all_mode_rejects_unlisted_outcomes(self):
        self.fake(
            'if [ "$1" = --list ]; then echo "a::m1::t1: test"; exit 0; fi\n'
            'echo "test a::m1::t1 ... ok"; echo "test a::m1::invented ... ok"'
        )
        code, _, err = self.main("--all")
        self.assertEqual(code, 1)
        self.assertIn("unlisted tests matched filter", err)
        self.assertIn("a::m1::invented", err)

    def test_all_depth_mode_rejects_unexpected_and_incomplete_outcomes(self):
        self.fake(
            'if [ "$1" = --list ]; then printf "%s\\n" '
            '"a::m1::t1: test" "a::m2::t2: test"; exit 0; fi\n'
            'echo "test a::m1::t1 ... ok"\n'
            'echo "    Invoking test: a::m2::t2"\n'
            'echo "test a::m1::extra ... ok"'
        )
        code, _, err = self.main("--all", "--depth", "1")
        self.assertEqual(code, 1)
        self.assertIn("incomplete execution", err)
        self.assertIn("unlisted tests matched", err)

    def test_result_json_distinguishes_complete_known_failure_from_success(self):
        self.fake("echo 'test presentation::x::tests::known_broken ... FAIL'; exit 1")
        report_path = self.dir / "report.json"
        code, _, _ = self.main("--files", "web/src/presentation/x.rs", "--result-json", str(report_path))
        self.assertEqual(code, 0)
        report = json.loads(report_path.read_text())
        self.assertTrue(report["complete"])
        self.assertEqual(report["passed_count"], 0)
        self.assertEqual(report["failed_count"], 1)
        self.assertEqual(report["failed_known_exclusions"], ["presentation::x::tests::known_broken"])
        self.assertEqual(report["expected_tests"], ["presentation::x::tests::known_broken"])
        self.assertEqual(report["terminal_outcomes"], [{
            "test": "presentation::x::tests::known_broken", "status": "failed"
        }])
        self.assertEqual(len(report["filters"]), 1)
        self.assertGreaterEqual(report["filters"][0]["duration_ms"], 0)

    def test_result_json_marks_incomplete_and_inventory_problems(self):
        self.fake(
            'if [ "$1" = --list ]; then echo "presentation::x::tests::hangs: test"; exit 0; fi\n'
            'echo "    Invoking test: presentation::x::tests::hangs"; exit 1'
        )
        report_path = self.dir / "incomplete.json"
        code, _, _ = self.main("--files", "web/src/presentation/x.rs", "--result-json", str(report_path))
        self.assertEqual(code, 1)
        report = json.loads(report_path.read_text())
        self.assertFalse(report["complete"])
        self.assertEqual(report["incomplete_count"], 1)
        self.assertTrue(report["problems"])

    def test_result_json_records_empty_selection_as_incomplete(self):
        self.fake('if [ "$1" = --list ]; then echo "cad_presentation::other::test: test"; exit 0; fi\n'
                  'echo "running 0 tests"')
        report_path = self.dir / "empty.json"
        code, _, _ = self.main("--files", "web/src/presentation/unused.rs", "--result-json", str(report_path))
        self.assertEqual(code, 1)
        report = json.loads(report_path.read_text())
        self.assertFalse(report["complete"])
        self.assertEqual(report["expected_tests"], [])
        self.assertEqual(report["passed_count"], 0)
        self.assertTrue(report["problems"])

    def test_result_json_records_listing_error(self):
        self.fake('if [ "$1" = --list ]; then echo "partial: test"; exit 1; fi\n'
                  'echo "test presentation::x::tests::a ... ok"')
        report_path = self.dir / "listing-error.json"
        code, _, _ = self.main("--files", "web/src/presentation/x.rs", "--result-json", str(report_path))
        self.assertEqual(code, 2)
        report = json.loads(report_path.read_text())
        self.assertFalse(report["complete"])
        self.assertEqual(report["listed_tests"], [])
        self.assertTrue(report["problems"])

    def test_conflicting_duplicate_terminal_outcomes_fail_and_are_preserved(self):
        name = "presentation::x::tests::ambiguous"
        self.fake(
            f'if [ "$1" = --list ]; then echo "{name}: test"; exit 0; fi\n'
            f'echo "test {name} ... FAILED"\n'
            f'echo "test {name} ... ok"\n'
            'exit 0'
        )
        report_path = self.dir / "duplicate-conflict.json"
        code, _, err = self.main(
            "--files", "web/src/presentation/x.rs", "--result-json", str(report_path)
        )
        self.assertEqual(code, 1)
        self.assertIn("duplicate terminal outcomes", err)
        report = json.loads(report_path.read_text())
        self.assertFalse(report["complete"])
        self.assertEqual(report["failed_count"], 1)
        self.assertEqual(report["duplicate_terminal_outcomes"], [name])
        self.assertEqual(report["terminal_outcomes"], [
            {"test": name, "status": "failed"},
            {"test": name, "status": "passed"},
        ])

    def test_identical_duplicate_terminal_outcomes_still_fail(self):
        name = "presentation::x::tests::repeated"
        self.fake(
            f'if [ "$1" = --list ]; then echo "{name}: test"; exit 0; fi\n'
            f'echo "test {name} ... ok"\n'
            f'echo "test {name} ... ok"'
        )
        code, _, err = self.main("--files", "web/src/presentation/x.rs")
        self.assertEqual(code, 1)
        self.assertIn("duplicate terminal outcomes", err)

    def test_duplicate_authoritative_list_names_fail_closed_and_are_reported(self):
        name = "presentation::x::tests::listed_twice"
        self.fake(
            f'if [ "$1" = --list ]; then printf "%s\\n" "{name}: test" "{name}: test"; exit 0; fi\n'
            f'echo "test {name} ... ok"'
        )
        report_path = self.dir / "duplicate-list.json"
        code, _, err = self.main(
            "--files", "web/src/presentation/x.rs", "--result-json", str(report_path)
        )
        self.assertEqual(code, 2)
        self.assertIn("duplicate test names in wasm listing", err)
        report = json.loads(report_path.read_text())
        self.assertFalse(report["complete"])
        self.assertEqual(report["listed_tests"], [name, name])
        self.assertEqual(report["duplicate_listed_tests"], [name])

    def test_guarded_root_owner_preserves_independent_child_and_falls_back_on_any_hash_change(self):
        source = "web/src/presentation.rs"
        base_bytes, current_bytes = b"committed presentation", b"reviewed helper-only presentation"
        source_path = self.dir / source
        source_path.parent.mkdir(parents=True, exist_ok=True)
        source_path.write_bytes(base_bytes)
        subprocess.run(["git", "init", "-q", str(self.dir)], check=True)
        subprocess.run(["git", "-C", str(self.dir), "config", "user.email", "tests@example.invalid"], check=True)
        subprocess.run(["git", "-C", str(self.dir), "config", "user.name", "Test"], check=True)
        subprocess.run(["git", "-C", str(self.dir), "add", source], check=True)
        subprocess.run(["git", "-C", str(self.dir), "commit", "-qm", "base"], check=True)
        source_path.write_bytes(current_bytes)
        case_test = "cad_presentation::mounted_tests::mounted_case_local_export_uses_current_exact_mechanical_scope"
        inspector_test = "presentation::objects::matrix_transform_inspector::mounted_tests::choice"
        unrelated = "presentation::keymap::unrelated::test"
        names = [case_test, inspector_test, unrelated]
        owner_path = self.dir / "owners.json"
        owner_path.write_text(json.dumps({"schema_version": 1, "sources": {
            source: {"tests": [case_test], "coverage": "mounted Case scope",
                    "base_sha256": hashlib.sha256(base_bytes).hexdigest(),
                    "current_sha256": hashlib.sha256(current_bytes).hexdigest()},
            "web/src/presentation/objects/matrix_transform_inspector.rs": {
                "tests": [inspector_test], "coverage": "mounted Inspector choice"}
        }}))

        with patch.object(runner, "list_wasm_tests", return_value=names):
            filters, expected, unmatched, _, diagnostics = runner.selection_for_listed_files(
                [source, "web/src/presentation/objects/matrix_transform_inspector.rs"], {},
                self.dir, owner_path)
        self.assertEqual(filters, sorted([case_test, inspector_test]))
        self.assertEqual(expected, {case_test: [case_test], inspector_test: [inspector_test]})
        self.assertEqual(unmatched, [])
        self.assertEqual(len(diagnostics), 2)

        # A changed committed revision or working copy disables the narrow root owner.
        def fallback():
            with patch.object(runner, "list_wasm_tests", return_value=names):
                return runner.selection_for_listed_files([source], {}, self.dir, owner_path)
        source_path.write_bytes(current_bytes + b" changed")
        filters, expected, unmatched, _, diagnostics = fallback()
        self.assertIn(unrelated, [name for group in expected.values() for name in group])
        self.assertTrue(any("presentation::" in line and "guard mismatch; conservative fallback" in line
                            for line in diagnostics))
        self.assertEqual(unmatched, [])
        source_path.write_bytes(b"changed committed base")
        subprocess.run(["git", "-C", str(self.dir), "add", source], check=True)
        subprocess.run(["git", "-C", str(self.dir), "commit", "-qm", "new base"], check=True)
        source_path.write_bytes(current_bytes)
        filters, expected, unmatched, _, diagnostics = fallback()
        self.assertIn(unrelated, [name for group in expected.values() for name in group])
        self.assertTrue(any("presentation::" in line and "guard mismatch; conservative fallback" in line
                            for line in diagnostics))

    def test_owner_mapping_selects_exact_listed_tests_and_diagnoses_coverage(self):
        owner_path = self.dir / "owners.json"
        owner_path.write_text(json.dumps({"schema_version": 1, "sources": {
            "web/src/presentation/objects/matrix_transform_inspector.rs": {
                "tests": ["presentation::objects::matrix_transform_inspector::mounted_tests::choice"],
                "coverage": "mounted choices and replacement event"
            }
        }}))
        names = ["presentation::objects::matrix_transform_inspector::mounted_tests::choice",
                 "presentation::objects::matrix_transform_inspector::unrelated::other"]
        with patch.object(runner, "TEST_OWNERS", owner_path), patch.object(runner, "list_wasm_tests", return_value=names):
            filters, expected, unmatched, listed, diagnostics = runner.selection_for_listed_files(
                ["web/src/presentation/objects/matrix_transform_inspector.rs"], {}, self.dir)
        self.assertEqual(filters, [names[0]])
        self.assertEqual(expected, {names[0]: [names[0]]})
        self.assertEqual(unmatched, [])
        self.assertEqual(listed, names)
        self.assertIn("owner map (mounted choices and replacement event)", diagnostics[0])

    def test_owner_mapping_fails_closed_when_a_reviewed_test_is_not_listed(self):
        owner_path = self.dir / "owners.json"
        owner_path.write_text(json.dumps({"schema_version": 1, "sources": {
            "web/src/presentation/objects/matrix_transform_inspector.rs": {
                "tests": ["presentation::objects::matrix_transform_inspector::mounted_tests::missing"],
                "coverage": "mounted controller-to-Inspector replacement"
            }
        }}))
        with patch.object(runner, "TEST_OWNERS", owner_path), patch.object(
                runner, "list_wasm_tests", return_value=["presentation::objects::matrix_transform_inspector::other"]):
            with self.assertRaisesRegex(runner.RunnerError, "are absent from the WASM list"):
                runner.selection_for_listed_files(
                    ["web/src/presentation/objects/matrix_transform_inspector.rs"], {}, self.dir)

    def test_generator_harness_builds_once_serves_allowlisted_assets_and_cleans_up(self):
        calls = []

        def build(command, **_kwargs):
            calls.append(command)
            assets = Path(command[-1])
            (assets / "layout-generators/src").mkdir(parents=True)
            (assets / "layout-generators/generated").mkdir(parents=True)
            (assets / "layout-generators/src/index.js").write_text("export const test = true;")
            (assets / "layout-generators/generated/catalogue.mjs").write_text("export const catalogue = [];")
            return type("Result", (), {"returncode": 0, "stdout": "", "stderr": ""})()

        source_env = {"KEEP": "unchanged"}
        with patch.object(runner.subprocess, "run", side_effect=build):
            with runner.packaged_generator_harness(self.dir, source_env) as env:
                module_url = env[runner.GENERATOR_MODULE_URL_ENV]
                with urlopen(module_url) as response:
                    self.assertEqual(response.headers["Access-Control-Allow-Origin"], "*")
                    self.assertEqual(response.read(), b"export const test = true;")
                with self.assertRaises(HTTPError) as response:
                    urlopen(module_url.replace("src/index.js", "../../etc/passwd"))
                self.assertEqual(response.exception.code, 404)
                response.exception.close()
            self.assertEqual(len(calls), 1)
            self.assertEqual(source_env, {"KEEP": "unchanged"})
            with self.assertRaises(URLError):
                urlopen(module_url, timeout=1)

    def test_generator_harness_is_selected_only_for_relevant_sources(self):
        self.assertTrue(runner.selected_generator_sources(["web/src/bundled_models.rs"], self.dir))
        self.assertTrue(runner.selected_generator_sources(
            ["web/src/presentation/parts/catalogue.rs"], self.dir))
        self.assertFalse(runner.selected_generator_sources(
            ["web/src/presentation/layout_camera.rs"], self.dir))
        self.assertTrue(runner.selected_generator_sources(
            ["web/src/presentation/objects/matrix_transform_inspector.rs"], self.dir),
            "the reviewed mounted controller owner loads the packaged catalogue asynchronously")

    def test_desktop_webdriver_config_is_wide_and_preserves_explicit_override(self):
        with runner.desktop_webdriver_config({}) as env:
            config_path = Path(env[runner.WEBDRIVER_CONFIG_ENV])
            self.assertEqual(json.loads(config_path.read_text()), {
                "goog:chromeOptions": {"args": ["--window-size=1280,900"]}
            })
        self.assertFalse(config_path.exists())
        supplied = self.dir / "caller-webdriver.json"
        supplied.write_text('{"goog:chromeOptions":{"args":["--window-size=1440,1000"]}}')
        with runner.desktop_webdriver_config({runner.WEBDRIVER_CONFIG_ENV: str(supplied)}) as env:
            self.assertEqual(env[runner.WEBDRIVER_CONFIG_ENV], str(supplied))
        crate_config = self.dir / "web/webdriver.json"
        crate_config.parent.mkdir(parents=True, exist_ok=True)
        crate_config.write_text('{"goog:chromeOptions":{"args":["--window-size=1360,900"]}}')
        with runner.desktop_webdriver_config({}, self.dir) as env:
            self.assertEqual(env[runner.WEBDRIVER_CONFIG_ENV], str(crate_config.resolve()))

    def test_main_keeps_one_generator_url_and_webdriver_config_through_listing_and_runs(self):
        build_calls, seen_urls, seen_configs = [], [], []

        def build(command, **_kwargs):
            build_calls.append(command)
            assets = Path(command[-1])
            (assets / "layout-generators/src").mkdir(parents=True)
            (assets / "layout-generators/generated").mkdir(parents=True)
            (assets / "layout-generators/src/index.js").write_text("export const test = true;")
            (assets / "layout-generators/generated/catalogue.mjs").write_text("export const catalogue = [];")
            return type("Result", (), {"returncode": 0, "stdout": "", "stderr": ""})()

        def list_tests(env, _root):
            seen_urls.append(env[runner.GENERATOR_MODULE_URL_ENV])
            seen_configs.append(env[runner.WEBDRIVER_CONFIG_ENV])
            self.assertTrue(Path(seen_configs[-1]).exists())
            return ["a::m1::t1", "b::m2::t2"]

        def run_filter(filter_, env, _root):
            seen_urls.append(env[runner.GENERATOR_MODULE_URL_ENV])
            seen_configs.append(env[runner.WEBDRIVER_CONFIG_ENV])
            name = "a::m1::t1" if filter_ == "a::m1::" else "b::m2::t2"
            return 0, {name: "ok"}, f"test {name} ... ok"

        with (
            patch.dict(os.environ, {runner.RUNNER_ENV: ""}),
            patch.object(runner, "runner_environment", return_value={"PATH": os.environ.get("PATH", "")}),
            patch.object(runner, "list_wasm_tests", side_effect=list_tests),
            patch.object(runner, "run_filter", side_effect=run_filter),
            patch.object(runner.subprocess, "run", side_effect=build),
        ):
            code = runner.main(["--all", "--root", str(self.dir)])

        self.assertEqual(code, 0)
        self.assertEqual(len(build_calls), 1)
        self.assertEqual(len(set(seen_urls)), 1)
        self.assertEqual(len(set(seen_configs)), 1)
        self.assertFalse(Path(seen_configs[0]).exists())


if __name__ == "__main__":
    unittest.main(verbosity=2)
