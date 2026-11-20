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
        reasons = COLD.refusal_reasons({}, lambda p: False, 6.5)
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
    def test_report_uses_step_attribution_and_real_cpu_when_commands_overlap(self):
        import contextlib
        import io
        output = "/repo/target/wasm32-unknown-unknown/wasm-check/deps"
        units = [
            {"step": "build", "start": 10.0, "secs": 4.0, "cpu": 2.0,
             "out": output, "target": "wasm32-unknown-unknown"},
            {"step": "test", "start": 10.0, "secs": 6.0, "cpu": 3.0,
             "out": "/repo/target/debug/deps", "target": "host"},
        ]
        data = {"run": (units, {"build": 4.0, "test": 6.0, "total": 6.0},
                        "cold: wall 6.0s, process tree CPU 6.0s")}
        with contextlib.redirect_stdout(io.StringIO()) as result:
            REPORT.compare(["run"], data)
        rows = {line.split()[0]: line.split()[1:] for line in result.getvalue().splitlines() if line.split()}
        self.assertEqual(rows["build"], ["4.0", "4.0", "2"])
        self.assertEqual(rows["test"], ["6.0", "6.0", "3"])
        self.assertEqual(rows["total"], ["6.0", "6.0", "5"])
        self.assertEqual(rows["root/wasm32/wasm-check"], ["2"])

class SccacheStatsTests(unittest.TestCase):
    SAMPLE = """\
Compile requests                      0
Compile requests executed         18181
Cache hits                         9190
Cache hits (Rust)                  9190
Cache misses                       8898
Cache misses (Rust)                8898
Cache hits rate                   50.81 %
Cache timeouts                        0
Cache read errors                     0
Forced recaches                       0
Cache write errors                    0
Cache errors                          0
Compilations                       8898
Compilation failures                 93
Non-cacheable compilations            0
Non-cacheable calls                1454
Cache size                            8 GiB
Max cache size                       30 GiB
"""

    def test_socket_prefers_xdg_runtime_dir(self):
        socket = COLD.sccache_socket({"XDG_RUNTIME_DIR": "/run/user/1000"}, 1000, lambda p: True)
        self.assertEqual(socket, "/run/user/1000/boardstudio-sccache/server.sock")

    def test_socket_defaults_to_run_user_uid(self):
        seen = []
        socket = COLD.sccache_socket({}, 1000, lambda p: seen.append(p) or True)
        self.assertEqual(socket, "/run/user/1000/boardstudio-sccache/server.sock")
        self.assertEqual(seen, [socket])

    def test_socket_falls_back_when_xdg_candidate_missing(self):
        exists = lambda p: p == "/run/user/1000/boardstudio-sccache/server.sock"  # noqa: E731
        socket = COLD.sccache_socket({"XDG_RUNTIME_DIR": "/tmp/gone"}, 1000, exists)
        self.assertEqual(socket, "/run/user/1000/boardstudio-sccache/server.sock")

    def test_socket_is_none_when_daemon_absent(self):
        self.assertIsNone(COLD.sccache_socket({}, 1000, lambda p: False))

    def test_parse_reads_counters_and_ignores_rates_and_sizes(self):
        stats = COLD.parse_sccache_stats(self.SAMPLE)
        self.assertEqual(stats["Compile requests"], 0)
        self.assertEqual(stats["Compile requests executed"], 18181)
        self.assertEqual(stats["Cache hits"], 9190)
        self.assertEqual(stats["Cache misses"], 8898)
        self.assertEqual(stats["Cache errors"], 0)
        self.assertEqual(stats["Compilation failures"], 93)
        self.assertEqual(stats["Non-cacheable calls"], 1454)

    def test_parse_defaults_missing_counters_to_zero(self):
        stats = COLD.parse_sccache_stats("Cache hits                         3\n")
        self.assertEqual(stats["Cache hits"], 3)
        self.assertEqual(stats["Cache misses"], 0)
        self.assertEqual(stats["Non-cacheable calls"], 0)

    def test_delta_math_sums_error_counters(self):
        before = COLD.parse_sccache_stats(self.SAMPLE)
        after = dict(before, **{"Compile requests executed": 18281,
                                "Cache hits": 9290, "Cache misses": 8899,
                                "Cache errors": 1, "Compilation failures": 95,
                                "Non-cacheable calls": 1454})
        line = COLD.format_sccache_delta(before, after)
        self.assertIn("+100 executed", line)
        self.assertIn("+100 hits", line)
        self.assertIn("+1 misses", line)
        self.assertIn("+0 non-cacheable", line)
        self.assertIn("+3 errors", line)

    def test_snapshot_is_fail_open(self):
        self.assertIsNone(COLD.snapshot_sccache("/nonexistent/sccache", "/nonexistent.sock"))
        # Nonzero exit also yields None rather than raising.
        import subprocess
        from unittest import mock
        with mock.patch.object(subprocess, "run") as run:
            run.return_value.returncode = 1
            run.return_value.stdout = ""
            self.assertIsNone(COLD.snapshot_sccache("sccache", "sock"))

    def test_report_sccache_handles_missing_snapshots(self):
        import contextlib
        import io
        import tempfile
        from unittest import mock
        with tempfile.TemporaryDirectory() as tmp:
            with mock.patch.object(REPORT, "EVIDENCE", Path(tmp)):
                output = io.StringIO()
                with contextlib.redirect_stdout(output):
                    self.assertEqual(REPORT.main(["ghost", "--sccache"]), 0)
                self.assertIn("no sccache snapshots", output.getvalue())

    def test_report_sccache_prints_delta_from_snapshots(self):
        import contextlib
        import io
        import tempfile
        from unittest import mock
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "run-sccache-before.log").write_text(self.SAMPLE)
            (root / "run-sccache-after.log").write_text(
                self.SAMPLE.replace("9190\nCache hits (Rust)", "9290\nCache hits (Rust)"))
            with mock.patch.object(REPORT, "EVIDENCE", root):
                output = io.StringIO()
                with contextlib.redirect_stdout(output):
                    self.assertEqual(REPORT.main(["run", "--sccache"]), 0)
                self.assertIn("+100 hits", output.getvalue())


class RustcLogTests(unittest.TestCase):
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


if __name__ == "__main__":
    unittest.main()
