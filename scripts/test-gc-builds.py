#!/usr/bin/env python3
"""Tests for scripts/gc-builds.py using temporary directories only."""
import contextlib
import io
import json
import os
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest import TestCase, main

from importlib.util import module_from_spec, spec_from_file_location

SPEC = spec_from_file_location("gc_builds", Path(__file__).with_name("gc-builds.py"))
GC = module_from_spec(SPEC)
SPEC.loader.exec_module(GC)


class GcBuildsTests(TestCase):
    def setUp(self):
        self.tmp = TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.repo = Path(self.tmp.name)
        self.root = self.repo / "web/target/builds"
        self.root.mkdir(parents=True)
        self.evidence = self.repo / ".scratch/dioxus-frontend-v1/evidence"
        self.evidence.mkdir(parents=True)
        self.run = self.repo / "run.json"
        self.clock = 1_000_000
        self.write_run()

    def write_run(self, served=None, scope=None, accepted=None):
        progress = {"served_candidate": {"build_id": served} if served else {},
                    "qualification": {"active_scope": scope or {}},
                    "parent_acceptances": accepted or []}
        self.run.write_text(json.dumps({"current_progress": progress}))

    def build(self, name, base=None, status="complete", provenance=True):
        path = self.root / name
        path.mkdir()
        (path / "payload").write_bytes(b"x" * 100)
        if provenance:
            data = {"source_commit": "a" * 40, "status": status, "commands": []}
            if base:
                data["base_build"] = str(self.root / base)
            (path / "provenance.json").write_text(json.dumps(data))
        self.clock += 100
        os.utime(path, (self.clock, self.clock))
        return path

    def run_gc(self, *args):
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            code = GC.main(list(args), root=self.root, run_path=self.run,
                           repo=self.repo, evidence_root=self.evidence)
        return code, out.getvalue()

    def names(self):
        return sorted(p.name for p in self.root.iterdir())

    def test_keep_latest(self):
        for n in ("a", "b", "c", "d", "e"):
            self.build(n)
        code, _ = self.run_gc("--apply", "--keep-latest", "2")
        self.assertEqual(code, 0)
        self.assertEqual(self.names(), ["d", "e"])

    def test_served_kept(self):
        for n in ("a", "b", "c"):
            self.build(n)
        self.write_run(served="a")
        self.run_gc("--apply", "--keep-latest", "1")
        self.assertEqual(self.names(), ["a", "c"])

    def test_reuse_baseline_kept_transitively(self):
        self.build("full")
        self.build("mid", base="full")
        for n in ("old1", "old2"):
            self.build(n)
        self.build("new", base="mid")
        self.run_gc("--apply", "--keep-latest", "1")
        self.assertEqual(self.names(), ["full", "mid", "new"])

    def test_accepted_and_active_scope_kept(self):
        for n in ("acc", "scope", "gone", "latest"):
            self.build(n)
        decision = self.repo / "decision.json"
        decision.write_text(json.dumps({"candidate": "acc"}))
        self.write_run(scope={"candidate": "scope"}, accepted=[{"decision_record": "decision.json"}])
        self.run_gc("--apply", "--keep-latest", "1")
        self.assertEqual(self.names(), ["acc", "latest", "scope"])

    def test_partial_flagged_and_listed_separately(self):
        self.build("good")
        self.build("broken", provenance=False)
        self.build("running", status="running")
        code, out = self.run_gc("--keep-latest", "5")
        self.assertIn("PARTIAL", out)
        self.assertEqual(sum(1 for l in out.splitlines() if l.startswith("PARTIAL")), 2)
        self.assertIn("KEEP", out.split("PARTIAL")[0])
        self.assertEqual(len(self.names()), 3)
        self.run_gc("--apply", "--keep-latest", "5")
        self.assertEqual(self.names(), ["good"])

    def test_symlink_refused(self):
        self.build("a")
        outside = self.repo / "outside"
        outside.mkdir()
        (outside / "keep.txt").write_text("precious")
        os.symlink(outside, self.root / "link")
        with self.assertRaises(SystemExit):
            self.run_gc("--apply", "--keep-latest", "1")
        self.assertTrue((outside / "keep.txt").exists())
        self.assertEqual(self.names(), ["a", "link"])

    def test_dry_run_deletes_nothing(self):
        for n in ("a", "b", "c", "d", "e"):
            self.build(n)
        before = self.names()
        code, out = self.run_gc()
        self.assertEqual(code, 0)
        self.assertEqual(self.names(), before)
        self.assertIn("would remove 2", out)
        self.assertIn("summary:", out)

    def test_provenance_preserved_only_into_existing_evidence(self):
        for n in ("a", "b", "c", "d"):
            self.build(n)
        (self.evidence / "a").mkdir()
        self.run_gc("--apply", "--keep-latest", "2")
        self.assertTrue((self.evidence / "a/provenance.json").is_file())
        self.assertFalse((self.evidence / "b").exists())


if __name__ == "__main__":
    main()
