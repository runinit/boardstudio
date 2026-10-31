#!/usr/bin/env python3
"""Focused tests for the cold-build measurement kit."""

import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parent.parent
KIT = ROOT / "scripts" / "build-time"


def load(name):
    spec = importlib.util.spec_from_file_location(name, KIT / f"{name}.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


COLD = load("cold")
REPORT = load("report")

_seed_spec = importlib.util.spec_from_file_location("seed", ROOT / "scripts" / "seed-worktree-target.py")
SEED = importlib.util.module_from_spec(_seed_spec)
_seed_spec.loader.exec_module(SEED)


class ColdRefusalTests(unittest.TestCase):
    def test_refuses_when_cargo_target_dir_is_set(self):
        reasons = COLD.refusal_reasons({"CARGO_TARGET_DIR": "elsewhere"}, lambda p: False, 0.5)
        self.assertTrue(any("CARGO_TARGET_DIR" in r for r in reasons))

    def test_refuses_when_a_target_path_is_a_symlink(self):
        reasons = COLD.refusal_reasons({}, lambda p: p == "target", 0.5)
        self.assertTrue(any("symlink" in r for r in reasons))

    def test_refuses_under_load(self):
        reasons = COLD.refusal_reasons({}, lambda p: False, 3.0)
        self.assertTrue(any("load" in r for r in reasons))

    def test_clean_setup_has_no_refusal(self):
        self.assertEqual(COLD.refusal_reasons({}, lambda p: False, 1.0), [])


class AttributionTests(unittest.TestCase):
    def records(self):
        return [
            {"crate": "a", "start": 0.0, "secs": 10.0},
            {"crate": "b", "start": 5.0, "secs": 10.0},
        ]

    def test_attributed_wall_splits_overlap_equally(self):
        attrib = REPORT.attributed_wall(self.records(), {"a", "b"})
        # 5s solo (a) + 5s shared (split, both selected) + 5s solo (b) = 15s of activity.
        self.assertAlmostEqual(attrib, 15.0)

    def test_attributed_wall_for_one_crate_in_overlap(self):
        attrib = REPORT.attributed_wall(self.records(), {"a"})
        # 5s solo + half of the 5s overlap.
        self.assertAlmostEqual(attrib, 7.5)

    def test_attribution_of_everything_equals_active_union(self):
        attrib = REPORT.attributed_wall(self.records(), {"a", "b"})
        self.assertAlmostEqual(attrib, REPORT.active_union(self.records()))


class RealCpuTests(unittest.TestCase):
    def test_wrapper_records_child_cpu_and_report_totals_it(self):
        import json, os, subprocess, sys, tempfile
        code = "sum(range(10**7))"
        with tempfile.TemporaryDirectory() as tmp:
            log = Path(tmp) / "units.jsonl"
            env = dict(os.environ, BUILDTIME_LOG=str(log))
            subprocess.run([sys.executable, "-B", str(KIT / "rustc-log.py"), sys.executable, "-c", code,
                            "--crate-name", "burner"], env=env, check=True)
            record = json.loads(log.read_text())
        self.assertGreater(record["cpu"], 0.05)
        self.assertAlmostEqual(REPORT.real_cpu([record]), record["cpu"])
        self.assertIsNone(REPORT.real_cpu([{"crate": "old", "start": 0.0, "secs": 1.0}]))


class SeedRefusalTests(unittest.TestCase):
    def make_tree(self, base, toolchain="same", targets=()):
        base.mkdir(parents=True, exist_ok=True)
        (base / "rust-toolchain.toml").write_text(toolchain)
        for rel in targets:
            (base / rel).mkdir(parents=True, exist_ok=True)

    def test_refuses_on_toolchain_mismatch_and_existing_target(self):
        import tempfile
        with tempfile.TemporaryDirectory() as tmp:
            donor, dest = Path(tmp) / "donor", Path(tmp) / "dest"
            self.make_tree(donor, "a", SEED.TARGETS)
            self.make_tree(dest, "b", ["target"])
            reasons = SEED.refusals(donor, dest)
            self.assertTrue(any("rust-toolchain" in r for r in reasons))
            self.assertTrue(any("already has" in r for r in reasons))

    def test_matching_empty_destination_has_no_refusal(self):
        import tempfile
        with tempfile.TemporaryDirectory() as tmp:
            donor, dest = Path(tmp) / "donor", Path(tmp) / "dest"
            self.make_tree(donor, "a", SEED.TARGETS)
            self.make_tree(dest, "a")
            self.assertEqual(SEED.refusals(donor, dest), [])


class SeedBehaviourTests(unittest.TestCase):
    make_tree = SeedRefusalTests.make_tree

    def setUp(self):
        import tempfile
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.orig_copy = SEED.copy
        self.addCleanup(setattr, SEED, "copy", self.orig_copy)

    def test_skips_missing_dirs_and_seeds_the_rest(self):
        donor, dest = self.root / "donor", self.root / "dest"
        self.make_tree(donor, "a", ["target", "cad/wasm/target"])
        self.make_tree(dest, "a")
        copied = []
        SEED.copy = lambda src, dst: (copied.append(dst.relative_to(dest).as_posix()), dst.mkdir(parents=True))
        self.assertEqual(SEED.seed(donor, dest), 0)
        self.assertEqual(copied, ["target", "cad/wasm/target"])

    def test_refuses_donor_without_target(self):
        donor, dest = self.root / "donor", self.root / "dest"
        self.make_tree(donor, "a", ["cad/wasm/target"])
        self.make_tree(dest, "a")
        self.assertTrue(any("no target" in r for r in SEED.refusals(donor, dest)))

    def test_failed_copy_removes_partial_destination(self):
        import subprocess
        donor, dest = self.root / "donor", self.root / "dest"
        self.make_tree(donor, "a", ["target", "cad/wasm/target"])
        self.make_tree(dest, "a")

        def fake(src, dst):
            dst.mkdir(parents=True)
            if "wasm" in str(dst):
                (dst / "half").write_text("x")
                raise subprocess.CalledProcessError(1, "cp")
        SEED.copy = fake
        self.assertEqual(SEED.seed(donor, dest), 1)
        self.assertFalse((dest / "target").exists())
        self.assertFalse((dest / "cad/wasm/target").exists())

    def test_locks_are_held_during_copy(self):
        donor, dest = self.root / "donor", self.root / "dest"
        self.make_tree(donor, "a", ["target"])
        self.make_tree(dest, "a")
        seen = []

        def fake(src, dst):
            with SEED.donor_idle(donor, ["target"]) as idle:
                seen.append(idle)
            dst.mkdir()
        SEED.copy = fake
        self.assertEqual(SEED.seed(donor, dest), 0)
        self.assertEqual(seen, [False])
        with SEED.donor_idle(donor, ["target"]) as idle:
            self.assertTrue(idle)

    def test_auto_prefers_most_dirs_then_newest_and_skips_busy(self):
        import os, subprocess
        dest = self.root / "dest"
        self.make_tree(dest, "a")
        subprocess.run(["git", "init", "-q", str(dest)], check=True)
        names = {"few": ["target"], "many_old": ["target", "cad/wasm/target"],
                 "many_new": ["target", "cad/wasm/target"], "wrong": SEED.TARGETS, "none": []}
        for i, (name, dirs) in enumerate(names.items()):
            wt = self.root / name
            self.make_tree(wt, "b" if name == "wrong" else "a", dirs)
            if dirs:
                info = wt / "target" / ".rustc_info.json"
                info.write_text("{}")
                os.utime(info, (1000 + (10 if name == "many_new" else i),) * 2)
        listing = "".join(f"worktree {self.root / n}\n\n" for n in ["dest", *names])
        orig = SEED.subprocess.run
        self.addCleanup(setattr, SEED.subprocess, "run", orig)
        SEED.subprocess.run = lambda *a, **k: subprocess.CompletedProcess(a, 0, stdout=listing)
        self.assertEqual(SEED.choose_donor(dest), self.root / "many_new")
        with SEED.donor_idle(self.root / "many_new", ["target"]):
            self.assertEqual(SEED.choose_donor(dest), self.root / "many_old")


if __name__ == "__main__":
    unittest.main()
