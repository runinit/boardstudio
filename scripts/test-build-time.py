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


if __name__ == "__main__":
    unittest.main()
