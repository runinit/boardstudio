#!/usr/bin/env python3
from __future__ import annotations

import contextlib
import importlib.util
import io
import os
from pathlib import Path
import sys
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("check", Path(__file__).with_name("check.py"))
check = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(check)


class CheckTests(unittest.TestCase):
    def test_every_referenced_script_and_manifest_exists(self):
        for name, (_, commands) in check.STEPS.items():
            for command in commands:
                for argument in command:
                    if argument.endswith(".py") or argument.endswith("Cargo.toml"):
                        self.assertTrue((ROOT / argument).is_file(), f"{name}: {argument} is missing")

    def test_full_check_runs_the_documented_steps_in_order(self):
        self.assertEqual(check.DEFAULT, ("repo", "tooling", "lint", "build", "test", "browser"))
        self.assertNotIn("security", check.DEFAULT)
        self.assertNotIn("typecheck", check.DEFAULT)

    def test_unknown_steps_are_rejected(self):
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit) as raised:
            check.main(["nonsense"])
        self.assertEqual(raised.exception.code, 2)

    def test_a_failing_command_stops_the_run_and_names_the_step(self):
        # `repo` has two commands: the second fails, so `tooling` must never start.
        results = iter([mock.Mock(returncode=0), mock.Mock(returncode=3)])
        with mock.patch.object(check.subprocess, "run", side_effect=lambda *a, **k: next(results)) as run, \
                contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()) as errors:
            self.assertEqual(check.run(["repo", "tooling"]), 1)
        self.assertEqual(run.call_count, 2)
        self.assertIn("step 'repo' failed", errors.getvalue())

    def test_native_checks_do_not_inherit_parent_appimage_directory(self):
        # System KiCad loads its libraries from APPDIR when an editor leaks it.
        probe = [sys.executable, "-c",
                 "import os, sys; sys.exit(0 if 'APPDIR' not in os.environ "
                 "and os.environ.get('APPIMAGE') == 'editor.AppImage' else 17)"]
        with mock.patch.dict(os.environ, {"APPDIR": "/tmp/editor-appimage", "APPIMAGE": "editor.AppImage"}), \
                mock.patch.dict(check.STEPS, {"test": (True, [probe])}), \
                contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(check.run(["test"]), 0)
            self.assertEqual(os.environ["APPDIR"], "/tmp/editor-appimage")

    def test_browser_checks_supply_a_bounded_timeout_without_changing_other_steps(self):
        probe = [sys.executable, "-c",
                 "import os, sys; actual = os.environ.get('WASM_BINDGEN_TEST_TIMEOUT', '<unset>'); "
                 "sys.exit(0 if actual == sys.argv[1] else "
                 "f'browser timeout was {actual}, expected {sys.argv[1]}')"]
        with mock.patch.dict(os.environ), \
                mock.patch.dict(check.STEPS, {
                    "browser": (True, [[*probe, "120"], [*probe, "120"]]),
                    "tooling": (True, [[*probe, "<unset>"]]),
                }), \
                contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            os.environ.pop("WASM_BINDGEN_TEST_TIMEOUT", None)
            self.assertEqual(check.run(["browser", "tooling"]), 0)
            self.assertNotIn("WASM_BINDGEN_TEST_TIMEOUT", os.environ)

    def test_browser_checks_preserve_an_explicit_timeout(self):
        probe = [sys.executable, "-c",
                 "import os, sys; sys.exit(0 if os.environ.get('WASM_BINDGEN_TEST_TIMEOUT') "
                 "== '37' else 17)"]
        with mock.patch.dict(os.environ, {"WASM_BINDGEN_TEST_TIMEOUT": "37"}), \
                mock.patch.dict(check.STEPS, {"browser": (True, [probe, probe])}), \
                contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(check.run(["browser"]), 0)
            self.assertEqual(os.environ["WASM_BINDGEN_TEST_TIMEOUT"], "37")

    def test_list_prints_every_step_without_running_anything(self):
        output = io.StringIO()
        with mock.patch.object(check.subprocess, "run") as run, contextlib.redirect_stdout(output):
            self.assertEqual(check.main(["--list"]), 0)
        run.assert_not_called()
        for name in check.STEPS:
            self.assertIn(name, output.getvalue())


if __name__ == "__main__":
    unittest.main()
