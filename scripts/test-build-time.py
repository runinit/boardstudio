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


if __name__ == "__main__":
    unittest.main()
