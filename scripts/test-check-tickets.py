#!/usr/bin/env python3
from __future__ import annotations

import contextlib
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location("check_tickets", Path(__file__).with_name("check-tickets.py"))
check = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(check)

ISSUES = "docs/plans/typed-core-edits/issues"
MAP = "docs/plans/typed-core-edits/map.md"


class TicketTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.root = Path(self.directory.name)
        self.addCleanup(self.directory.cleanup)
        self.write(MAP, "")
        self.baseline = {}

    def write(self, name: str, text: str):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")

    def ticket(self, name: str, status: str | None, listed: bool = True):
        header = f"# 1: Title\n\n{'Status: ' + status if status is not None else 'Type: build'}\nType: build\n"
        self.write(f"{ISSUES}/{name}", header)
        if listed:
            current = (self.root / MAP).read_text(encoding="utf-8")
            self.write(MAP, current + f"[x](issues/{name})\n")

    def violations(self):
        return check.violations(self.root)

    def test_a_listed_ticket_with_a_known_status_is_clean(self):
        for status in check.STATUSES:
            self.ticket(f"{status}.md", status)
        self.assertEqual(self.violations(), {})

    def test_trailing_whitespace_after_the_status_is_accepted(self):
        self.ticket("a.md", "ready-for-agent  ")
        self.assertEqual(self.violations(), {})

    def test_an_unknown_status_is_reported_with_the_allowed_values(self):
        self.ticket("a.md", "needs-design")
        self.assertEqual(self.violations(), {"a.md": {check.STATUS}})
        problems = check.problems(self.root, {})
        self.assertIn("needs-design", problems[0])
        self.assertIn("ready-for-human", problems[0])

    def test_a_missing_status_line_is_reported(self):
        self.ticket("a.md", None)
        self.assertEqual(self.violations(), {"a.md": {"status"}})

    def test_a_ticket_the_map_does_not_name_is_reported(self):
        self.ticket("a.md", "resolved", listed=False)
        self.assertEqual(self.violations(), {"a.md": {"listed"}})

    def test_a_longer_file_name_does_not_list_a_shorter_one(self):
        self.ticket("19-x.md", "resolved")
        self.write(f"{ISSUES}/9-x.md", "# 9: T\n\nStatus: resolved\n")
        self.assertEqual(self.violations(), {"9-x.md": {"listed"}})

    def test_a_status_in_the_body_does_not_count(self):
        self.write(f"{ISSUES}/a.md", "# 1: T\n" + "\n" * 20 + "Status: resolved\n")
        self.write(MAP, "issues/a.md\n")
        self.assertEqual(self.violations(), {"a.md": {"status"}})

    def test_baselined_violations_pass_and_new_ones_fail(self):
        self.ticket("old.md", "implemented (awaiting verification)", listed=False)
        self.ticket("new.md", "needs-design")
        baseline = {"old.md": ["listed", "status"]}
        problems = check.problems(self.root, baseline)
        self.assertEqual(len(problems), 1)
        self.assertIn("new.md", problems[0])

    def test_a_baseline_entry_that_no_longer_applies_is_reported_as_stale(self):
        self.ticket("a.md", "resolved")
        problems = check.problems(self.root, {"a.md": ["status"], "gone.md": ["listed"]})
        text = "\n".join(problems)
        self.assertIn("stale-baseline", text)
        self.assertIn("a.md", text)
        self.assertIn("gone.md", text)

    def test_main_reads_the_baseline_file_and_sets_the_exit_code(self):
        self.ticket("a.md", "needs-design")
        baseline = self.root / "scripts/check-tickets-baseline.json"
        self.write("scripts/check-tickets-baseline.json", "{}")
        with contextlib.redirect_stderr(io.StringIO()), contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(check.main([str(self.root)]), 1)
            baseline.write_text(json.dumps({"a.md": ["status"]}), encoding="utf-8")
            self.assertEqual(check.main([str(self.root)]), 0)

    def test_update_baseline_records_current_violations(self):
        self.ticket("a.md", "needs-design", listed=False)
        self.write("scripts/check-tickets-baseline.json", "{}")
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(check.main(["--update-baseline", str(self.root)]), 0)
        recorded = json.loads((self.root / "scripts/check-tickets-baseline.json").read_text(encoding="utf-8"))
        self.assertEqual(recorded, {"a.md": ["listed", "status"]})


if __name__ == "__main__":
    unittest.main()
