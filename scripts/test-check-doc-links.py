#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
from pathlib import Path
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location("check_doc_links", Path(__file__).with_name("check-doc-links.py"))
check = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(check)


class DocLinkTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.root = Path(self.directory.name)
        (self.root / "docs/reference").mkdir(parents=True)
        self.addCleanup(self.directory.cleanup)

    def write(self, name: str, text: str):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")

    def test_reports_missing_local_paths_and_accepts_directories(self):
        self.write("README.md", "[docs](./docs/) [missing](./docs/nope.md)\n")
        issues = check.broken_links(self.root)
        self.assertEqual(len(issues), 1)
        self.assertIn("nope.md", issues[0])

    def test_ignores_remote_links_anchors_and_code_fences(self):
        self.write("README.md", "[web](https://example.com/x) [mail](mailto:a@b.c) [top](#top)\n```\n[gone](./gone.md)\n```\n")
        self.assertEqual(check.broken_links(self.root), [])

    def test_resolves_relative_to_the_file_and_decodes_names_and_anchors(self):
        self.write("docs/reference/a b.md", "x\n")
        self.write("docs/index.md", '[a](<reference/a b.md#part>) [b](reference/a%20b.md "title") [up](../README.md)\n')
        self.write("README.md", "ok\n")
        self.assertEqual(check.broken_links(self.root), [])

    def test_checks_nested_docs_and_skips_excluded_directories(self):
        self.write("docs/reference/page.md", "[bad](../missing.md)\n")
        self.write("docs/library/vendor.md", "[bad](../missing.md)\n")
        issues = check.broken_links(self.root)
        self.assertEqual(len(issues), 1)
        self.assertIn("docs/reference/page.md", issues[0])


if __name__ == "__main__":
    unittest.main()
