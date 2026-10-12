#!/usr/bin/env python3
from __future__ import annotations

import contextlib
import importlib.util
import io
from pathlib import Path
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location("check_core_tests", Path(__file__).with_name("check-core-tests.py"))
check = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(check)

CARGO = """\
[package]
name = "boardstudio_core"
autotests = false

[[test]]
name = "integration"
path = "tests/integration.rs"

[[test]]
name = "performance"
path = "tests/performance.rs"
"""


class CoreTestRegistrationTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.root = Path(self.directory.name)
        self.addCleanup(self.directory.cleanup)
        self.write("core/Cargo.toml", CARGO)
        self.write("core/tests/integration.rs", '#[path = "matrix_companions.rs"]\nmod matrix_companions;\n')
        self.write("core/tests/matrix_companions.rs", "")
        self.write("core/tests/performance.rs", "")

    def write(self, name: str, text: str):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")

    def test_accepts_files_registered_as_targets_or_path_modules(self):
        self.assertEqual(check.unregistered_tests(self.root), [])

    def test_reports_a_file_that_no_target_compiles(self):
        self.write("core/tests/splay_companions.rs", "")
        issues = check.unregistered_tests(self.root)
        self.assertEqual(len(issues), 1)
        self.assertIn("core/tests/splay_companions.rs", issues[0])
        self.assertIn("integration.rs", issues[0])

    def test_ignores_support_directories_and_non_rust_files(self):
        self.write("core/tests/support/boundary.rs", "")
        self.write("core/tests/fixtures/data.json", "{}")
        self.assertEqual(check.unregistered_tests(self.root), [])

    def test_a_path_attribute_naming_another_file_does_not_register_this_one(self):
        self.write("core/tests/matrix_companions_extra.rs", "")
        self.assertEqual(len(check.unregistered_tests(self.root)), 1)

    def test_main_exits_one_with_each_issue_and_zero_when_clean(self):
        errors = io.StringIO()
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(check.main([str(self.root)]), 0)
        self.write("core/tests/orphan.rs", "")
        with contextlib.redirect_stderr(errors), contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(check.main([str(self.root)]), 1)
        self.assertIn("orphan.rs", errors.getvalue())


if __name__ == "__main__":
    unittest.main()
