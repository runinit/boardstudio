#!/usr/bin/env python3
"""Hermetic checks of the release gate's wiring and failure propagation."""

import importlib
import os
import subprocess
import threading
import unittest
from unittest.mock import patch

import check


class CheckTests(unittest.TestCase):
    def test_browser_targets_and_wasm_lint_are_independent(self):
        self.assertEqual(check.browser(),
                         [check.DRAG_SMOKE, check.CAD_BROWSER, *check.PROVIDER_CHECKS, check.PAGE_SUITE])
        self.assertEqual(check.BROWSER_PREBUILDS, [check.PAGE_TEST_BUILD])
        self.assertNotIn("browser-packaged", check.STEPS)
        self.assertNotIn("browser-wasm", check.STEPS)
        runner = importlib.import_module("run-wasm-tests")
        self.assertEqual(self.option_values(runner.PAGE_BUILD, "-p"), ["boardstudio-web"])
        packages = {"boardstudio-web"} | {
            f"boardstudio-web-{crate}" for crate in (
                "host", "worker-host", "runtime", "ui-shared", "keycaps", "library",
                "keymap", "case", "parts", "pcb", "layout", "layout-objects",
            )
        }
        self.assertEqual(set(self.option_values(check.LINT[-1], "--package")), packages)

    @staticmethod
    def option_values(command, option):
        return [value for flag, value in zip(command, command[1:]) if flag == option]

    def test_build_overlaps_web_and_browser_prebuilds_and_retains_artifacts(self):
        barrier = threading.Barrier(2, timeout=2)
        css_calls = []
        lane_envs = {}
        state = {}
        env = {}

        def serial(command, environment, cwd, outputs=None, **kwargs):
            if command == ["tailwind"]:
                self.assertEqual(cwd, check.ROOT / "web")
                css_calls.append(list(command))
                return 0, 0
            if command == check.STEPS["build"][1][0]:
                lane_envs["web"] = environment.copy()
                barrier.wait()
            elif command == check.PAGE_TEST_BUILD:
                lane_envs["page"] = environment.copy()
                barrier.wait()
                outputs.append((command, "/tmp/page.wasm\n"))
            return 0, 0

        with patch.object(check, "check_jobs", return_value=2), \
             patch.object(check, "test_css_command", return_value=["tailwind"]), \
             patch.object(check, "run_serial", side_effect=serial):
            _, passed = check.run_commands("build", check.STEPS["build"][1], env, state=state)
            self.assertTrue(passed)
            self.assertEqual(css_calls, [["tailwind"]])
            self.assertEqual(lane_envs["web"][check.TEST_CSS_READY], "1")
            self.assertEqual(lane_envs["web"][check.IN_OVERLAP_ENV], "1")
            self.assertEqual(lane_envs["page"][check.TEST_CSS_READY], "1")
            _, second = check.run_commands("build", check.STEPS["build"][1], {}, state=state)
            self.assertTrue(second)
            self.assertEqual(len(css_calls), 1)
        self.assertEqual(state["page"], "/tmp/page.wasm")
        self.assertEqual(env[check.TEST_CSS_READY], "1")

    def test_drag_smoke_plan_skips_page_prebuild_without_browser(self):
        with patch.object(check.browser_tools, "locate_chrome"), \
             patch.object(check.browser_tools, "locate_chromedriver"), \
             patch.object(check, "run_plan") as execute:
            check.run(["drag-smoke"])
        plan = dict(execute.call_args.args[0])
        self.assertIn("drag-smoke", plan)
        self.assertNotIn(check.PAGE_TEST_BUILD, plan["build"])

    def test_keep_going_runs_every_browser_launch_after_a_failure(self):
        calls = iter([(0, 1)] + [(0, 0)] * 5)

        def launch(command, env, directory):
            return next(calls)

        with patch.object(check, "check_jobs", return_value=1), \
             patch.object(check, "run_serial", side_effect=launch) as launched:
            _, passed = check.run_browser(check.browser(), {}, check.ROOT, {}, True)
        self.assertFalse(passed)
        self.assertEqual([call.args[0] for call in launched.call_args_list],
                         [check.DRAG_SMOKE, check.CAD_BROWSER, *check.PROVIDER_CHECKS, check.PAGE_SUITE])

    def test_run_browser_lane_schedules(self):
        def run_lanes(commands, jobs):
            recorded = []

            def fake_parallel(lanes, _env):
                recorded.append(lanes)
                return 0, True

            with patch.object(check, "check_jobs", return_value=jobs), \
                 patch.object(check, "run_parallel", side_effect=fake_parallel):
                _, passed = check.run_browser(commands, {}, check.ROOT, {"page": "/tmp/page.wasm"}, False)
                self.assertTrue(passed)
            self.assertEqual(len(recorded), 1)
            return recorded[0]

        full6 = run_lanes(check.browser(), 6)
        self.assertEqual(len(full6), 4)
        self.assertEqual(full6[0][0][0], [*check.DRAG_SMOKE, "--suite", "drag"])
        self.assertEqual(full6[1][0][0], [*check.DRAG_SMOKE, "--suite", "collision"])
        self.assertEqual(len(full6[2]), 1 + len(check.PROVIDER_CHECKS))
        self.assertEqual(full6[2][0][0], check.CAD_BROWSER)
        self.assertEqual(full6[3][0][0], [*check.PAGE_SUITE, "--test-binary", "/tmp/page.wasm"])
        self.assertEqual(full6[3][0][2]["BOARDSTUDIO_CHECK_JOBS"], "2")

        page3 = run_lanes([check.PAGE_SUITE], 3)
        self.assertEqual(len(page3), 1)
        self.assertEqual(page3[0][0][0], [*check.PAGE_SUITE, "--test-binary", "/tmp/page.wasm"])
        self.assertEqual(page3[0][0][2]["BOARDSTUDIO_CHECK_JOBS"], "2")

        full2 = run_lanes(check.browser(), 2)
        self.assertEqual(len(full2), 3)
        self.assertEqual(full2[0][0][0], check.DRAG_SMOKE)
        self.assertEqual(full2[2][0][2]["BOARDSTUDIO_CHECK_JOBS"], "1")

    def test_native_baseline_sees_captured_workspace_output(self):
        plan = [("test", check.CARGO_TESTS)]

        def cargo(command, env, cwd, *, captured):
            captured.append((command, "workspace output"))
            return 0, 0

        with patch.object(check, "run_serial", side_effect=cargo), \
             patch.object(check.check_native_tests, "verify", return_value=(True, True)) as verify:
            self.assertEqual(check._run_plan(plan, True), 0)
            verify.assert_called_once_with(["workspace output"], full=True, update=True,
                                           baseline_path=check.check_native_tests.BASELINE)
            verify.return_value = False, False
            self.assertEqual(check._run_plan(plan), 1)
            self.assertEqual(check._run_plan([], True), 1)

    def test_overlap_passes_one_jobserver_and_separate_lint_target(self):
        names = ["repo", "tooling", "build", "lint", "test"]
        group = [(name, check.STEPS[name][1]) for name in names]
        with patch.dict(os.environ, {"MAKEFLAGS": ""}), \
             patch.object(check.os, "cpu_count", return_value=16), \
             patch.object(check, "run_parallel", return_value=(0, True)) as parallel:
            os.environ.pop("RUST_TEST_THREADS", None)
            self.assertEqual(check.run_overlapped_steps(group, True), (0, True))
        batch = [lane[0] for lane in parallel.call_args.args[0]]
        self.assertEqual([command[2] for command, _, _ in batch], names)
        self.assertIn("--update-native-baseline", batch[-1][0])
        flags = {env["MAKEFLAGS"] for _, _, env in batch}
        self.assertEqual(len(flags), 1)
        self.assertIn("--jobserver-auth=fifo:", flags.pop())
        self.assertEqual(batch[3][2]["CARGO_TARGET_DIR"], str(check.ROOT / "target/lint"))
        self.assertEqual(batch[4][2]["RUST_TEST_THREADS"], "4")

    def test_overlap_runs_css_first_for_a_contiguous_full_plan(self):
        names = ["repo", "tooling", "build", "lint", "test"]
        plan = [(name, check.STEPS[name][1]) for name in names]
        events = []

        def css(command, *_):
            events.append(command)
            return 0, 0

        def overlap(group, _, **_kwargs):
            events.append(group)
            return 0, True

        with patch.dict(os.environ, {"CI": ""}), patch.object(check.os, "cpu_count", return_value=8), \
             patch.object(check, "check_jobs", return_value=6), \
             patch.object(importlib.import_module("build-web"), "page_tools", return_value=True), \
             patch.object(check, "test_css_command", return_value=["tailwind"]), \
             patch.object(check, "run_serial", side_effect=css), \
             patch.object(check, "run_overlapped_steps", side_effect=overlap):
            self.assertEqual(check._run_plan(plan), 0)
            self.assertEqual(events, [["tailwind"], plan])
            self.assertEqual(check.overlap_span(plan[2:]), (0, 3))
            self.assertIsNone(check.overlap_span(plan[:2]))
            self.assertIsNone(check.overlap_span([*plan[:2], ("build", []), *plan[3:]]))
            with patch.dict(os.environ, {"CI": "1"}):
                self.assertIsNone(check.overlap_span(plan))

    def test_overlap_chains_browser_onto_build_lane_in_full_plan(self):
        names = ["repo", "tooling", "build", "lint", "test", "browser"]
        plan = [(name, check.STEPS[name][1]) for name in names]
        with patch.dict(os.environ, {"CI": "", "MAKEFLAGS": ""}), \
             patch.object(check.os, "cpu_count", return_value=8), \
             patch.object(check, "check_jobs", return_value=6), \
             patch.object(importlib.import_module("build-web"), "page_tools", return_value=True), \
             patch.object(check, "run_parallel", return_value=(0, True)) as parallel:
            self.assertEqual(check.overlap_span(plan), (0, 6))
            self.assertEqual(check.run_overlapped_steps(plan, False, keep_going=True), (0, True))
        batch = [lane[0] for lane in parallel.call_args.args[0]]
        self.assertEqual([cmd[2:] for cmd, _, _ in batch], [
            ["repo"], ["tooling"], ["build", "browser", "--keep-going"], ["lint"], ["test"],
        ])
        with patch.dict(os.environ, {"CI": "1"}):
            self.assertIsNone(check.overlap_span(plan))

    def test_parallel_native_gate_only_receives_workspace_cargo_output(self):
        commands = check.STEPS["test"][1]

        def capture(batch, env, *, captured):
            captured.extend((command, " ".join(command)) for lane in batch for command, _, _ in lane)
            return 0, True

        with patch.dict(os.environ, {"CI": "1", "APPDIR": "/app",
                                     "MAKEFLAGS": "--jobserver-auth=fifo:/parent/fifo"}), \
             patch.object(check, "check_jobs", return_value=2), \
             patch.object(check, "run_parallel", side_effect=capture) as parallel, \
             patch.object(check.check_native_tests, "verify", return_value=(True, False)) as verify:
            self.assertEqual(check._run_plan([("test", commands)]), 0)
            verify.assert_called_once_with(["cargo test --workspace --locked --lib --bins --tests"],
                                           full=True, update=False,
                                           baseline_path=check.check_native_tests.BASELINE)
        lanes = parallel.call_args.args[0]
        self.assertEqual([[cmd for cmd, _, _ in lane] for lane in lanes], [check.CARGO_TESTS, commands[-2:]])
        self.assertNotIn("APPDIR", lanes[1][1][2])
        self.assertEqual({env["MAKEFLAGS"] for lane in lanes for _, _, env in lane},
                         {"--jobserver-auth=fifo:/parent/fifo"})

    def test_invalid_inherited_lock_is_refused_with_only_python(self):
        env = dict(os.environ, BOARDSTUDIO_BUILD_LOCK_FD="2")
        result = subprocess.run(["python3", "scripts/check.py", "repo"], cwd=check.ROOT,
                                env=env, text=True, capture_output=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("invalid BOARDSTUDIO_BUILD_LOCK_FD", result.stderr)

if __name__ == "__main__":
    unittest.main()
