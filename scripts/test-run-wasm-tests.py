#!/usr/bin/env python3
"""Tests for run-wasm-tests.py using a fake runner (no Chrome, no cargo)."""

import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import tempfile
import unittest

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
        self.assertEqual(self.log.read_text().strip(), "presentation::layout_camera::")

    def test_real_failure_fails_the_run_and_is_named(self):
        self.fake("echo 'test presentation::x::tests::good ... ok'; echo 'test presentation::x::tests::bad ... FAIL'; exit 1")
        code, out, _ = self.main("--files", "web/src/presentation/x.rs")
        self.assertEqual(code, 1)
        self.assertIn("executed 2, failed 1", out)
        self.assertIn("FAILED presentation::x::tests::bad", out)

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

    def test_zero_executed_widens_to_parent_module_before_failing(self):
        self.fake('case "$1" in presentation::a::b::c::) ;; presentation::a::b::) echo "test presentation::a::b::sibling::t ... ok";; esac')
        code, out, err = self.main("--files", "web/src/presentation/a/b/c.rs")
        self.assertEqual(code, 0, err)
        self.assertEqual(self.log.read_text().split(), ["presentation::a::b::c::", "presentation::a::b::"])

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
            'echo "test ${1}t ... ok"'
        )
        code, out, _ = self.main("--all")
        self.assertEqual(code, 0, out)
        calls = self.log.read_text().split("\n")
        self.assertIn("--list", calls[0])
        self.assertEqual(sorted(c for c in calls[1:] if c), ["a::m1::", "b::m2::"])
        self.assertIn("executed 2", out)


if __name__ == "__main__":
    unittest.main(verbosity=2)
