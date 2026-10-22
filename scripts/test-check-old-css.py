#!/usr/bin/env python3
from __future__ import annotations

import contextlib
import importlib.util
import io
from pathlib import Path
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location("check_old_css", Path(__file__).with_name("check-old-css.py"))
check = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(check)


class OldCssTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.root = Path(self.directory.name)
        self.addCleanup(self.directory.cleanup)

    def write(self, name: str, text: str):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")

    def run_main(self, *arguments: str) -> tuple[int, str, str]:
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = check.main([*arguments, str(self.root)])
        return code, out.getvalue(), err.getvalue()

    def test_distinct_selectors_and_variables_are_counted_across_web_files(self):
        self.write("web/assets/m1.css", ".m1-a { color: var(--wb-x); } .m1-a:hover { } .board-b {}")
        self.write("web/src/view.rs", 'let class = "m1-a"; let s = ".board-c"; let v = "--wb-y";')
        selectors, variables = check.names(self.root)
        self.assertEqual(selectors, {".m1-a", ".board-b", ".board-c"})
        self.assertEqual(variables, {"--wb-x", "--wb-y"})

    def test_build_output_and_files_outside_web_are_not_counted(self):
        self.write("web/crates/app/target/debug/out.css", ".m1-built {}")
        self.write("web/pkg/glue.css", ".board-built {}")
        self.write("docs/note.css", ".m1-doc { --wb-doc: 1; }")
        self.assertEqual(check.names(self.root), (set(), set()))

    def test_default_run_reports_counts_and_passes(self):
        self.write("web/assets/m1.css", ".m1-a { --wb-x: 1; }")
        code, out, err = self.run_main()
        self.assertEqual(code, 0)
        self.assertIn("1 .board-*/.m1-* selectors and 1 --wb-* variables", out)
        self.assertEqual(err, "")

    def test_strict_run_fails_while_any_name_remains(self):
        self.write("web/assets/m1.css", ".m1-a {}")
        code, _, err = self.run_main("--strict")
        self.assertEqual(code, 1)
        self.assertIn("strict mode fails", err)

    def test_strict_run_passes_when_none_remain(self):
        self.write("web/src/view.rs", "fn main() {}")
        code, out, _ = self.run_main("--strict")
        self.assertEqual(code, 0)
        self.assertIn("0 .board-*/.m1-* selectors and 0 --wb-* variables", out)


if __name__ == "__main__":
    unittest.main()
