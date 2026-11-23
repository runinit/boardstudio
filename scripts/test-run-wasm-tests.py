#!/usr/bin/env python3
"""Exercise the page runner against a deterministic runner process."""

import importlib
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

RUNNER = importlib.import_module("run-wasm-tests")


class PageRunnerTests(unittest.TestCase):
    def run_fixture(self, mode, *selection):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            report = root / "results.json"
            fake = root / "runner.py"
            fake.write_text("""
import os
import sys

names = ["another::test_c", "presentation::a::test_a", "presentation::b::test_b"]
mode = os.environ["MODE"]
if sys.argv[1:] == ["--list"]:
    for name in names + (names[:1] if mode == "duplicate-list" else []):
        print(name + ": test")
else:
    prefix, *skip = sys.argv[1:]
    for name in names:
        if prefix not in name or any(excluded in name for excluded in skip[1::2]):
            continue
        print("Invoking test:", name)
        if mode == "incomplete":
            continue
        print(f"test {name} ... ok")
        if mode == "duplicate":
            print(f"test {name} ... ok")
    if mode == "bad-exit":
        sys.exit(2)
""")
            env = {RUNNER.RUNNER_ENV: f"{sys.executable} {fake}", "MODE": mode}
            with patch.dict(os.environ, env):
                code = RUNNER.main(["--root", tmp, "--result-json", str(report), *selection])
            return code, json.loads(report.read_text())

    def test_all_runs_each_listed_name_once_with_isolate(self):
        code, report = self.run_fixture("pass", "--all", "--depth", "1", "--isolate", "presentation::a::")
        self.assertEqual(code, 0)
        self.assertTrue(report["complete"])
        self.assertEqual(report["listed_tests"], report["expected_tests"])
        self.assertEqual(report["terminal_outcomes"], [
            {"test": name, "status": "passed"} for name in report["listed_tests"]
        ])

    def test_incomplete_duplicate_or_crashed_runs_fail(self):
        for mode in ("incomplete", "duplicate", "duplicate-list", "bad-exit"):
            with self.subTest(mode=mode):
                code, report = self.run_fixture(mode, "--all", "--depth", "1")
                self.assertNotEqual(code, 0)
                self.assertFalse(report["complete"])
                self.assertTrue(report["problems"])

    def test_focused_source_selects_its_module_and_fails_when_unmapped(self):
        code, report = self.run_fixture("pass", "--files", "web/src/presentation/a.rs")
        self.assertEqual(code, 0)
        self.assertEqual(report["expected_tests"], ["presentation::a::test_a"])
        code, report = self.run_fixture("pass", "--files", "web/src/presentation/absent.rs")
        self.assertNotEqual(code, 0)
        self.assertIn("no listed WASM tests", report["problems"][0])

    def test_list_cache_uses_wasm_digest_and_custom_runner_bypasses_it(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            artifact = root / "page.wasm"
            artifact.write_bytes(b"one")
            env = {RUNNER.PAGE_ARTIFACT: str(artifact)}
            with patch.object(RUNNER, "list_wasm_tests", side_effect=[["a"], ["b"], ["custom"]]) as listing:
                self.assertEqual(RUNNER.cached_wasm_tests(env, root, False), ["a"])
                self.assertEqual(RUNNER.cached_wasm_tests(env, root, False), ["a"])
                artifact.write_bytes(b"two")
                self.assertEqual(RUNNER.cached_wasm_tests(env, root, False), ["b"])
                self.assertEqual(RUNNER.cached_wasm_tests(env, root, True), ["custom"])
                self.assertEqual(listing.call_count, 3)

    def test_build_only_generates_css_before_cargo(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "web").mkdir()
            (root / "web/tailwind.css").touch()
            artifact = root / "page.wasm"
            artifact.touch()
            cargo_output = json.dumps({
                "reason": "compiler-artifact", "manifest_path": str(root / "web/Cargo.toml"),
                "target": {"kind": ["bin"], "name": "boardstudio-web"},
                "profile": {"test": True}, "executable": str(artifact),
            })
            with patch.object(RUNNER, "test_css_command", return_value=["tailwind"]), \
                 patch.object(RUNNER.subprocess, "run") as run:
                run.return_value.stdout = cargo_output
                run.return_value.returncode = 0
                self.assertEqual(RUNNER.build_page_test(root, {}), artifact)
            self.assertEqual([call.args[0] for call in run.call_args_list], [["tailwind"], RUNNER.PAGE_BUILD])
            self.assertEqual(run.call_args.kwargs["env"]["CARGO_TARGET_DIR"], str(root / "target/wasm-tests"))


if __name__ == "__main__":
    unittest.main()
