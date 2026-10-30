#!/usr/bin/env python3
from __future__ import annotations

import contextlib
import importlib.util
import io
import json
import os
import re
import shlex
from pathlib import Path
import subprocess
import sys
import tempfile
import threading
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("check", Path(__file__).with_name("check.py"))
check = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(check)
CARGO_TARGET = check.cargo_target


class CheckTests(unittest.TestCase):
    def setUp(self):
        target = mock.patch.object(check, "cargo_target", return_value=ROOT / "target")
        target.start()
        self.addCleanup(target.stop)
        driver = mock.patch.dict(os.environ, {"BOARDSTUDIO_DRAG_DRIVER": "rust"})
        driver.start()
        self.addCleanup(driver.stop)
        chrome = mock.patch.object(check.browser_tools, "locate_chrome", return_value="chrome")
        self.chrome = chrome.start()
        self.addCleanup(chrome.stop)

    def test_preflight_rejects_invalid_lock_for_focused_browser_and_quick(self):
        import check_quick
        selection = mock.Mock(notes=[], unplaced=[])
        plan = [("browser", [["wasm-pack", "test", "--chrome", "web/crates/keymap"]])]
        with mock.patch.dict(os.environ, {"BOARDSTUDIO_BUILD_LOCK_FD": "invalid"}), \
                mock.patch.object(check, "_run_plan") as launch, \
                mock.patch.object(check.subprocess, "run") as metadata, \
                mock.patch.object(check_quick, "workspace_packages", return_value={}), \
                mock.patch.object(check_quick, "select", return_value=selection), \
                mock.patch.object(check, "quick_plan", return_value=plan), \
                contextlib.redirect_stderr(io.StringIO()), contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(check.run(["browser"], only=["keymap"]), 1)
            self.assertEqual(check.quick(["file.rs"]), 1)
        launch.assert_not_called()
        metadata.assert_not_called()

    def test_page_build_preflight_reports_cad_wasm_target(self):
        output = io.StringIO()
        plan = [("build", [[check.PY, "scripts/build-web.py", "--fast"]])]
        with mock.patch.object(check, "_run_plan", return_value=0), \
                mock.patch.object(check.shutil, "disk_usage", return_value=mock.Mock(free=1024 ** 3)), \
                contextlib.redirect_stdout(output):
            self.assertEqual(check.run_plan(plan), 0)
        self.assertIn(str(ROOT / "cad/wasm/target"), output.getvalue())

    def test_cad_only_plan_reports_its_build_targets(self):
        output = io.StringIO()
        plan = [("cad", [[check.PY, "cad/scripts/test-cad.py"]])]
        with mock.patch.object(check, "_run_plan", return_value=0), \
                mock.patch.object(check.shutil, "disk_usage", return_value=mock.Mock(free=1024 ** 3)), \
                contextlib.redirect_stdout(output):
            self.assertEqual(check.run_plan(plan), 0)
        for target in ("cad/monstertruck/target", "cad/wasm/target", "cad/monstertruck-qualification/target"):
            self.assertIn(str(ROOT / target), output.getvalue())

    def test_preflight_reports_configured_target_and_tmpdir(self):
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "external" / "missing" / "target"
            target.parent.parent.mkdir()
            output = io.StringIO()
            metadata = subprocess.CompletedProcess([], 0, json.dumps({"target_directory": str(target)}), "")
            with mock.patch.dict(os.environ, {"TMPDIR": directory}, clear=True), \
                    mock.patch.object(check, "cargo_target", side_effect=CARGO_TARGET), \
                    mock.patch.object(check.subprocess, "run", return_value=metadata) as cargo, \
                    mock.patch.object(check.shutil, "disk_usage", return_value=mock.Mock(free=1024 ** 3)) as usage, \
                    mock.patch.object(check, "_run_plan", return_value=0), contextlib.redirect_stdout(output):
                self.assertEqual(check.run_plan([("build", [["build"]])]), 0)
            self.assertEqual(cargo.call_args.args[0][:3], ["cargo", "metadata", "--no-deps"])
            self.assertIn(target.parent.parent, [call.args[0] for call in usage.call_args_list])
            self.assertIn(str(target), output.getvalue())
            self.assertIn(directory, output.getvalue())
            self.assertIn("1.0 GiB", output.getvalue())

    def test_preflight_metadata_failure_reports_unknown(self):
        output = io.StringIO()
        with mock.patch.dict(os.environ, {}, clear=True), \
                mock.patch.object(check, "cargo_target", side_effect=CARGO_TARGET), \
                mock.patch.object(check.subprocess, "run", side_effect=OSError("missing cargo")), \
                mock.patch.object(check, "_run_plan", return_value=0), \
                contextlib.redirect_stdout(output), contextlib.redirect_stderr(output):
            self.assertEqual(check.run_plan([("build", [["build"]])]), 0)
        self.assertIn("build target unknown", output.getvalue())

    def test_launch_seam_covers_serial_and_parallel(self):
        commands = [["cargo", "test", "--locked", str(i)] for i in range(2)]
        for jobs in ("1", "2"):
            with self.subTest(jobs=jobs), mock.patch.dict(os.environ, {"BOARDSTUDIO_CHECK_JOBS": jobs}), \
                    mock.patch.object(check, "launch_command", return_value=subprocess.CompletedProcess([], 0, "ok\n")) as launch, \
                    contextlib.redirect_stdout(io.StringIO()):
                self.assertTrue(check.run_commands("test", commands, os.environ.copy())[1])
                self.assertEqual(launch.call_count, 2)

    def test_scheduler_passes_real_inherited_lock_to_model_lookup(self):
        import fcntl
        with tempfile.TemporaryDirectory() as directory:
            lock = Path(directory) / ".cache/tmp/boardstudio-build.lock"
            lock.parent.mkdir(parents=True)
            with lock.open("w") as held:
                fcntl.flock(held, fcntl.LOCK_EX)
                code = ("import sys, json, subprocess; sys.path.insert(0, %r); import browser_tools; "
                        "root=browser_tools.Path(browser_tools.__file__).resolve().parents[1]; "
                        "artifact={'reason':'compiler-artifact','package_id':'path+'+(root/'tools/browser-harness').as_uri()+'#boardstudio-browser-harness', 'target':{'name':'cad-provider-pages','kind':['bin']},'executable':'/tmp/fake-driver'}; "
                        "def_run=lambda args, **kw: (args[0]=='cargo' or (_ for _ in ()).throw(AssertionError('lock reacquired'))) and subprocess.CompletedProcess(args,0,json.dumps(artifact),''); "
                        "browser_tools.subprocess.run=def_run; assert str(browser_tools._build_harness('cad-provider-pages')) == '/tmp/fake-driver'" % str(ROOT / "scripts"))
                command = [sys.executable, "-c", code, "scripts/model-lookup-pilot.py", "--gate"]
                with mock.patch.dict(os.environ, {"HOME": directory, "BOARDSTUDIO_BUILD_LOCK_FD": str(held.fileno())}):
                    _, passed = check.run_commands("browser", [command], os.environ.copy(), ROOT, None)
                self.assertTrue(passed)

    def test_serial_missing_executable_reports_startup_error(self):
        output = io.StringIO()
        with contextlib.redirect_stderr(output), contextlib.redirect_stdout(io.StringIO()):
            _, code, _, _ = check.run_serial(["/nonexistent/boardstudio-check-command"], os.environ.copy())
        self.assertEqual(code, 127)
        self.assertIn("startup failed", output.getvalue())

    def test_scheduler_relays_drag_output_to_collector(self):
        marker = 'BOARDSTUDIO_DRAG_BINDING {"proof":true}'
        command = [sys.executable, "-c", "print(%r)" % marker, "scripts/run-drag-browser.py"]
        receipt = mock.Mock()
        receipt.run_id = "run"
        receipt.prepare.return_value = "attempt"
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            _, passed = check.run_commands("browser", [command], os.environ.copy(), ROOT, None, receipt=receipt)
        self.assertTrue(passed)
        self.assertIn(marker, output.getvalue())
        self.assertIn(marker, receipt.observe.call_args.args[4])

    def test_invalid_driver_fails_before_build(self):
        plan = [("build", [["build"]]), ("browser", [["wasm-pack", "test", "--chrome", "web/crates/keymap"]])]
        with mock.patch.dict(os.environ, {"CHROMEDRIVER": "/missing/driver", "CHROME_BIN": "/bin/true"}), \
                mock.patch.object(check, "run_commands") as launch, contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(check.run_plan(plan), 1)
        launch.assert_not_called()

    def test_page_only_plan_checks_driver_before_build(self):
        plan = [("lint", [["lint"]]), ("browser", [[check.PY, "scripts/run-wasm-tests.py", "--files", "page.rs"]])]
        with mock.patch.dict(os.environ, {"CHROMEDRIVER": "/missing/driver", "CHROME_BIN": "/bin/true",
                                         "BOARDSTUDIO_WASM_TEST_COMMAND": ""}), \
                mock.patch.object(check, "run_commands", return_value=(0, True)) as launch, \
                contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(check.run_plan(plan), 1)
        launch.assert_not_called()

    def test_page_only_custom_runner_skips_browser_preflight(self):
        plan = [("browser", [[check.PY, "scripts/run-wasm-tests.py", "--files", "page.rs"]])]
        with mock.patch.dict(os.environ, {"CHROMEDRIVER": "/missing/driver",
                                         "BOARDSTUDIO_WASM_TEST_COMMAND": "/bin/true"}), \
                mock.patch.object(check, "run_commands", return_value=(0, True)) as launch, \
                contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(check.run_plan(plan), 0)
        launch.assert_called_once()

    def test_selected_driver_reaches_actual_command_despite_path_conflict(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            selected = root / "selected"
            selected.write_text("#!/bin/sh\nexit 0\n")
            selected.chmod(0o755)
            conflict = root / "chromedriver"
            conflict.write_text("#!/bin/sh\nexit 1\n")
            conflict.chmod(0o755)
            commands = [["wasm-pack", "test", "--headless", "--chrome", "web/crates/keymap"]]
            with mock.patch.dict(os.environ, {"CHROMEDRIVER": str(selected), "CHROME_BIN": "/bin/true", "PATH": directory}), \
                    mock.patch.object(check, "run_commands", return_value=(0, True)) as launch, \
                    contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
                self.assertEqual(check.run_plan([("browser", commands)]), 0)
            actual = launch.call_args.args[1][0]
            self.assertEqual(actual[actual.index("--chromedriver") + 1], str(selected))
            self.assertEqual(launch.call_args.args[2]["CHROMEDRIVER"], str(selected))

    def test_nonexecutable_driver_fails_before_build(self):
        with tempfile.TemporaryDirectory() as directory:
            driver = Path(directory) / "driver"
            driver.write_text("no")
            plan = [("build", [["build"]]), ("browser", [["wasm-pack", "test", "--chrome", "web/crates/keymap"]])]
            with mock.patch.dict(os.environ, {"CHROMEDRIVER": str(driver), "CHROME_BIN": "/bin/true"}), \
                    mock.patch.object(check, "run_commands") as launch, contextlib.redirect_stderr(io.StringIO()):
                self.assertEqual(check.run_plan(plan), 1)
            launch.assert_not_called()

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
        opt_in = {
            "cad/scripts/test-startup-readiness.py": "Rust diagnostic requiring a packaged site and Chrome",
            "scripts/test-drag-browser-consumer-controls.py": "Linux CI consumer fault controls",
            "scripts/test-drag-browser-deep-home-timeout.py": "Linux CI process cleanup control",
            "scripts/test-drag-browser-deep-home.py": "Linux CI deep HOME browser controls",
            "scripts/test-graphite-controls.py": "manual Graphite browser failure controls against a built site",
            "scripts/test-model-lookup-pilot.py": "opt-in model-lookup browser pilot controls",
            "scripts/test-model-lookup.cjs": "original Node fixture retained for parity comparisons",
            "scripts/test-ergogen-parity.cjs": "maintainer Node parity contract retained for comparison",
            "scripts/test-drag-project-poll.cjs": "historical Node drag runner helper controls",
        }
        self.assertEqual(scripts, registered | opt_in.keys())
        self.assertTrue(all(opt_in.values()))

    def assert_gate_contract(self, steps, workflow, gate, replacement_refs=None):
        self.assertTrue(steps["tooling"][0], "tooling must remain a default step")
        self.assertIn(gate, steps["tooling"][1], "required parity gate missing from tooling")
        if gate != ["node", "scripts/test-ergogen-parity.cjs"]:
            self.assertEqual(len(replacement_refs or []), 2, "replacement plan and evidence required")
            for ref in replacement_refs:
                self.assertTrue((ROOT / ref).is_file(), f"missing replacement reference: {ref}")

        # Recognize only a simple unconditional native job and a standalone run step.
        # Other YAML forms fail closed until this contract is deliberately extended.
        lines = workflow.splitlines()
        self.assertEqual(lines.count("jobs:"), 1)
        jobs = lines[lines.index("jobs:") + 1:]
        jobs = jobs[:next((i for i, line in enumerate(jobs) if line and not line[0].isspace() and not line.startswith("#")), len(jobs))]
        starts = [i for i, line in enumerate(jobs) if re.fullmatch(r"  native:\s*(?:#.*)?", line)]
        self.assertEqual(len(starts), 1, "native job missing or ambiguous")
        job = jobs[starts[0] + 1:]
        job = job[:next((i for i, line in enumerate(job) if re.match(r"  [^ #]", line)), len(job))]
        keys = [re.match(r"    ([a-zA-Z-]+):(?:\s|$)", line).group(1)
                for line in job if re.match(r"    ([a-zA-Z-]+):(?:\s|$)", line)]
        self.assertEqual(sorted(keys), ["runs-on", "steps"], "unsupported or conditional native job")
        self.assertEqual(sum(line == "    steps:" for line in job), 1)
        step_lines = job[job.index("    steps:") + 1:]
        groups = []
        for line in step_lines:
            if re.match(r"      - ", line):
                groups.append([line[8:]])
            elif groups and line.startswith("        "):
                groups[-1].append(line[8:])
        found = False
        for group in groups:
            # Only plain name/id and run fields are accepted; conditions, shell,
            # working directory and block scalars can change execution semantics.
            if not all(re.fullmatch(r"(?:name|id|run): [^\n]+", line) for line in group):
                continue
            runs = [line[5:] for line in group if line.startswith("run: ")]
            if len(runs) != 1:
                continue
            try:
                words = shlex.split(runs[0])
            except ValueError:
                continue
            if (len(words) == 6 and words[:2] == ["python3", "scripts/check.py"]
                    and set(words[2:]) == {"repo", "tooling", "lint", "test"}):
                found = True
        self.assertTrue(found, "CI must configure an unconditional tooling step")

    def test_ergogen_gate_transition_contract(self):
        workflow = (ROOT / ".github/workflows/check.yaml").read_text()
        gate = [check.PY, "-B", "scripts/ergogen-parity/test_generate.py", "--hermetic"]
        self.assert_gate_contract(check.STEPS, workflow, gate,
                                  ["docs/research/2026-10-22-ergogen-parity-fixture-pipeline.md",
                                   "docs/research/2026-10-29-ergogen-oracle-boundary.md"])
        self.assertIn([check.PY, "-B", "scripts/ergogen-parity/test_generate.py", "--source"],
                      check.STEPS["browser"][1])
        self.assertNotIn(["node", "scripts/test-ergogen-parity.cjs"], check.STEPS["tooling"][1])
        self.assertNotIn(["node", "--version"], check.STEPS["tooling"][1])

    def test_ergogen_gate_contract_rejects_retirement_and_accepts_documented_cutover(self):
        gate = ["node", "scripts/test-ergogen-parity.cjs"]
        workflow = (ROOT / ".github/workflows/check.yaml").read_text()
        for steps in ({"tooling": (True, [])}, {"tooling": (False, [gate])},
                      {"tooling": (True, []), "parity": (False, [gate])}):
            with self.subTest(steps=steps), self.assertRaises(AssertionError):
                self.assert_gate_contract(steps, workflow, gate)

        # These references describe work in progress, not approval of a cutover.
        replacement = ["python3", "scripts/ergogen-parity/sandbox_generate.py", "--check",
                       "--cache", "/tmp/ergogen-checkout", "--temp-parent", "/tmp/ergogen-run"]
        self.assert_gate_contract({"tooling": (True, [replacement])}, workflow, replacement,
                                  ["docs/plans/retro-agent-guardrails.md",
                                   "docs/research/2026-10-29-ergogen-oracle-boundary.md"])

    def test_gate_contract_structural_regressions(self):
        gate = ["node", "scripts/test-ergogen-parity.cjs"]
        steps = {"tooling": (True, [gate])}
        def workflow(job_extra="", step_extra="", command="python3 scripts/check.py repo tooling lint test"):
            return f"jobs:\n  native:\n    runs-on: ubuntu-latest\n{job_extra}    steps:\n      - name: checks\n{step_extra}        run: {command}\n"
        self.assert_gate_contract(steps, workflow(command="python3 scripts/check.py test lint tooling repo"), gate)
        # Configuration cannot guarantee execution after an earlier failed step.
        self.assert_gate_contract(steps, workflow().replace("      - name: checks",
                                  "      - run: exit 1\n      - name: checks"), gate)
        for ci in (workflow(job_extra="    if: false\n"), workflow(job_extra="    needs: other\n"),
                   workflow(step_extra="        if: false\n"),
                   workflow(command="python3 scripts/check.py repo lint test"),
                   workflow(command="echo python3 scripts/check.py repo tooling lint test"),
                   workflow(command="python3 scripts/check.py repo tooling lint test || true"),
                   workflow().replace("  native:", "  decoy:")):
            with self.subTest(ci=ci), self.assertRaises(AssertionError):
                self.assert_gate_contract(steps, ci, gate)

    def test_gate_contract_replacement_requires_references(self):
        workflow = (ROOT / ".github/workflows/check.yaml").read_text()
        replacement = ["python3", "scripts/ergogen-parity/sandbox_generate.py", "--check",
                       "--cache", "/tmp/ergogen-checkout", "--temp-parent", "/tmp/ergogen-run"]
        steps = {"tooling": (True, [replacement])}
        for refs in ([], ["docs/plans/retro-agent-guardrails.md", "missing.md"]):
            with self.assertRaises(AssertionError):
                self.assert_gate_contract(steps, workflow, replacement, replacement_refs=refs)
        self.assert_gate_contract(steps, workflow, replacement, replacement_refs=[
            "docs/plans/retro-agent-guardrails.md", "docs/research/2026-10-29-ergogen-oracle-boundary.md"])

    def test_default_commands_do_not_invoke_node_toolchain(self):
        forbidden = {"node", "npm", "npx", "pnpm", "yarn", "corepack"}
        for step in check.DEFAULT:
            for command in check.STEPS[step][1]:
                with self.subTest(step=step, command=command):
                    self.assertNotIn(Path(command[0]).name, forbidden)

    def test_model_lookup_runs_in_browser_not_node_tooling(self):
        self.assertNotIn(["node", "scripts/test-model-lookup.cjs"], check.STEPS["tooling"][1])
        self.assertIn([check.PY, "scripts/model-lookup-pilot.py", "--gate"], check.STEPS["browser"][1])
        self.assertIn([check.PY, "-B", "scripts/test-build-patched-wasm-bindgen.py"], check.STEPS["tooling"][1])

    def test_measurement_protocols_run_in_tooling_and_the_worker_protocol_in_the_provider_gate(self):
        commands = check.STEPS["tooling"][1]
        self.assertIn([check.PY, "-B", "cad/scripts/test-measure-cad.py"], commands)
        self.assertNotIn(["node", "cad/scripts/test-measure-cad-worker.cjs"], commands)
        self.assertIn([check.PY, "cad/scripts/test-cad-provider.py"], check.STEPS["browser"][1])
    def test_ci_web_tools_do_not_install_playwright(self):
        # e0b517115 retired Playwright from the web-tools action. The workflow
        # cutover (no-NODE_PATH browser step, drag-chrome provisioning) stays
        # with ticket 008's remaining hosted gate, so this tree keeps HEAD's
        # Chrome-for-Testing CI wiring; only the action retirement is asserted.
        action = (ROOT / ".github/actions/web-tools/action.yml").read_text()
        self.assertNotIn("playwright", action.lower())
        self.assertNotIn("NODE_PATH", action)

    def test_parts_inspector_launcher_requires_running_site_url(self):
        env = os.environ.copy()
        env.pop("BOARDSTUDIO_PREVIEW_URL", None)
        result = subprocess.run([sys.executable, "scripts/run-parts-inspector-scroll.py"], cwd=ROOT,
                                env=env, capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("BOARDSTUDIO_PREVIEW_URL", result.stderr)

    def test_parts_inspector_discovers_chrome_before_build_and_preserves_environment(self):
        runner = """
import json, os, runpy, sys
from pathlib import Path
sys.path.insert(0, 'scripts')
import browser_tools
def build(name):
    print('BUILD', name)
    return Path('/fake/parts-inspector-scroll')
def launch(binary, args):
    print('LAUNCH', json.dumps([binary, args, os.environ['CHROME_BIN'], os.environ['KEEP_ME'], os.environ['BOARDSTUDIO_PREVIEW_URL']]))
browser_tools._build_harness = build
os.execv = launch
runpy.run_path('scripts/run-parts-inspector-scroll.py', run_name='__main__')
"""
        with tempfile.TemporaryDirectory() as directory:
            chrome = Path(directory) / "chromium"
            chrome.write_text("#!/bin/sh\nexit 0\n")
            chrome.chmod(0o755)
            env = dict(os.environ, PATH=directory, BOARDSTUDIO_PREVIEW_URL="http://example.test/", KEEP_ME="kept")
            env.pop("CHROME_BIN", None)
            env.pop("BOARDSTUDIO_DRAG_CHROME_BIN", None)
            for override in (None, "/chosen/chrome"):
                with self.subTest(override=override):
                    selected = dict(env)
                    if override:
                        selected["CHROME_BIN"] = override
                    result = subprocess.run([sys.executable, "-c", runner], cwd=ROOT, env=selected,
                                            capture_output=True, text=True)
                    self.assertEqual(result.returncode, 0, result.stderr)
                    lines = result.stdout.splitlines()
                    self.assertEqual(lines[0], "BUILD parts-inspector-scroll")
                    binary, args, actual, kept, url = json.loads(lines[1].removeprefix("LAUNCH "))
                    self.assertEqual((binary, args), ("/fake/parts-inspector-scroll", ["/fake/parts-inspector-scroll"]))
                    self.assertEqual((actual, kept, url), (override or str(chrome), "kept", "http://example.test/"))
            chrome.unlink()
            result = subprocess.run([sys.executable, "-c", runner], cwd=ROOT, env=env,
                                    capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("CHROME_BIN", result.stderr)
            self.assertNotIn("Traceback", result.stderr)
            self.assertNotIn("BUILD", result.stdout)
            self.assertNotIn("LAUNCH", result.stdout)

    def test_parts_inspector_missing_build_prerequisites_exit_cleanly_before_launch(self):
        runner = """
import os, runpy, subprocess, sys
sys.path.insert(0, 'scripts')
def forbidden(*args, **kwargs):
    raise AssertionError('build or browser launched')
subprocess.run = forbidden
os.execv = forbidden
runpy.run_path('scripts/run-parts-inspector-scroll.py', run_name='__main__')
"""
        with tempfile.TemporaryDirectory() as directory:
            env = dict(os.environ, PATH=directory, HOME=directory,
                       BOARDSTUDIO_PREVIEW_URL="http://example.test/", CHROME_BIN="/chosen/chrome")
            env.pop("BOARDSTUDIO_BUILD_LOCK_FD", None)
            for missing in ("flock", "cargo"):
                with self.subTest(missing=missing):
                    if missing == "cargo":
                        (Path(directory) / "flock").symlink_to("/usr/bin/flock")
                    result = subprocess.run([sys.executable, "-c", runner], cwd=ROOT, env=env,
                                            capture_output=True, text=True)
                    self.assertNotEqual(result.returncode, 0)
                    self.assertIn(missing, result.stderr.lower())
                    self.assertIn("install", result.stderr.lower())
                    self.assertNotIn("Traceback", result.stderr)
                    self.assertNotIn("build or browser launched", result.stderr)

    def test_cad_capture_controls_have_a_human_runnable_entry_point(self):
        result = subprocess.run([sys.executable, "cad/scripts/test-cad-capture-controls.py", "--help"],
                                cwd=ROOT, capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("output", result.stdout.lower())

    def test_graphite_manual_runners_are_rust_with_historical_source_archived(self):
        for name in ("feedback", "usability"):
            self.assertTrue((ROOT / f"scripts/run-graphite-{name}.py").is_file())
            self.assertFalse((ROOT / f"scripts/test-graphite-{name}.cjs").exists())
            self.assertTrue((ROOT / f"docs/archive/graphite/test-graphite-{name}.cjs").is_file())

    def test_retired_drag_driver_is_rejected_before_build_or_launch(self):
        spec = importlib.util.spec_from_file_location("drag_browser", ROOT / "scripts/run-drag-browser.py")
        drag = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(drag)
        for value in ("node", "other", ""):
            with self.subTest(value=value), mock.patch.dict(os.environ, {"BOARDSTUDIO_DRAG_DRIVER": value}), \
                    mock.patch.object(check, "run_plan") as run_plan, \
                    mock.patch.object(drag.browser_tools, "build_drag_harness") as build:
                for steps in (["drag-smoke"], ["browser"], list(check.DEFAULT), ["build", "browser"]):
                    with contextlib.redirect_stderr(io.StringIO()) as error:
                        self.assertEqual(check.run(steps), 1)
                    self.assertIn("BOARDSTUDIO_DRAG_DRIVER", error.getvalue())
                with self.assertRaises(SystemExit) as raised:
                    drag.main()
                self.assertIn("BOARDSTUDIO_DRAG_DRIVER", str(raised.exception))
                run_plan.assert_not_called()
                build.assert_not_called()

    def test_browser_runs_packaged_provider_regressions(self):
        self.assertIn([check.PY, "cad/scripts/test-cad-provider.py"], check.STEPS["browser"][1])
        self.assertIn([check.PY, "cad/scripts/face-completeness-gate.py"], check.STEPS["browser"][1])
        self.assertIn([check.PY, "-B", "cad/scripts/test-face-completeness-web.py", "--hermetic"],
                      check.STEPS["tooling"][1])
        self.assertIn([check.PY, "-B", "cad/scripts/test-provider-pages.py", "--hermetic"],
                      check.STEPS["tooling"][1])
        self.assertIn([check.PY, "-B", "cad/scripts/test-cad-provider-gate.py"],
                      check.STEPS["tooling"][1])

    def test_browser_gate_runs_cad_capture_controls_with_the_shared_lock(self):
        for script in ("cad/scripts/test-cad-capture-controls.py", "cad/scripts/test-cold-responses.py",
                       "cad/scripts/face-completeness-gate.py", "cad/scripts/test-cad-provider.py"):
            command = [check.PY, script]
            self.assertIn(command, check.STEPS["browser"][1])
            with mock.patch.object(check.browser_tools, "_inherited_build_lock", return_value=9), \
                    mock.patch.object(check.subprocess, "run", return_value=mock.Mock(returncode=0)) as run, \
                    contextlib.redirect_stdout(io.StringIO()):
                check.run_serial(command, {"BOARDSTUDIO_BUILD_LOCK_FD": "9"})
            self.assertEqual(run.call_args.kwargs["env"]["BOARDSTUDIO_BUILD_LOCK_FD"], "9")
            self.assertEqual(run.call_args.kwargs["pass_fds"], (9,))

    def test_full_check_runs_the_documented_steps_in_order(self):
        self.assertEqual(check.DEFAULT, ("repo", "tooling", "lint", "build", "test", "browser"))
        self.assertNotIn("security", check.DEFAULT)
        self.assertNotIn("typecheck", check.DEFAULT)

    def test_lint_overlaps_harness_clippy_with_root_clippys(self):
        # Two parties: the harness (submitted after the first root clippy) blocks until the
        # second root clippy is in flight beside it.
        barrier = threading.Barrier(2, timeout=10)
        seen = []

        def fake_serial(command, environment, cwd, captured, outputs):
            text = " ".join(command)
            if check.is_harness_clippy(command) or "wasm32-unknown-unknown" in text:
                barrier.wait()
            seen.append(text)
            return 0.0, 0, "s", "e"

        commands = [c for c in check.LINT if c[:2] == ["cargo", "clippy"]]
        self.assertEqual(len(commands), 3)
        with mock.patch.object(check, "run_serial", side_effect=fake_serial):
            elapsed, passed = check._run_lint_overlap(commands, {}, check.ROOT, None)
        self.assertTrue(passed)
        roots = [text for text in seen if "--manifest-path" not in text]
        self.assertEqual(len(roots), 2)
        # Root clippys keep their plan order even though the harness runs beside them.
        self.assertLess(seen.index(roots[0]), seen.index(roots[1]))

    def test_lint_overlap_reports_harness_failure(self):
        def fake_serial(command, environment, cwd, captured, outputs):
            if check.is_harness_clippy(command):
                return 0.0, 1, "s", "e"
            return 0.0, 0, "s", "e"

        commands = [c for c in check.LINT if c[:2] == ["cargo", "clippy"]]
        with mock.patch.object(check, "run_serial", side_effect=fake_serial):
            elapsed, passed = check._run_lint_overlap(commands, {}, check.ROOT, None)
        self.assertFalse(passed)

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
        self.chrome.side_effect = check.browser_tools.MissingBrowserTool("missing browser tool")
        for names in (["browser"], ["drag-smoke"], list(check.DEFAULT), ["build", "browser"]):
            error = io.StringIO()
            with self.subTest(names=names), mock.patch.object(check, "run_plan") as run_plan, \
                    mock.patch.object(check.subprocess, "run") as launch, \
                    contextlib.redirect_stderr(error):
                self.assertEqual(check.run(names), 1)
            run_plan.assert_not_called()
            launch.assert_not_called()
            self.assertIn("check: missing browser tool", error.getvalue())

    def test_default_drag_driver_uses_rust_and_chrome_only(self):
        with mock.patch.dict(os.environ):
            os.environ.pop("BOARDSTUDIO_DRAG_DRIVER", None)
            with mock.patch.object(check, "run_plan", return_value=0):
                self.assertEqual(check.run(["drag-smoke"]), 0)
            self.chrome.assert_called_once_with()

    def test_browser_tools_are_probed_once_before_the_unchanged_command_order(self):
        calls = []
        self.chrome.side_effect = lambda: calls.append("probe") or "chrome"

        def record(plan, update_native_baseline=False):
            calls.append([name for name, _ in plan])
            return 0

        with mock.patch.object(check, "run_plan", side_effect=record):
            self.assertEqual(check.run(list(check.DEFAULT)), 0)
        self.assertEqual(calls, ["probe", list(check.DEFAULT)])
        self.chrome.assert_called_once_with()

    def test_scoped_browser_only_does_not_acquire_the_drag_smoke_prerequisites(self):
        self.chrome.side_effect = check.browser_tools.MissingBrowserTool("absent")
        with mock.patch.object(check.subprocess, "run", return_value=mock.Mock(returncode=0, stdout="")), \
                mock.patch.object(check, "_parallel_process", return_value=mock.Mock(returncode=0, stdout="")), \
                contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(check.main(["browser", "--only", "case"]), 0)
        self.chrome.assert_not_called()

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
                mock.patch.object(check, "_parallel_process", side_effect=lambda command, registry, **kwargs: fake(command, **kwargs)), \
                mock.patch.dict(os.environ, {"BOARDSTUDIO_CHECK_JOBS": "3"}), \
                mock.patch.dict(check.STEPS, {"browser": (True, [serial, *launches, serial])}), \
                contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(error):
            self.assertEqual(check.run(["browser"]), 1)
        self.assertIn("web/crates/case", error.getvalue())
        self.assertEqual(seen[0], serial)
        self.assertEqual(len(seen), 4, "the trailing serial command does not run after a failed batch")
        self.assertEqual(check.launch_order(launches[1]), 0, "heavy crates start first")
        self.assertEqual(check.launch_order(launches[0]), 1)

    def test_a_harness_timeout_before_any_test_output_fails_closed_on_one_launch(self):
        # Missing loader or test lines do not prove no test ran, so the stall is reported, never relaunched.
        stalled = ("Running headless tests in Chrome on `http://127.0.0.1:1/`\n"
                   + check.HARNESS_TIMEOUT_LINE + "\n")
        launches = [["wasm-pack", "test", "--headless", "--chrome", f"web/crates/{name}", "--locked", "--lib"]
                    for name in ("keymap", "ui-shared")]
        with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as config:
            json.dump({"goog:chromeOptions": {"binary": "/usr/bin/true"}}, config)
        self.addCleanup(Path(config.name).unlink)
        for jobs in ("3", "1"):
            with self.subTest(jobs=jobs):
                attempts, configs = [], []

                def fake(command, **kwargs):
                    attempts.append(launch := check.launch_crate(command))
                    configs.append(kwargs["env"].get("WASM_BINDGEN_TEST_WEBDRIVER_JSON"))
                    return mock.Mock(returncode=1 if launch == "ui-shared" else 0,
                                     stdout=stalled if launch == "ui-shared" else "test panels::a ... ok\n")

                receipt = mock.Mock()
                with mock.patch.object(check.subprocess, "run", side_effect=fake), \
                           mock.patch.object(check, "_parallel_process", side_effect=lambda command, registry, **kwargs: fake(command, **kwargs)), \
                        mock.patch.object(check, "_RECEIPT", receipt), \
                        mock.patch.dict(os.environ, {"BOARDSTUDIO_CHECK_JOBS": jobs,
                                                     "WASM_BINDGEN_TEST_WEBDRIVER_JSON": config.name, "CHROME_BIN": "/bin/true"}), \
                        mock.patch.dict(check.STEPS, {"browser": (True, launches)}), \
                        contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
                    self.assertEqual(check.run(["browser"]), 1)
                self.assertEqual(sorted(attempts), ["keymap", "ui-shared"], "no relaunch")
                self.assertEqual(configs, [config.name] * 2, "the caller's webdriver config is kept")
                observed = [call.args for call in receipt.observe.call_args_list
                            if check.launch_crate(call.args[1]) == "ui-shared"]
                self.assertEqual(len(observed), 1)
                self.assertEqual((observed[0][2], observed[0][4]), (1, stalled),
                                 "the receipt keeps the only attempt's exit code and whole output")
                self.assertEqual(check.summarize_wasm_failure(stalled)["kind"], "timeout")

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
                mock.patch.object(check, "_parallel_process", side_effect=lambda command, registry, **kwargs: fake(command, **kwargs)), \
                mock.patch.dict(check.STEPS, {"test": (True, commands)}), \
                contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(check.run(["test"]), 0)
        self.assertEqual(sorted(launched), sorted(commands))

        launched.clear()
        with mock.patch.object(check.subprocess, "run", side_effect=fake), \
                mock.patch.object(check, "_parallel_process", side_effect=lambda command, registry, **kwargs: fake(command, **kwargs)), \
                mock.patch.dict(os.environ, {"BOARDSTUDIO_CHECK_JOBS": "1"}), \
                mock.patch.dict(check.STEPS, {"test": (True, commands)}), \
                contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(check.run(["test"]), 0)
        self.assertEqual(launched, commands, "one job runs them in order")

    def test_parallel_heartbeat_tracks_started_and_completed_workers_before_blocked_first(self):
        first_started, second_started, third_started = (threading.Event() for _ in range(3))
        release_first, release_second, release_third = (threading.Event() for _ in range(3))
        second_output = threading.Event()

        class Output(io.StringIO):
            def write(self, text):
                written = super().write(text)
                if "second output" in text:
                    second_output.set()
                return written

        output = Output()
        result = []

        def fake(command, **kwargs):
            started, release = {
                "first": (first_started, release_first),
                "second": (second_started, release_second),
                "third": (third_started, release_third),
            }[command[0]]
            started.set()
            if not release.wait(5):
                raise TimeoutError(command[0])
            return mock.Mock(returncode=0, stdout=f"{command[0]} output\n")

        with mock.patch.object(check, "_parallel_process", side_effect=lambda command, registry, **kwargs: fake(command, **kwargs)), \
                mock.patch.dict(os.environ, {"BOARDSTUDIO_CHECK_JOBS": "2"}), \
                contextlib.redirect_stdout(output):
            worker = threading.Thread(target=lambda: result.append(check.run_parallel(
                [["first"], ["second"], ["third"]], os.environ.copy())))
            worker.start()
            try:
                self.assertTrue(first_started.wait(2))
                self.assertTrue(second_started.wait(2))
                self.assertNotIn("$ third", output.getvalue())
                release_second.set()
                self.assertTrue(third_started.wait(2))
                self.assertTrue(second_output.wait(2))
                text = output.getvalue()
                self.assertIn("exit 0 $ second", text)
                self.assertIn("$ third", text)
                self.assertNotIn("exit 0 $ first", text)
            finally:
                release_first.set()
                release_second.set()
                release_third.set()
                worker.join(5)
            self.assertFalse(worker.is_alive())
        self.assertEqual(result[0][1], [])
        for name in ("first", "second", "third"):
            self.assertIn(f"$ {name}\n{name} output\n", output.getvalue())

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
                mock.patch.object(check, "_parallel_process", side_effect=lambda command, registry, **kwargs: fake(command, **kwargs)), \
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
                mock.patch.object(check, "_parallel_process", side_effect=lambda command, registry, **kwargs: fake(command, **kwargs)), \
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
        self.assertIn("web/crates/library", stderr)
        self.assertIn("web/crates/case", stderr)
        self.assertIn("step 'browser (only: library, case, parts)' failed", stderr)

        # In a parallel batch with --keep-going, a failed crate launch continues to trailing
        # independent browser commands unless the page `--list` prerequisite itself fails.
        seen.clear()
        launches = [["wasm-pack", "test", "--headless", "--chrome", f"web/crates/{name}", "--locked", "--lib"]
                    for name in ("library", "case")]
        page_list = ["wasm-pack", "test", "--headless", "--chrome", "web", "--bin", "boardstudio-web", "--", "--list"]
        page_run = [sys.executable, "scripts/run-wasm-tests.py", "--all"]
        with mock.patch.object(check.subprocess, "run", side_effect=fake), \
                mock.patch.object(check, "_parallel_process", side_effect=lambda command, registry, **kwargs: fake(command, **kwargs)), \
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
                mock.patch.object(check, "_parallel_process", side_effect=lambda command, registry, **kwargs: fail_list(command, **kwargs)), \
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
