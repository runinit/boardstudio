#!/usr/bin/env python3
from __future__ import annotations

import contextlib
import importlib.util
import io
import os
from pathlib import Path
import sys
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("check", Path(__file__).with_name("check.py"))
check = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(check)


class CheckTests(unittest.TestCase):
    def setUp(self):
        driver = mock.patch.dict(os.environ, {"BOARDSTUDIO_DRAG_DRIVER": "node"})
        driver.start()
        self.addCleanup(driver.stop)
        locate = mock.patch.object(check.browser_tools, "locate", return_value=("node", "chrome"))
        self.locate = locate.start()
        self.addCleanup(locate.stop)
        chrome = mock.patch.object(check.browser_tools, "locate_chrome", return_value="chrome")
        self.chrome = chrome.start()
        self.addCleanup(chrome.stop)

    def test_every_referenced_script_and_manifest_exists(self):
        for name, (_, commands) in check.STEPS.items():
            for command in commands:
                for argument in command:
                    if argument.endswith(".py") or argument.endswith("Cargo.toml"):
                        self.assertTrue((ROOT / argument).is_file(), f"{name}: {argument} is missing")

    def test_supported_test_scripts_are_registered_or_explicitly_opt_in(self):
        scripts = {str(path.relative_to(ROOT)) for directory in ("scripts", "cad/scripts")
                   for pattern in ("test-*.py", "test-*.cjs", "*.test.py")
                   for path in (ROOT / directory).glob(pattern)}
        registered = {argument for _, commands in check.STEPS.values()
                      for command in commands for argument in command if argument in scripts}
        delegated = {"scripts/test-drag-interactions.cjs": "scripts/run-drag-browser.py"}
        opt_in = {
            "cad/scripts/test-cold-responses.py": "diagnostic requiring a packaged site, capture binary and Chrome",
            "cad/scripts/test-startup-readiness.py": "diagnostic requiring a packaged site, capture binary and Chrome",
            "scripts/test-drag-browser-consumer-controls.py": "Linux CI consumer fault controls",
            "scripts/test-graphite-feedback.cjs": "interactive Graphite acceptance against a running dev server",
            "scripts/test-graphite-usability.cjs": "compatibility entry point for Graphite acceptance",
            "scripts/test-parts-inspector-scroll.cjs": "Parts inspector check against a running site and Chrome",
        }
        self.assertEqual(scripts, registered | delegated.keys() | opt_in.keys())
        self.assertTrue(all(opt_in.values()))
        for runner in delegated.values():
            self.assertTrue(any(runner in command for _, commands in check.STEPS.values()
                                for command in commands))

    def test_measurement_protocols_run_in_tooling_with_node_prerequisite(self):
        commands = check.STEPS["tooling"][1]
        self.assertIn([check.PY, "-B", "cad/scripts/test-measure-cad.py"], commands)
        self.assertIn(["node", "cad/scripts/test-measure-cad-runtime.cjs"], commands)
        self.assertLess(commands.index(["node", "--version"]),
                        commands.index(["node", "cad/scripts/test-measure-cad-runtime.cjs"]))

    def test_ci_web_tools_do_not_install_playwright(self):
        action = (ROOT / ".github/actions/web-tools/action.yml").read_text()
        workflow = (ROOT / ".github/workflows/check.yaml").read_text()
        self.assertNotIn("playwright", action.lower())
        self.assertNotIn("NODE_PATH", action)
        self.assertIn("env -u NODE_PATH python3 scripts/check.py build browser", workflow)
        self.assertIn("Verify Rust browser checks need no Playwright", workflow)

    def test_browser_runs_packaged_provider_regressions(self):
        self.assertIn([check.PY, "cad/scripts/test-cad-provider.py"], check.STEPS["browser"][1])

    def test_full_check_runs_the_documented_steps_in_order(self):
        self.assertEqual(check.DEFAULT, ("repo", "tooling", "lint", "build", "test", "browser"))
        self.assertNotIn("security", check.DEFAULT)
        self.assertNotIn("typecheck", check.DEFAULT)

    def test_lint_runs_wasm_clippy_over_the_web_workspace_crates(self):
        # Native Clippy builds the non-WASM stubs, so the WASM-gated web crates
        # (and their browser tests) need an explicit WASM-target Clippy command.
        wasm_clippy = [command for command in check.LINT
                       if command[:2] == ["cargo", "clippy"] and "wasm32-unknown-unknown" in command]
        self.assertEqual(len(wasm_clippy), 1, "the lint step runs exactly one WASM-target Clippy command")
        command = wasm_clippy[0]
        packages = [command[index + 1] for index, token in enumerate(command[:-1]) if token == "--package"]
        self.assertEqual(
            packages,
            ["boardstudio-web", *[f"boardstudio-web-{crate}" for crate in check.browser_crates()]],
        )
        self.assertEqual(command[command.index("--target") + 1], "wasm32-unknown-unknown")
        self.assertIn("--all-targets", command, "the library, binary, and browser-test target kinds are all linted")
        self.assertNotIn("--workspace", command, "the WASM scope stays bounded to the web workspace crates")
        self.assertIn("--locked", command)
        self.assertIn("--no-deps", command)
        self.assertEqual(command[command.index("--"):], ["--", "-D", "warnings"])
        # The existing formatting and native workspace lint stay, and lint remains a default step.
        self.assertEqual(check.LINT[0], ["cargo", "fmt", "--all", "--check"])
        self.assertTrue(any(part == "--workspace" for part in check.LINT[1]))
        self.assertIs(check.STEPS["lint"][1], check.LINT)
        self.assertIn("lint", check.DEFAULT)

    def test_unknown_steps_are_rejected(self):
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit) as raised:
            check.main(["nonsense"])
        self.assertEqual(raised.exception.code, 2)

    def test_a_failing_command_stops_the_run_and_names_the_step(self):
        # The second of three `repo` commands fails: the third and `tooling` must never start.
        results = iter([mock.Mock(returncode=0), mock.Mock(returncode=3)])
        with mock.patch.dict(check.STEPS, {"repo": (True, [["first"], ["second"], ["never"]])}), \
                mock.patch.object(check.subprocess, "run", side_effect=lambda *a, **k: next(results)) as run, \
                contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()) as errors:
            self.assertEqual(check.run(["repo", "tooling"]), 1)
        self.assertEqual(run.call_count, 2)
        self.assertIn("step 'repo' failed", errors.getvalue())

    def test_native_checks_do_not_inherit_parent_appimage_directory(self):
        # System KiCad loads its libraries from APPDIR when an editor leaks it.
        probe = [sys.executable, "-c",
                 "import os, sys; sys.exit(0 if 'APPDIR' not in os.environ "
                 "and os.environ.get('APPIMAGE') == 'editor.AppImage' else 17)"]
        with mock.patch.dict(os.environ, {"APPDIR": "/tmp/editor-appimage", "APPIMAGE": "editor.AppImage"}), \
                mock.patch.dict(check.STEPS, {"test": (True, [probe])}), \
                contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(check.run(["test"]), 0)
            self.assertEqual(os.environ["APPDIR"], "/tmp/editor-appimage")

    def test_browser_checks_supply_a_bounded_timeout_without_changing_other_steps(self):
        probe = [sys.executable, "-c",
                 "import os, sys; actual = os.environ.get('WASM_BINDGEN_TEST_TIMEOUT', '<unset>'); "
                 "sys.exit(0 if actual == sys.argv[1] else "
                 "f'browser timeout was {actual}, expected {sys.argv[1]}')"]
        with mock.patch.dict(os.environ), \
                mock.patch.dict(check.STEPS, {
                    "browser": (True, [[*probe, "120"], [*probe, "120"]]),
                    "tooling": (True, [[*probe, "<unset>"]]),
                }), \
                contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            os.environ.pop("WASM_BINDGEN_TEST_TIMEOUT", None)
            self.assertEqual(check.run(["browser", "tooling"]), 0)
            self.assertNotIn("WASM_BINDGEN_TEST_TIMEOUT", os.environ)

    def test_browser_checks_preserve_an_explicit_timeout(self):
        probe = [sys.executable, "-c",
                 "import os, sys; sys.exit(0 if os.environ.get('WASM_BINDGEN_TEST_TIMEOUT') "
                 "== '37' else 17)"]
        with mock.patch.dict(os.environ, {"WASM_BINDGEN_TEST_TIMEOUT": "37"}), \
                mock.patch.dict(check.STEPS, {"browser": (True, [probe, probe])}), \
                contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(check.run(["browser"]), 0)
            self.assertEqual(os.environ["WASM_BINDGEN_TEST_TIMEOUT"], "37")

    def test_list_prints_every_step_without_running_anything(self):
        output = io.StringIO()
        with mock.patch.object(check.subprocess, "run") as run, contextlib.redirect_stdout(output):
            self.assertEqual(check.main(["--list"]), 0)
        run.assert_not_called()
        for name in check.STEPS:
            self.assertIn(name, output.getvalue())

    def test_browser_step_builds_fresh_page_assets_before_the_drag_smoke(self):
        captured = {}

        def record(plan, update_native_baseline=False):
            captured["plan"] = plan
            return 0

        with mock.patch.object(check, "run_plan", side_effect=record):
            self.assertEqual(check.run(["browser"]), 0)
        self.assertEqual([name for name, _ in captured["plan"]], ["build", "browser"])
        self.assertEqual(captured["plan"][-1][1][0], check.DRAG_SMOKE)

    def test_drag_smoke_can_run_as_a_fresh_build_preflight(self):
        captured = {}

        def record(plan, update_native_baseline=False):
            captured["plan"] = plan
            return 0

        with mock.patch.object(check, "run_plan", side_effect=record):
            self.assertEqual(check.run(["drag-smoke"]), 0)
        self.assertEqual([name for name, _ in captured["plan"]], ["build", "drag-smoke"])
        self.assertEqual(captured["plan"][-1][1], [check.DRAG_SMOKE])

    def test_missing_browser_tools_fail_before_any_command_or_build_is_launched(self):
        for driver, probe in (("node", self.locate), ("rust", self.chrome)):
            probe.side_effect = check.browser_tools.MissingBrowserTool("missing browser tool")
            for names in (["browser"], ["drag-smoke"], list(check.DEFAULT), ["build", "browser"]):
                error = io.StringIO()
                with self.subTest(driver=driver, names=names), \
                        mock.patch.dict(os.environ, {"BOARDSTUDIO_DRAG_DRIVER": driver}), \
                        mock.patch.object(check, "run_plan") as run_plan, \
                        mock.patch.object(check.subprocess, "run") as launch, \
                        contextlib.redirect_stderr(error):
                    self.assertEqual(check.run(names), 1)
                run_plan.assert_not_called()
                launch.assert_not_called()
                self.assertIn("check: missing browser tool", error.getvalue())
            probe.side_effect = None

    def test_default_drag_driver_uses_rust_and_chrome_only(self):
        spec = importlib.util.spec_from_file_location("drag_browser", ROOT / "scripts/run-drag-browser.py")
        drag = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(drag)
        with mock.patch.dict(os.environ):
            os.environ.pop("BOARDSTUDIO_DRAG_DRIVER", None)
            self.assertEqual(drag.drag_driver(), "rust")
            with mock.patch.object(check, "run_plan", return_value=0):
                self.assertEqual(check.run(["drag-smoke"]), 0)
            self.chrome.assert_called_once_with()
            self.locate.assert_not_called()

    def test_browser_tools_are_probed_once_before_the_unchanged_command_order(self):
        for driver, probe, other in (("node", self.locate, self.chrome), ("rust", self.chrome, self.locate)):
            calls = []
            probe.reset_mock()
            other.reset_mock()
            probe.side_effect = lambda: calls.append("probe") or "chrome"

            def record(plan, update_native_baseline=False):
                calls.append([name for name, _ in plan])
                return 0

            with self.subTest(driver=driver), \
                    mock.patch.dict(os.environ, {"BOARDSTUDIO_DRAG_DRIVER": driver}), \
                    mock.patch.object(check, "run_plan", side_effect=record):
                self.assertEqual(check.run(list(check.DEFAULT)), 0)
            self.assertEqual(calls, ["probe", list(check.DEFAULT)])
            probe.assert_called_once_with()
            other.assert_not_called()
            probe.side_effect = None

    def test_scoped_browser_only_does_not_acquire_the_drag_smoke_prerequisites(self):
        self.locate.side_effect = check.browser_tools.MissingBrowserTool("absent")
        with mock.patch.object(check.subprocess, "run", return_value=mock.Mock(returncode=0, stdout="")), \
                contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(check.main(["browser", "--only", "case"]), 0)
        self.locate.assert_not_called()

    def test_host_browser_gate_includes_cad_worker_tests_alongside_storage(self):
        launches = check.select_browser_crate_launches(["host"])
        self.assertEqual(len(launches), 1)
        command = launches[0]
        self.assertEqual(command[command.index("--lib") + 1:], [],
                         "Host filtering must not exclude the CAD worker browser regressions")


    def test_each_command_and_step_reports_its_wall_time(self):
        output = io.StringIO()
        with mock.patch.dict(check.STEPS, {"repo": (True, [["first"], ["second"]])}), \
                mock.patch.object(check.subprocess, "run", return_value=mock.Mock(returncode=0)), \
                mock.patch.object(check.time, "monotonic", side_effect=[0.0, 2.0, 2.0, 5.0]), \
                contextlib.redirect_stdout(output):
            self.assertEqual(check.run(["repo"]), 0)
        text = output.getvalue()
        self.assertIn("(2.0s)", text)
        self.assertIn("check: step times: repo 5.0s, total 5.0s", text)

    def test_browser_checks_build_test_wasm_without_debug_info_unless_overridden(self):
        probe = [sys.executable, "-c",
                 "import os, sys; sys.exit(0 if os.environ.get('CARGO_PROFILE_DEV_DEBUG') == sys.argv[1] else 17)"]
        with mock.patch.dict(os.environ), \
                mock.patch.dict(check.STEPS, {"browser": (True, [[*probe, "false"]]),
                                              "tooling": (True, [[*probe, "<unset>"]])}), \
                contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            os.environ.pop("CARGO_PROFILE_DEV_DEBUG", None)
            self.assertEqual(check.run(["browser"]), 0)
            # Other steps, and the caller's own environment, keep the normal profile.
            self.assertEqual(check.run(["tooling"]), 1)
            self.assertNotIn("CARGO_PROFILE_DEV_DEBUG", os.environ)

    def test_crate_launches_run_together_and_a_failure_names_the_launch(self):
        launches = [["wasm-pack", "test", "--headless", "--chrome", f"web/crates/{name}", "--locked", "--lib"]
                    for name in ("keymap", "layout", "case")]
        serial = [sys.executable, "-c", "pass"]
        seen = []

        def fake(command, **kwargs):
            seen.append(command)
            code = 1 if "web/crates/case" in command else 0
            return mock.Mock(returncode=code, stdout="output\n")

        error = io.StringIO()
        with mock.patch.object(check.subprocess, "run", side_effect=fake), \
                mock.patch.dict(os.environ, {"BOARDSTUDIO_CHECK_JOBS": "3"}), \
                mock.patch.dict(check.STEPS, {"browser": (True, [serial, *launches, serial])}), \
                contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(error):
            self.assertEqual(check.run(["browser"]), 1)
        self.assertIn("launch failed: wasm-pack test --headless --chrome web/crates/case", error.getvalue())
        self.assertEqual(seen[0], serial)
        self.assertEqual(len(seen), 4, "the trailing serial command does not run after a failed batch")
        self.assertEqual(check.launch_order(launches[1]), 0, "heavy crates start first")
        self.assertEqual(check.launch_order(launches[0]), 1)

    def test_the_browser_job_cap_defaults_to_four_and_one_means_serial(self):
        with mock.patch.dict(os.environ):
            os.environ.pop("BOARDSTUDIO_CHECK_JOBS", None)
            self.assertEqual(check.check_jobs(), 4)
            os.environ["BOARDSTUDIO_CHECK_JOBS"] = "junk"
            self.assertEqual(check.check_jobs(), 4)
            os.environ["BOARDSTUDIO_CHECK_JOBS"] = "0"
            self.assertEqual(check.check_jobs(), 1)

    def test_build_is_fast_and_release_is_opt_in(self):
        self.assertIn("--fast", check.STEPS["build"][1][0])
        self.assertNotIn("--fast", check.STEPS["release"][1][0])
        self.assertNotIn("release", check.DEFAULT)

    def test_layout_test_css_build_runs_from_the_web_project(self):
        command = ["dx", "build"]
        with mock.patch.object(check.subprocess, "run", return_value=mock.Mock(returncode=0)) as run, \
                contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(check.run_plan([("layout-test-css", [command])]), 0)
        self.assertEqual(run.call_args.kwargs["cwd"], check.ROOT / "web")

    def test_independent_native_test_commands_run_together(self):
        commands = [[sys.executable, "-c", f"print({index})"] for index in range(3)]
        launched = []

        def fake(command, **kwargs):
            launched.append(command)
            return mock.Mock(returncode=0, stdout="ok\n")

        with mock.patch.object(check.subprocess, "run", side_effect=fake), \
                mock.patch.dict(check.STEPS, {"test": (True, commands)}), \
                contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(check.run(["test"]), 0)
        self.assertEqual(sorted(launched), sorted(commands))

        launched.clear()
        with mock.patch.object(check.subprocess, "run", side_effect=fake), \
                mock.patch.dict(os.environ, {"BOARDSTUDIO_CHECK_JOBS": "1"}), \
                mock.patch.dict(check.STEPS, {"test": (True, commands)}), \
                contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(check.run(["test"]), 0)
        self.assertEqual(launched, commands, "one job runs them in order")

    def test_browser_crates_are_derived_from_the_browser_plan_without_duplication(self):
        self.assertEqual(
            check.browser_crates(),
            ("host", "runtime", "ui-model", "ui-shared", "keycaps", "library",
             "keymap", "catalogue", "case", "parts", "pcb", "layout"),
        )
        custom = [["wasm-pack", "test", "--headless", "--chrome", "web/crates/custom-crate", "--lib"]]
        with mock.patch.dict(check.STEPS, {"browser": (True, custom)}):
            self.assertEqual(check.browser_crates(), ("custom-crate",))

    def test_browser_only_selects_crate_launches_without_build_or_drag_smoke_and_reports_partial_run(self):
        seen = []

        def fake(command, **kwargs):
            seen.append(command)
            return mock.Mock(returncode=0, stdout="ok\n")

        output = io.StringIO()
        with mock.patch.object(check.subprocess, "run", side_effect=fake), \
                contextlib.redirect_stdout(output), contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(check.main(["browser", "--only", "case", "ui-shared"]), 0)
        self.assertEqual(len(seen), 3, "case plus both ui-shared launches run without build or smoke")
        self.assertTrue(all(check.is_crate_launch(cmd) for cmd in seen))
        self.assertFalse(any("build-web.py" in " ".join(cmd) or "run-drag-browser.py" in " ".join(cmd)
                             for cmd in seen))
        text = output.getvalue()
        self.assertIn("== browser (only: case, ui-shared)", text)
        self.assertIn("check: partial steps passed: browser (only: case, ui-shared)", text)
        self.assertNotIn("check: all steps passed:", text)

    def test_browser_only_layout_runs_layout_test_css_prerequisite_and_stops_if_it_fails(self):
        seen = []

        def fake_pass(command, **kwargs):
            seen.append((command, kwargs.get("cwd")))
            return mock.Mock(returncode=0, stdout="ok\n")

        with mock.patch.object(check.subprocess, "run", side_effect=fake_pass), \
                mock.patch.dict(os.environ, {"BOARDSTUDIO_CHECK_JOBS": "1"}), \
                contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(check.main(["browser", "--only", "layout"]), 0)
        self.assertEqual(seen[0], (check.LAYOUT_TEST_CSS, check.ROOT / "web"))
        self.assertEqual(len(seen), 3, "layout-test-css followed by both layout crate launches")

        seen.clear()
        error = io.StringIO()
        with mock.patch.object(check.subprocess, "run", return_value=mock.Mock(returncode=2, stdout="")), \
                contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(error):
            self.assertEqual(check.main(["browser", "--only", "layout", "--keep-going"]), 1)
        self.assertIn("step 'layout-test-css' failed", error.getvalue())

    def test_browser_only_and_keep_going_validate_argument_combinations(self):
        invalid = (
            (["browser", "--only", "nonexistent"], "unknown browser crate(s): nonexistent"),
            (["--only", "case"], "--only applies to `browser`"),
            (["build", "browser", "--only", "case"], "--only applies to `browser`"),
            (["repo", "browser", "--only", "case"], "--only applies to `browser`"),
            (["quick", "--only", "case"], "--only applies to `browser`"),
            (["--list", "--only", "case"], "--only"),
            (["repo", "--keep-going"], "--keep-going applies to the `browser` step"),
            (["--list", "--keep-going"], "--keep-going"),
        )
        for argv, expected_error in invalid:
            stderr = io.StringIO()
            with self.subTest(argv=argv), contextlib.redirect_stderr(stderr), \
                    self.assertRaises(SystemExit) as raised:
                check.main(argv)
            self.assertEqual(raised.exception.code, 2)
            self.assertIn(expected_error, stderr.getvalue())

    def test_browser_keep_going_runs_remaining_launches_serially_and_exits_nonzero(self):
        seen = []

        def fake(command, **kwargs):
            seen.append(command)
            failed = any(part in ("web/crates/library", "web/crates/case") for part in command)
            return mock.Mock(returncode=1 if failed else 0, stdout="output\n")

        error = io.StringIO()
        with mock.patch.object(check.subprocess, "run", side_effect=fake), \
                mock.patch.dict(os.environ, {"BOARDSTUDIO_CHECK_JOBS": "1"}), \
                contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(error):
            self.assertEqual(
                check.main(["browser", "--only", "library,case", "parts", "--keep-going"]),
                1,
            )
        self.assertEqual(
            [check.launch_crate(cmd) for cmd in seen],
            ["library", "case", "parts"],
        )
        stderr = error.getvalue()
        self.assertIn("launch failed: wasm-pack test --headless --chrome web/crates/library", stderr)
        self.assertIn("launch failed: wasm-pack test --headless --chrome web/crates/case", stderr)
        self.assertIn("step 'browser (only: library, case, parts)' failed", stderr)

        # In a parallel batch with --keep-going, a failed crate launch continues to trailing
        # independent browser commands unless the page `--list` prerequisite itself fails.
        seen.clear()
        launches = [["wasm-pack", "test", "--headless", "--chrome", f"web/crates/{name}", "--locked", "--lib"]
                    for name in ("library", "case")]
        page_list = ["wasm-pack", "test", "--headless", "--chrome", "web", "--bin", "boardstudio-web", "--", "--list"]
        page_run = [sys.executable, "scripts/run-wasm-tests.py", "--all"]
        with mock.patch.object(check.subprocess, "run", side_effect=fake), \
                mock.patch.dict(os.environ, {"BOARDSTUDIO_CHECK_JOBS": "2"}), \
                mock.patch.dict(check.STEPS, {"browser": (True, [*launches, page_list, page_run])}), \
                contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(check.run(["browser"], keep_going=True), 1)
        self.assertIn(page_run, seen)

        seen.clear()

        def fail_list(command, **kwargs):
            seen.append(command)
            return mock.Mock(returncode=1 if "--list" in command else 0, stdout="")

        with mock.patch.object(check.subprocess, "run", side_effect=fail_list), \
                mock.patch.dict(check.STEPS, {"browser": (True, [*launches, page_list, page_run])}), \
                contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(check.run(["browser"], keep_going=True), 1)
        self.assertNotIn(page_run, seen, "page suite does not run when its --list prerequisite failed")

    def test_wasm_failure_summary_distinguishes_raw_wasm_trap_rust_panic_cascade_and_timeout(self):
        trap_log = """\
running 1 test
Failed to detect test as having been run. It might have timed out.
console output:
    Invoking test: library::mounted_tests::project_menu_name_draft_survives_unrelated_accepted_revision_and_commits_latest_document
    wasm-bindgen: imported JS function that was not marked as `catch` threw an error:
    memory access out of bounds

    Stack:
    RuntimeError: memory access out of bounds
        at boardstudio_web_library-f0d869e22bb8c46e.wasm.__wbindgen_exn_store (http://127.0.0.1:36115/wasm-bindgen-test_bg.wasm:wasm-function[54137]:0x14f1a6e)
        at handleError (http://127.0.0.1:36115/wasm-bindgen-test:3084:14)
driver status: signal: 9 (SIGKILL)
"""
        trap = check.summarize_wasm_failure(trap_log)
        self.assertEqual(trap["kind"], "wasm_trap")
        self.assertEqual(
            trap["test"],
            "library::mounted_tests::project_menu_name_draft_survives_unrelated_accepted_revision_and_commits_latest_document",
        )
        self.assertEqual(trap["location"], "__wbindgen_exn_store")
        self.assertEqual(trap["detail"], "RuntimeError: memory access out of bounds")
        self.assertTrue(trap["timed_out_after_failure"])
        trap_text = check.format_wasm_failure_summary(trap)
        self.assertIn("first failure [raw WASM trap]", trap_text)
        self.assertIn("memory access out of bounds", trap_text)
        self.assertIn("__wbindgen_exn_store", trap_text)
        self.assertIn("later harness timeout was a follow-on symptom", trap_text)

        cascade_log = """\
running 4 tests
test shared_viewer::tests::viewer_events_reject_changed_owner_identity_and_unmount ... ok
test shared_viewer::tests::vendor_step_sibling_lines_attach_to_pcb_and_module_models_with_single_row_transform ... FAIL
test shared_viewer::tests::mounted_case_gesture_feedback_preserves_canvas_geometry ... FAIL
test case_assembly_layers::tests::layer_menu_toggles_exact_rows_and_unavailable_rows_are_not_checked ... FAIL
Failed to detect test as having been run. It might have timed out.
console output:
    Invoking test: shared_viewer::tests::viewer_events_reject_changed_owner_identity_and_unmount
    Invoking test: shared_viewer::tests::vendor_step_sibling_lines_attach_to_pcb_and_module_models_with_single_row_transform
    panicked at web/crates/case/src/shared_viewer.rs:3794:9:
    assertion `left == right` failed
      left: Array [Number(12), Number(26), Number(42), Number(18), Number(35), Number(54)]
     right: Array [Number(12.0), Number(26.0), Number(42.0), Number(18.0), Number(35.0), Number(54.0)]

    Stack:
    Error
    Invoking test: shared_viewer::tests::mounted_case_gesture_feedback_preserves_canvas_geometry
    panicked at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library/std/src/panicking.rs:144:9:
    cannot modify the panic hook from a panicking thread
    RuntimeError: unreachable
        at boardstudio_web_case.wasm.__rustc::__rust_abort (wasm:0x1)
    panicked at web/crates/case/src/shared_viewer.rs:4358:14:
    real SharedViewer canvas mounts
    Invoking test: case_assembly_layers::tests::layer_menu_toggles_exact_rows_and_unavailable_rows_are_not_checked
    panicked at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library/std/src/panicking.rs:144:9:
    cannot modify the panic hook from a panicking thread
"""
        cascade = check.summarize_wasm_failure(cascade_log)
        self.assertEqual(cascade["kind"], "rust_panic")
        self.assertEqual(
            cascade["test"],
            "shared_viewer::tests::vendor_step_sibling_lines_attach_to_pcb_and_module_models_with_single_row_transform",
        )
        self.assertEqual(cascade["location"], "web/crates/case/src/shared_viewer.rs:3794:9")
        self.assertIn("Number(12)", cascade["detail"])
        self.assertIn("Number(12.0)", cascade["detail"])
        self.assertEqual(cascade["panic_hook_cascade_count"], 2)
        self.assertTrue(cascade["timed_out_after_failure"])
        cascade_text = check.format_wasm_failure_summary(cascade)
        self.assertIn("first failure [Rust panic/assertion]", cascade_text)
        self.assertIn("2 later test(s) hit 'cannot modify the panic hook from a panicking thread'", cascade_text)

        timeout_log = """\
running 1 test
Failed to detect test as having been run. It might have timed out.
console output:
    Invoking test: presentation::x::tests::genuinely_hangs
driver status: signal: 9 (SIGKILL)
"""
        timeout = check.summarize_wasm_failure(timeout_log)
        self.assertEqual(timeout["kind"], "timeout")
        self.assertEqual(timeout["test"], "presentation::x::tests::genuinely_hangs")
        self.assertFalse(timeout["timed_out_after_failure"])
        self.assertIn("first failure [timeout]", check.format_wasm_failure_summary(timeout))

        error = io.StringIO()
        with mock.patch.object(check.subprocess, "run", return_value=mock.Mock(returncode=1, stdout=trap_log)), \
                mock.patch.dict(os.environ, {"BOARDSTUDIO_CHECK_JOBS": "1"}), \
                contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(error):
            self.assertEqual(check.main(["browser", "--only", "library"]), 1)
        self.assertIn("first failure [raw WASM trap]", error.getvalue())


if __name__ == "__main__":
    unittest.main()
