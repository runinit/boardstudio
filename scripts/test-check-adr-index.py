#!/usr/bin/env python3
from __future__ import annotations

import contextlib
import importlib.util
import io
from pathlib import Path
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location("check_adr_index", Path(__file__).with_name("check-adr-index.py"))
check = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(check)


class AdrIndexTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.root = Path(self.directory.name)
        self.addCleanup(self.directory.cleanup)
        self.write("docs/adr/0001-first.md", "# First\n")
        self.write("docs/adr/0002-second.md", "# Second\n")
        self.write("docs/adr/README.md", "| [0001](0001-first.md) | a |\n| [0002](0002-second.md) | b |\n")

    def write(self, name: str, text: str):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")

    def test_every_listed_adr_passes(self):
        self.assertEqual(check.unlisted(self.root), [])

    def test_an_adr_missing_from_the_index_is_reported_by_name(self):
        self.write("docs/adr/0003-third.md", "# Third\n")
        issues = check.unlisted(self.root)
        self.assertEqual(len(issues), 1)
        self.assertIn("0003-third.md", issues[0])

    def test_the_index_itself_is_not_an_adr(self):
        self.assertEqual(check.unlisted(self.root), [])

    def test_a_missing_index_reports_every_adr(self):
        (self.root / "docs/adr/README.md").unlink()
        self.assertEqual(len(check.unlisted(self.root)), 2)

    def test_main_exits_one_for_an_unlisted_adr_and_zero_when_clean(self):
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(check.main([str(self.root)]), 0)
        self.write("docs/adr/0003-third.md", "# Third\n")
        errors = io.StringIO()
        with contextlib.redirect_stderr(errors), contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(check.main([str(self.root)]), 1)
        self.assertIn("0003-third.md", errors.getvalue())


if __name__ == "__main__":
    unittest.main()
