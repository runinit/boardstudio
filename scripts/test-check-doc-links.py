#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest import mock

SPEC = importlib.util.spec_from_file_location("check_doc_links", Path(__file__).with_name("check-doc-links.py"))
check = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(check)


class DocLinkTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.root = Path(self.directory.name)
        (self.root / "docs/reference").mkdir(parents=True)
        self.addCleanup(self.directory.cleanup)
        subprocess.run(["git", "init", "-q", str(self.root)], check=True)

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

    def test_rejects_existing_absolute_local_paths_including_inside_checkout(self):
        self.write("docs/reference/present.md", "ok\n")
        with tempfile.TemporaryDirectory() as outside:
            target = Path(outside) / "present.md"
            target.write_text("outside\n")
            self.write("README.md", f"[outside]({target}) [inside]({self.root}/docs/reference/present.md) [file]({target.as_uri()}) [windows](C:/local/present.md)\n")
            issues = check.broken_links(self.root)
        self.assertEqual(len(issues), 4)
        self.assertTrue(all("absolute" in issue for issue in issues))

    def test_rejects_relative_and_encoded_lexical_escapes(self):
        with tempfile.TemporaryDirectory() as outside:
            target = Path(outside) / "present.md"
            target.write_text("outside\n")
            relative = f"../../{target.parent.name}/present.md"
            self.write("docs/index.md", f"[outside]({relative}) [encoded]({relative.replace('..', '%2e%2e')})\n")
            issues = check.broken_links(self.root)
        self.assertEqual(len(issues), 2)
        self.assertTrue(all("outside checkout" in issue for issue in issues))

    def test_rejects_symlink_escape_but_accepts_internal_symlinks(self):
        with tempfile.TemporaryDirectory() as outside:
            (Path(outside) / "present.md").write_text("outside\n")
            (self.root / "docs/external").symlink_to(outside, target_is_directory=True)
            self.write("docs/reference/present.md", "ok\n")
            (self.root / "docs/internal").symlink_to("reference", target_is_directory=True)
            self.write("README.md", "[bad](docs/external/present.md) [ok](docs/internal/present.md)\n")
            issues = check.broken_links(self.root)
        self.assertEqual(len(issues), 1)
        self.assertIn("outside checkout", issues[0])

    def test_rejects_ignored_evidence_even_when_present_and_via_symlink(self):
        self.write(".gitignore", ".evidence/\n")
        self.write(".evidence/present.log", "private\n")
        (self.root / "docs/receipt.log").symlink_to("../.evidence/present.log")
        self.write("README.md", "[log](.evidence/present.log) [dir](.evidence/) [alias](docs/receipt.log)\n")
        present = check.broken_links(self.root)
        self.assertEqual(len(present), 3)
        self.assertTrue(all("ignored" in issue for issue in present))
        (self.root / ".evidence/present.log").unlink()
        self.assertEqual(check.broken_links(self.root), present)

    def test_batches_ignore_lookup_and_accepts_tracked_docs_with_ignore_rules(self):
        self.write(".gitignore", "*.md\n")
        self.write("docs/reference/a b.md", "ok\n")
        self.write("README.md", '[space](docs/reference/a%20b.md) [directory](docs/reference/)\n')
        subprocess.run(["git", "-C", str(self.root), "add", "-f", "README.md", "docs/reference/a b.md"], check=True)
        with mock.patch.object(subprocess, "run", wraps=subprocess.run) as run:
            self.assertEqual(check.broken_links(self.root), [])
        self.assertEqual(run.call_count, 1)

    def test_rejects_ignored_symlink_to_tracked_document(self):
        self.write(".gitignore", "private-link\n")
        self.write("docs/reference/present.md", "ok\n")
        subprocess.run(["git", "-C", str(self.root), "add", "docs/reference/present.md"], check=True)
        (self.root / "private-link").symlink_to("docs/reference", target_is_directory=True)
        self.write("README.md", "[private](private-link/present.md)\n")
        issues = check.broken_links(self.root)
        self.assertEqual(len(issues), 1)
        self.assertIn("ignored", issues[0])

    def test_temporary_doc_root_uses_git_objects_without_local_evidence(self):
        self.write(".gitignore", ".evidence/\n")
        self.write("README.md", "[docs](docs/reference/) [page](docs/reference/a%20b.md#part)\n")
        self.write("docs/reference/a b.md", "# Part\n")
        subprocess.run(["git", "-C", str(self.root), "add", "."], check=True)
        tree = subprocess.check_output(["git", "-C", str(self.root), "write-tree"], text=True).strip()
        self.write("README.md", "[local](.evidence/not-in-git.log)\n")
        stored = subprocess.check_output(["git", "-C", str(self.root), "show", f"{tree}:README.md"], text=True)
        self.write("README.md", stored)
        self.assertFalse((self.root / ".evidence").exists())
        self.assertEqual(check.broken_links(self.root), [])


if __name__ == "__main__":
    unittest.main()
