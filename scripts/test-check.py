#!/usr/bin/env python3
from __future__ import annotations

import contextlib
import importlib.util
import io
import json
import os
import re
import shlex
import signal
from pathlib import Path
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("check", Path(__file__).with_name("check.py"))
check = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(check)
CARGO_TARGET = check.cargo_target
import tailwind_tools  # noqa: E402  (check.py put the scripts directory on sys.path)
build_web = importlib.import_module("build-web")
TAILWIND = Path("/tools/tailwindcss")
TAILWIND_CSS = [str(TAILWIND), "-i", "tailwind.css", "-o", "assets/tailwind.css"]


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
        # Plans never probe the machine's Tailwind CLI; a test that does restores the real lookup.
        tailwind = mock.patch.object(check, "tailwind_cli", return_value=TAILWIND)
        self.tailwind = tailwind.start()
        self.addCleanup(tailwind.stop)
        # overlap_span probes every plain-page tool through build-web.py; keep those off the machine too.
        page_tailwind = mock.patch.object(build_web, "tailwind_cli", return_value=TAILWIND)
        self.page_tailwind = page_tailwind.start()
        self.addCleanup(page_tailwind.stop)
        page_bindgen = mock.patch.object(build_web, "locked_wasm_bindgen", return_value=Path("/tools/wasm-bindgen"))
        self.page_bindgen = page_bindgen.start()
        self.addCleanup(page_bindgen.stop)
        page_dx = mock.patch.object(build_web, "matching_dx", return_value="/tools/dx")
        self.page_dx = page_dx.start()
        self.addCleanup(page_dx.stop)
        check._PAGE_TEST_ARTIFACT = None
        self.addCleanup(setattr, check, "_PAGE_TEST_ARTIFACT", None)

    def fake_wasm_build(self, command):
        if not check.is_crate_build(command):
            return None
        lines = []
        for idx, arg in enumerate(command[:-1]):
            if arg != "-p":
                continue
            package = command[idx + 1]
            if package == check.PAGE_PACKAGE:
                binary = self.wasm_dir / "page.wasm"
                binary.touch()
                lines.append(json.dumps({
                    "reason": "compiler-artifact", "manifest_path": str(ROOT / "web/Cargo.toml"),
                    "profile": {"test": True}, "target": {"kind": ["bin"], "name": "boardstudio-web"},
                    "executable": str(binary),
                }))
                continue
            crate = package.removeprefix("boardstudio-web-")
            binary = self.wasm_dir / f"{crate}.wasm"
            binary.touch()
            lines.append(json.dumps({
                "reason": "compiler-artifact", "manifest_path": str(ROOT / "web/crates" / crate / "Cargo.toml"),
                "profile": {"test": True}, "target": {"kind": ["lib"]}, "executable": str(binary),
            }))
        return mock.Mock(returncode=0, stdout="\n".join(lines))

    def mock_wasm_dir(self):
        self.wasm_dir = Path(self.enterContext(tempfile.TemporaryDirectory()))
        runner = importlib.import_module("run-wasm-tests")
        self.enterContext(mock.patch.object(runner, "wasm_bindgen_dir", return_value=self.wasm_dir))
        self.enterContext(mock.patch.dict(os.environ, {"CHROME_BIN": "/bin/true", "CHROMEDRIVER": "/bin/true"}))

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
        self.assertIn(str(ROOT / "cad/target"), output.getvalue())
        self.assertNotIn(str(ROOT / "cad/wasm/target"), output.getvalue())

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
        self.mock_wasm_dir()
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
            self.assertEqual(launch.call_args.args[2]["BOARDSTUDIO_WASM_BINDGEN_DIR"], str(self.wasm_dir))

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
            "scripts/test-case-rest.py": "Case workspace journey against a checked server, with visual captures",
            "scripts/test-theme.py": "ticket 002 agent-browser Theme contract journey",
            "scripts/test-dock-primitives.py": "ticket 009 shared agent-browser dock test driver",
            "scripts/test-header.py": "ticket 010 agent-browser header journey",
            "scripts/test-dock-layout.py": "ticket 011 agent-browser dock layout journey",
            "scripts/test-command-palette.py": "ticket 012 agent-browser command palette journey",
            "scripts/test-toasts.py": "ticket 013 agent-browser Toast journey",
            "scripts/test-shell-glue.py": "ticket 014 agent-browser shell glue journey",
            "scripts/test-shell-position.py": "ticket 014 agent-browser shell positioning journey",
            "scripts/test-layout-toolbar.py": "ticket 015 agent-browser Layout toolbar journey",
            "scripts/test-canvas-layers.py": "ticket 016 agent-browser canvas layers journey",
            "scripts/test-layout-add-actions.py": "ticket 016 agent-browser Layout add actions journey",
            "scripts/test-layout-adoption.py": "ticket 016 agent-browser Layout drag and Undo journey",
            "scripts/test-layout-chooser-integration.py": "ticket 016 agent-browser Layout chooser journey",
            "scripts/test-layout-field-styles.py": "ticket 016 agent-browser Layout field styles journey",
            "scripts/test-layout-point-controls.py": "ticket 016 agent-browser Layout point controls journey",
            "scripts/test-layout-review-regressions.py": "ticket 016 agent-browser Layout review regressions",
            "scripts/test-layout-view-controls.py": "ticket 016 agent-browser Layout view controls journey",
            "scripts/test-case-settings.py": "ticket 017 agent-browser Case settings journey",
            "scripts/test-parts-library.py": "ticket 020 agent-browser Parts library journey",
            "scripts/test-parts-review-fixes.py": "ticket 020 agent-browser Parts review regressions",
            "scripts/test-keymap-layer-tabs.py": "trusted keyboard activation against a checked server",
            "scripts/test-library-export.py": "ticket 023 role/name workflow against a checked server with a coarse pointer",
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

    def test_stylesheet_uses_the_pinned_then_path_then_dx_installed_cli_before_the_page_build(self):
        def tool(path):
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("#!/bin/sh\necho 'tailwindcss v4.1.5'\n")
            path.chmod(0o755)
            return path

        def css(path):
            return [str(path), "-i", "tailwind.css", "-o", "assets/tailwind.css"]

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            pinned, on_path = tool(root / "pinned/tailwindcss"), tool(root / "bin/tailwindcss")
            installed = tool(root / "dx/tailwindcss-v4.1.5/tailwindcss")
            with mock.patch.object(check, "tailwind_cli", tailwind_tools.tailwind_cli), \
                    mock.patch.object(tailwind_tools, "DX_TOOLS", root / "dx"), \
                    mock.patch.dict(os.environ, {"PATH": str(root / "bin"),
                                                 "BOARDSTUDIO_TAILWINDCSS": str(pinned)}, clear=True):
                self.assertEqual(check.test_css_command(), css(pinned))
                del os.environ["BOARDSTUDIO_TAILWINDCSS"]
                self.assertEqual(check.test_css_command(), css(on_path))
                os.environ["PATH"] = str(root / "empty")
                self.assertEqual(check.test_css_command(), css(installed))
                with mock.patch.object(tailwind_tools, "DX_TOOLS", root / "none"):
                    self.assertEqual(check.test_css_command()[:3], ["dx", "build", "--web"])

    def test_pinned_tailwind_cli_trusts_an_explicit_executable_without_probing(self):
        with tempfile.TemporaryDirectory() as directory:
            pinned = Path(directory) / "tailwindcss"
            pinned.write_text("#!/bin/sh\nexit 3\n")
            pinned.chmod(0o755)
            with mock.patch.dict(os.environ, {"BOARDSTUDIO_TAILWINDCSS": str(pinned)}, clear=True):
                self.assertEqual(tailwind_tools.tailwind_cli(), pinned)

    def test_pinned_tailwind_cli_rejects_a_missing_or_unexecutable_pin(self):
        with tempfile.TemporaryDirectory() as directory:
            missing = Path(directory) / "absent"
            plain = Path(directory) / "plain"
            plain.write_text("not an executable\n")
            plain.chmod(0o644)
            for bad in (missing, plain):
                with mock.patch.dict(os.environ, {"BOARDSTUDIO_TAILWINDCSS": str(bad)}, clear=True):
                    with self.assertRaises(ValueError):
                        tailwind_tools.tailwind_cli()

    def test_hosted_web_suites_partition_the_local_browser_commands(self):
        packaged = check.STEPS["browser-packaged"][1]
        wasm = check.STEPS["browser-wasm"][1]
        self.assertCountEqual(packaged + wasm, check.STEPS["browser"][1])
        self.assertTrue(set(map(tuple, packaged)).isdisjoint(map(tuple, wasm)))
        self.assertIn(check.DRAG_SMOKE, packaged)
        build, *launches = wasm
        self.assertTrue(check.is_crate_build(build))
        self.assertEqual([build[idx + 1] for idx, arg in enumerate(build) if arg == "-p"],
                         list(check.WEB_WASM_PACKAGES[1:]))
        self.assertEqual(build[-2], "--features")
        self.assertEqual(build[-1].split(","),
                         ["boardstudio-web-host/page", "boardstudio-web-worker-host/cad"])
        scoped_build, scoped_launch = check.with_crate_build([check.crate_launch("layout")])
        self.assertEqual(scoped_build, [*check.WASM_CRATE_BUILD, "-p", "boardstudio-web-layout"])
        self.assertEqual(scoped_launch, check.crate_launch("layout"))
        worker_build, worker_launch = check.with_crate_build([check.crate_launch("worker-host")])
        self.assertEqual(worker_build, [*check.WASM_CRATE_BUILD, "-p", "boardstudio-web-worker-host",
                                        "--features", "boardstudio-web-worker-host/cad"])
        self.assertEqual(worker_launch, check.crate_launch("worker-host"))
        self.assertEqual(len(launches), 13)
        self.assertTrue(all(check.is_crate_launch(cmd) for cmd in launches))
        self.assertIn("scripts/run-wasm-tests.py", packaged[-1])
        self.assertFalse(any("--list" in cmd for cmd in packaged + wasm))
        self.assertIn([check.PY, "cad/scripts/test-cad-browser.py"], packaged)

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
        self.assertEqual(check.DEFAULT, ("repo", "tooling", "build", "lint", "test", "browser"))
        self.assertNotIn("security", check.DEFAULT)
        self.assertNotIn("typecheck", check.DEFAULT)

    def test_lint_has_generated_test_css_before_wasm_clippy(self):
        plans = []
        with mock.patch.object(check, "run_plan", side_effect=lambda plan, *args: plans.append(plan) or 0):
            self.assertEqual(check.run(["lint"]), 0)
            self.assertEqual(check.run(list(check.DEFAULT)), 0)
        self.assertEqual([name for name, _ in plans[0]], ["test-css", "lint"])
        full = [name for name, _ in plans[1]]
        self.assertLess(full.index("build"), full.index("lint"))
        self.assertNotIn("test-css", full)

    def test_css_dependent_browser_subsets_prepare_test_css(self):
        for crate in ("ui-shared", "keycaps", "case", "layout"):
            plans = []
            with self.subTest(crate=crate), \
                    mock.patch.object(check, "run_plan", side_effect=lambda plan, *args: plans.append(plan) or 0):
                self.assertEqual(check.run(["browser"], only=[crate]), 0)
                self.assertEqual([name for name, _ in plans[0]], ["test-css", f"browser (only: {crate})"])

    def test_root_clippy_uses_nonincremental_lint_profile(self):
        for command in check.LINT[1:3]:
            self.assertEqual(command[command.index("--profile") + 1], "lint")
            self.assertEqual(command[-3:], ["--", "-D", "warnings"])
        self.assertNotIn("--profile", check.LINT[4])

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
        self.assertEqual(captured["plan"][0][1][1:], check.BROWSER_PREBUILDS)

    def test_drag_smoke_can_run_as_a_fresh_build_preflight(self):
        captured = {}

        def record(plan, update_native_baseline=False):
            captured["plan"] = plan
            return 0

        with mock.patch.object(check, "run_plan", side_effect=record):
            self.assertEqual(check.run(["drag-smoke"]), 0)
        self.assertEqual([name for name, _ in captured["plan"]], ["build", "drag-smoke"])
        self.assertEqual(captured["plan"][-1][1], [check.DRAG_SMOKE])
        self.assertEqual(captured["plan"][0][1][1:], [check.HARNESS_BUILD])

    def test_browser_prebuild_failure_stops_before_other_artifacts(self):
        commands = check.STEPS["build"][1]

        def fail_second(command, environment, cwd, captured, outputs):
            outputs.append((command, ""))
            return 0.0, 7 if check.is_crate_build(command) else 0, "", ""

        with mock.patch.object(check, "check_jobs", return_value=1), \
             mock.patch.object(check, "run_serial", side_effect=fail_second) as run, \
             contextlib.redirect_stderr(io.StringIO()):
            self.assertFalse(check.run_commands("build", commands, {})[1])
        self.assertEqual([call.args[0] for call in run.call_args_list], commands[:2])

    def test_build_splits_page_staging_and_combines_the_wasm_prebuilds(self):
        commands = check.STEPS["build"][1]
        combined = check.combined_wasm_prebuild(check.BROWSER_CRATE_BUILD)
        self.assertIsNotNone(combined)
        stage = [check.PY, "scripts/build-web.py", "--fast", "--phase", "stage"]
        barrier = threading.Barrier(3, timeout=5)
        seen = []
        output = io.StringIO()

        def serial(command, environment, *args, **kwargs):
            self.assertEqual(command, [check.PY, "scripts/build-web.py", "--fast", "--phase", "cargo"])
            seen.append("cargo")
            return 0.0, 0, "", ""

        def launch(command, **kwargs):
            self.assertEqual(kwargs["env"]["CARGO_PROFILE_DEV_DEBUG"], "false")
            seen.append(command)
            barrier.wait()
            label = {tuple(stage): "stage", tuple(combined): "combined",
                     tuple(check.HARNESS_BUILD): "harness"}[tuple(command)]
            return subprocess.CompletedProcess(command, 0, stdout=f"payload {label}\n")

        def fake_artifacts(text, _cwd, _crates, *, stream=None):
            print(text, end="", file=stream)
            return {}

        with mock.patch.object(check, "check_jobs", return_value=2), \
             mock.patch.object(check, "plain_page_tools_available", return_value=True), \
             mock.patch.object(check, "run_serial", side_effect=serial), \
             mock.patch.object(check, "launch_command", side_effect=launch), \
             mock.patch.object(check, "crate_test_artifacts", side_effect=fake_artifacts), \
             mock.patch.object(check, "page_test_artifact", return_value=Path("/fake/page.wasm")), \
             contextlib.redirect_stdout(output):
            self.assertTrue(check.run_commands("build", commands, {})[1])
        self.assertEqual(seen[0], "cargo")
        self.assertCountEqual(seen[1:], [stage, combined, check.HARNESS_BUILD])
        self.assertNotIn(check.PAGE_TEST_BUILD, seen)
        self.assertEqual([line for line in output.getvalue().splitlines() if line.startswith("payload")],
                         ["payload stage", "payload combined", "payload harness"])

    def test_build_without_plain_page_tools_combines_prebuilds_after_a_serial_page(self):
        commands = check.STEPS["build"][1]
        combined = check.combined_wasm_prebuild(check.BROWSER_CRATE_BUILD)
        self.assertIsNotNone(combined)
        barrier = threading.Barrier(2, timeout=5)
        seen = []
        output = io.StringIO()

        def serial(command, environment, *args, **kwargs):
            self.assertEqual(command, commands[0])
            seen.append("page")
            return 0.0, 0, "", ""

        def launch(command, **kwargs):
            self.assertEqual(kwargs["env"]["CARGO_PROFILE_DEV_DEBUG"], "false")
            seen.append(command)
            barrier.wait()
            label = {tuple(combined): "combined", tuple(check.HARNESS_BUILD): "harness"}[tuple(command)]
            return subprocess.CompletedProcess(command, 0, stdout=f"payload {label}\n")

        def fake_artifacts(text, _cwd, _crates, *, stream=None):
            print(text, end="", file=stream)
            return {}

        with mock.patch.object(check, "check_jobs", return_value=2), \
             mock.patch.object(check, "plain_page_tools_available", return_value=False), \
             mock.patch.object(check, "run_serial", side_effect=serial), \
             mock.patch.object(check, "launch_command", side_effect=launch), \
             mock.patch.object(check, "crate_test_artifacts", side_effect=fake_artifacts), \
             mock.patch.object(check, "page_test_artifact", return_value=Path("/fake/page.wasm")), \
             contextlib.redirect_stdout(output):
            self.assertTrue(check.run_commands("build", commands, {})[1])
        self.assertEqual(seen[0], "page")
        self.assertCountEqual(seen[1:], [combined, check.HARNESS_BUILD])
        self.assertNotIn(check.PAGE_TEST_BUILD, seen)
        self.assertEqual([line for line in output.getvalue().splitlines() if line.startswith("payload")],
                         ["payload combined", "payload harness"])

    def test_build_keeps_serial_wasm_prebuilds_when_combining_rejects_drift(self):
        commands = check.STEPS["build"][1]
        stage = [check.PY, "scripts/build-web.py", "--fast", "--phase", "stage"]
        barrier = threading.Barrier(3, timeout=5)
        seen = []
        output = io.StringIO()

        def serial(command, environment, *args, **kwargs):
            self.assertEqual(command, [check.PY, "scripts/build-web.py", "--fast", "--phase", "cargo"])
            seen.append("cargo")
            return 0.0, 0, "", ""

        def launch(command, **kwargs):
            seen.append(command)
            if command in (stage, check.BROWSER_CRATE_BUILD, check.HARNESS_BUILD):
                barrier.wait()
            if command == check.PAGE_TEST_BUILD:
                self.assertIn(check.BROWSER_CRATE_BUILD, seen)
            label = "stage" if command == stage else commands.index(command)
            return subprocess.CompletedProcess(command, 0, stdout=f"payload {label}\n")

        def fake_artifacts(text, _cwd, _crates, *, stream=None):
            print(text, end="", file=stream)
            return {}

        with mock.patch.object(check, "check_jobs", return_value=2), \
             mock.patch.object(check, "plain_page_tools_available", return_value=True), \
             mock.patch.object(check, "combined_wasm_prebuild", return_value=None), \
             mock.patch.object(check, "run_serial", side_effect=serial), \
             mock.patch.object(check, "launch_command", side_effect=launch), \
             mock.patch.object(check, "crate_test_artifacts", side_effect=fake_artifacts), \
             contextlib.redirect_stdout(output):
            self.assertTrue(check.run_commands("build", commands, {})[1])
        self.assertEqual(seen[0], "cargo")
        self.assertCountEqual(seen[1:], [stage, *commands[1:]])
        self.assertEqual([line for line in output.getvalue().splitlines() if line.startswith("payload")],
                         ["payload stage", "payload 1", "payload 2", "payload 3"])

    def test_build_overlap_renders_diagnostics_without_raw_artifact_json(self):
        self.mock_wasm_dir()
        commands = check.STEPS["build"][1]
        combined = check.combined_wasm_prebuild(check.BROWSER_CRATE_BUILD)
        self.assertIsNotNone(combined)
        artifact_json = self.fake_wasm_build(combined).stdout
        diag_json = json.dumps({
            "reason": "compiler-message",
            "message": {"rendered": "warning: unused variable\n"},
        })

        def run_case(code):
            def launch(command, **kwargs):
                if check.is_crate_build(command):
                    return subprocess.CompletedProcess(
                        command, code,
                        stdout=f"Compiling boardstudio-web-layout\n{diag_json}\n{artifact_json}\n",
                    )
                label = "stage" if command[-1] == "stage" else "harness"
                return subprocess.CompletedProcess(command, 0, stdout=f"ok {label}\n")

            output = io.StringIO()
            errors = io.StringIO()
            with mock.patch.object(check, "check_jobs", return_value=2), \
                 mock.patch.object(check, "plain_page_tools_available", return_value=True), \
                 mock.patch.object(check, "run_serial", return_value=(0.0, 0, "", "")), \
                 mock.patch.object(check, "launch_command", side_effect=launch), \
                 contextlib.redirect_stdout(output), \
                 contextlib.redirect_stderr(errors):
                passed = check.run_commands("build", commands, {})[1]
            return passed, output.getvalue(), errors.getvalue()

        passed, out, err = run_case(0)
        self.assertTrue(passed, err)
        self.assertNotIn("compiler-artifact", out)
        self.assertIn("Compiling boardstudio-web-layout\nwarning: unused variable\n", out)
        self.assertLess(out.index("warning: unused variable"), out.index("ok harness"))
        self.assertEqual(check._PAGE_TEST_ARTIFACT, self.wasm_dir / "page.wasm")

        passed, out, err = run_case(101)
        self.assertFalse(passed)
        self.assertNotIn("compiler-artifact", out)
        self.assertIn("Compiling boardstudio-web-layout\nwarning: unused variable\n", out)
        self.assertIn("command failed", err)

    def test_build_overlap_reports_failure_and_skips_dependent_page(self):
        commands = check.STEPS["build"][1]
        seen = []

        def launch(command, **kwargs):
            seen.append(command)
            return subprocess.CompletedProcess(command, 0, stdout="invalid artifacts")

        with mock.patch.object(check, "check_jobs", return_value=2), \
             mock.patch.object(check, "plain_page_tools_available", return_value=True), \
             mock.patch.object(check, "run_serial", return_value=(0.0, 0, "", "")), \
             mock.patch.object(check, "launch_command", side_effect=launch), \
             mock.patch.object(check, "crate_test_artifacts", side_effect=ValueError("missing artifacts")), \
             contextlib.redirect_stdout(io.StringIO()) as output, \
             contextlib.redirect_stderr(io.StringIO()) as errors:
            self.assertFalse(check.run_commands("build", commands, {})[1])
        self.assertIn(check.HARNESS_BUILD, seen)
        self.assertNotIn(check.PAGE_TEST_BUILD, seen)
        self.assertIn("missing artifacts", output.getvalue())
        self.assertIn("command failed", errors.getvalue())

    def test_build_overlap_disabled_for_receipt_or_one_worker(self):
        commands = check.STEPS["build"][1]
        def serial_run(command, environment, cwd, captured, outputs, *args, **kwargs):
            outputs.append((command, ""))
            return 0.0, 0, "", ""

        for jobs, receipt in ((1, None), (2, mock.Mock())):
            with self.subTest(jobs=jobs), \
                 mock.patch.object(check, "check_jobs", return_value=jobs), \
                 mock.patch.object(check, "run_parallel") as parallel, \
                 mock.patch.object(check, "run_serial", side_effect=serial_run) as serial, \
                 mock.patch.object(check, "crate_test_artifacts", return_value={}):
                self.assertTrue(check.run_commands("build", commands, {}, receipt=receipt)[1])
            parallel.assert_not_called()
            self.assertEqual([call.args[0] for call in serial.call_args_list], commands)

    def test_build_single_command_keeps_full_serial_build_without_phase_split(self):
        commands = [[check.PY, "scripts/build-web.py", "--fast"]]

        def serial_run(command, environment, cwd, captured, outputs, *args, **kwargs):
            outputs.append((command, ""))
            return 0.0, 0, "", ""

        with mock.patch.object(check, "check_jobs", return_value=2), \
             mock.patch.object(check, "plain_page_tools_available", return_value=True), \
             mock.patch.object(check, "run_parallel") as parallel, \
             mock.patch.object(check, "run_serial", side_effect=serial_run) as serial:
            self.assertTrue(check.run_commands("build", commands, {})[1])
        parallel.assert_not_called()
        self.assertEqual([call.args[0] for call in serial.call_args_list], commands)

    def test_combined_wasm_prebuild_merges_crate_and_page_test_builds(self):
        combined = check.combined_wasm_prebuild(check.BROWSER_CRATE_BUILD)
        self.assertTrue(check.is_crate_build(combined))
        self.assertEqual(combined[:len(check.WASM_CRATE_BUILD)], check.WASM_CRATE_BUILD)
        rest = combined[len(check.WASM_CRATE_BUILD):]
        browser_rest = check.BROWSER_CRATE_BUILD[len(check.WASM_CRATE_BUILD):]
        browser_packages = [browser_rest[index + 1]
                            for index, token in enumerate(browser_rest[:-1]) if token == "-p"]
        packages = [rest[index + 1] for index, token in enumerate(rest[:-1]) if token == "-p"]
        self.assertEqual(packages, [*browser_packages, check.PAGE_PACKAGE])
        self.assertEqual([rest[index + 1] for index, token in enumerate(rest[:-1]) if token == "--bin"],
                         [check.PAGE_PACKAGE])
        self.assertEqual(rest.count("--lib"), 0, "--lib lives in the shared prefix")
        self.assertIn("--no-default-features", rest)
        features = [rest[index + 1] for index, token in enumerate(rest[:-1]) if token == "--features"]
        self.assertEqual(features, ["boardstudio-web-host/page,boardstudio-web-worker-host/cad,"
                                    "boardstudio-web/page,"
                                    "boardstudio-web-host/test-support"])
        self.assertEqual(rest[-7:], ["-p", check.PAGE_PACKAGE, "--bin", check.PAGE_PACKAGE,
                                    "--no-default-features", "--features", features[0]])

    def test_combined_wasm_prebuild_rejects_subset_and_drifted_flags(self):
        import sys
        import types
        subset = check.with_crate_build([check.crate_launch("layout")])[0]
        self.assertIsNone(check.combined_wasm_prebuild(subset))
        self.assertIsNone(check.combined_wasm_prebuild([*check.BROWSER_CRATE_BUILD, "--extra"]))

        real = importlib.import_module("run-wasm-tests").PAGE_BUILD

        def edited(*edits):
            page = list(real)
            for edit in edits:
                edit(page)
            return page

        def replace(old, new):
            def edit(page):
                page[page.index(old)] = new
            return edit

        def drop(option, takes_value=True):
            def edit(page):
                start = page.index(option)
                del page[start:start + (2 if takes_value else 1)]
            return edit

        def repackage(page):
            page[page.index("-p") + 1] = "boardstudio-web-layout"

        cases = {
            "profile": edited(replace("wasm-check", "dev")),
            "target": edited(replace("wasm32-unknown-unknown", "x86_64-unknown-linux-gnu")),
            "message-format": edited(replace("--message-format=json", "--message-format=short")),
            "unlocked": edited(drop("--locked", takes_value=False)),
            "default-features": edited(drop("--no-default-features", takes_value=False)),
            "extra-target": edited(lambda page: page.append("--lib")),
            "other-package": edited(repackage),
            "missing-bin": edited(drop("--bin")),
            "missing-features": edited(drop("--features")),
        }
        for name, page in cases.items():
            with self.subTest(name=name):
                fake = types.SimpleNamespace(PAGE_BUILD=page)
                with mock.patch.dict(sys.modules, {"run-wasm-tests": fake}):
                    self.assertIsNone(check.combined_wasm_prebuild(check.BROWSER_CRATE_BUILD))

    def test_page_test_artifact_requires_one_existing_bin_artifact(self):
        self.mock_wasm_dir()
        wasm = self.wasm_dir / "page.wasm"
        wasm.touch()

        def artifact(**changes):
            message = {"reason": "compiler-artifact", "profile": {"test": True},
                       "target": {"kind": ["bin"], "name": "boardstudio-web"},
                       "manifest_path": str(ROOT / "web/Cargo.toml"), "executable": str(wasm)}
            message.update(changes)
            return json.dumps(message)

        lib = json.dumps({"reason": "compiler-artifact", "profile": {"test": True},
                          "target": {"kind": ["lib"], "name": "boardstudio_web_layout"},
                          "manifest_path": str(ROOT / "web/crates/layout/Cargo.toml"),
                          "executable": str(self.wasm_dir / "layout.wasm")})
        self.assertEqual(check.page_test_artifact(f"Compiling page\n{lib}\n{artifact()}\n", ROOT), wasm)
        for lines in ([lib], [artifact(), artifact()],
                      [artifact(executable=str(self.wasm_dir / "missing.wasm"))],
                      [artifact(manifest_path=str(ROOT / "web/crates/layout/Cargo.toml"))],
                      [artifact(profile={"test": False})],
                      [artifact(target={"kind": ["bin"], "name": "other"})]):
            with self.subTest(lines=lines), self.assertRaises(ValueError):
                check.page_test_artifact("\n".join(lines), ROOT)

    def test_page_test_artifact_env_matches_the_wasm_runner(self):
        runner = importlib.import_module("run-wasm-tests")
        self.assertEqual(check.PAGE_TEST_ARTIFACT_ENV, runner.PAGE_ARTIFACT)

    def test_browser_step_reuses_the_combined_prebuild_page_artifact(self):
        page_wasm = Path("/fake/combined-page.wasm")
        seen_env = {}

        def fake_run_commands(name, commands, environment, *args, **kwargs):
            if name == "build":
                check._PAGE_TEST_ARTIFACT = page_wasm
                return 0.0, True
            seen_env[name] = dict(environment)
            return 0.0, True

        def run_plan(plan):
            with mock.patch.object(check, "run_commands", side_effect=fake_run_commands), \
                 mock.patch.object(check.wasm_bindgen_tools, "configure",
                                   side_effect=lambda env, root: env), \
                 contextlib.redirect_stdout(io.StringIO()):
                return check._run_plan(plan)

        plan = [("build", check.STEPS["build"][1]), ("browser", [check.PAGE_SUITE])]
        self.assertEqual(run_plan(plan), 0)
        self.assertEqual(seen_env["browser"][check.PAGE_TEST_ARTIFACT_ENV], str(page_wasm))
        # A later plan without a combined prebuild does not reuse a stale artifact.
        seen_env.clear()
        self.assertEqual(run_plan([("browser", [check.PAGE_SUITE])]), 0)
        self.assertNotIn(check.PAGE_TEST_ARTIFACT_ENV, seen_env["browser"])

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
        self.mock_wasm_dir()

        def fake(command, **kwargs):
            return self.fake_wasm_build(command) or mock.Mock(returncode=0, stdout="")

        with mock.patch.object(check.subprocess, "run", side_effect=fake), \
                mock.patch.object(check, "_parallel_process", return_value=mock.Mock(returncode=0, stdout="")), \
                contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(check.main(["browser", "--only", "case"]), 0)
        self.chrome.assert_not_called()

    def test_browser_gate_selects_page_and_worker_host_suites_without_test_filters(self):
        launches = check.select_browser_crate_launches(["host", "worker-host"])
        self.assertEqual(launches, [check.crate_launch("host"), check.crate_launch("worker-host")])


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

    def test_tooling_batches_keep_plan_order_failures_receipts_and_lock_scope(self):
        first = [check.PY, "first.py"]
        second = check.PAGE_TEST_BUILD
        serial = [check.PY, "scripts/test-serve-web.py"]
        trailing = [check.PY, "trailing.py"]
        commands = [first, second, serial, trailing]
        output, errors = io.StringIO(), io.StringIO()
        with tempfile.TemporaryDirectory() as directory:
            cwd = Path(directory)
            env = {"BOARDSTUDIO_BUILD_LOCK_FD": "9"}
            together = threading.Barrier(2)

            def launch(command, *, registry=None, **kwargs):
                self.assertEqual(kwargs["cwd"], cwd)
                self.assertEqual(kwargs.get("pass_fds"), (9,) if command == second else None)
                self.assertEqual(kwargs["env"].get("BOARDSTUDIO_BUILD_LOCK_FD"),
                                 "9" if command == second else None)
                self.assertEqual(registry is None, command == serial)
                if command in (first, second):
                    together.wait(timeout=5)
                label = f"output for {command[1]}"
                code = int(command != second)
                if registry is None and "stdout" not in kwargs:
                    print(label)
                return subprocess.CompletedProcess(command, code, stdout=label + "\n")

            collector = mock.Mock()
            with mock.patch.object(check, "check_jobs", return_value=2), \
                    mock.patch.object(check.browser_tools, "_inherited_build_lock", return_value=9), \
                    mock.patch.object(check, "launch_command", side_effect=launch), \
                    contextlib.redirect_stdout(output), contextlib.redirect_stderr(errors):
                self.assertFalse(check.run_commands("tooling", commands, env, cwd, receipt=collector)[1])

        self.assertEqual([call.args[1] for call in collector.observe.call_args_list], commands)
        command_lines = [line for line in output.getvalue().splitlines() if line.startswith("$ ")]
        self.assertEqual(command_lines, ["$ " + shlex.join(command) for command in commands])
        payload_lines = [line for line in output.getvalue().splitlines() if line.startswith("output for ")]
        self.assertEqual(payload_lines, [f"output for {command[1]}" for command in commands])
        self.assertEqual(errors.getvalue().splitlines(),
                         ["check: command failed: " + shlex.join(command) for command in (first, serial, trailing)])

    def test_one_tooling_worker_runs_commands_serially(self):
        commands = [[check.PY, "first.py"], [check.PY, "second.py"]]
        with mock.patch.object(check, "check_jobs", return_value=1), \
                mock.patch.object(check, "run_parallel") as parallel, \
                mock.patch.object(check, "run_serial", return_value=(0.0, 0, "", "")) as serial:
            self.assertTrue(check.run_commands("tooling", commands, {})[1])
        parallel.assert_not_called()
        self.assertEqual([call.args[0] for call in serial.call_args_list], commands)

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
        self.mock_wasm_dir()
        build, *launches = check.with_crate_build([check.crate_launch(name) for name in ("keymap", "layout", "case")])
        serial = [sys.executable, "-c", "pass"]
        seen = []

        def fake(command, **kwargs):
            seen.append(command)
            if result := self.fake_wasm_build(command):
                return result
            code = 1 if kwargs["cwd"] == ROOT / "web/crates/case" else 0
            return mock.Mock(returncode=code, stdout="output\n")

        error = io.StringIO()
        with mock.patch.object(check.subprocess, "run", side_effect=fake), \
                mock.patch.object(check, "_parallel_process", side_effect=lambda command, registry, **kwargs: fake(command, **kwargs)), \
                mock.patch.dict(os.environ, {"BOARDSTUDIO_CHECK_JOBS": "3"}), \
                mock.patch.dict(check.STEPS, {"browser": (True, [serial, build, *launches, serial])}), \
                contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(error):
            self.assertEqual(check.run(["browser"]), 1)
        self.assertIn("web/crates/case", error.getvalue())
        self.assertEqual(seen[0], serial)
        self.assertEqual(seen[1], build)
        self.assertEqual(len(seen), 5, "the trailing serial command does not run after a failed batch")
        self.assertEqual(check.launch_order(launches[1]), 0, "heavy crates start first")
        self.assertEqual(check.launch_order(launches[0]), 1)

    def test_a_harness_timeout_before_any_test_output_fails_closed_on_one_launch(self):
        # Missing loader or test lines do not prove no test ran, so the stall is reported, never relaunched.
        stalled = ("Running headless tests in Chrome on `http://127.0.0.1:1/`\n"
                   + check.HARNESS_TIMEOUT_LINE + "\n")
        self.mock_wasm_dir()
        launches = check.with_crate_build([check.crate_launch(name) for name in ("keymap", "ui-shared")])
        with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as config:
            json.dump({"pageLoadStrategy": "normal", "goog:chromeOptions": {"binary": "/usr/bin/true"}}, config)
        self.addCleanup(Path(config.name).unlink)
        for jobs in ("3", "1"):
            with self.subTest(jobs=jobs):
                attempts, configs = [], []

                def fake(command, **kwargs):
                    if result := self.fake_wasm_build(command):
                        return result
                    attempts.append(launch := kwargs["cwd"].name)
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

    def test_the_browser_job_cap_defaults_to_available_cpus_up_to_six(self):
        with mock.patch.dict(os.environ), mock.patch.object(check.os, "cpu_count", return_value=12):
            os.environ.pop("BOARDSTUDIO_CHECK_JOBS", None)
            self.assertEqual(check.check_jobs(), 6)
            os.environ["BOARDSTUDIO_CHECK_JOBS"] = "junk"
            self.assertEqual(check.check_jobs(), 6)
            os.environ["BOARDSTUDIO_CHECK_JOBS"] = "0"
            self.assertEqual(check.check_jobs(), 1)
            os.environ.pop("BOARDSTUDIO_CHECK_JOBS")
            with mock.patch.object(check.os, "cpu_count", return_value=4):
                self.assertEqual(check.check_jobs(), 4)
            with mock.patch.object(check.os, "cpu_count", return_value=None):
                self.assertEqual(check.check_jobs(), 4)

    def test_harness_clippy_overlaps_root_clippys_after_formats(self):
        import threading
        started = threading.Barrier(2, timeout=5)
        seen = []

        def run(command, *args, **kwargs):
            seen.append(command)
            if command[:2] == ["cargo", "clippy"] and "--target" not in command:
                started.wait()
            if check.is_harness_clippy(command):
                kwargs["outputs"].append((command, "harness diagnostics"))
                return 0.0, 1, "", ""
            return 0.0, 0, "", ""

        with mock.patch.object(check, "run_serial", side_effect=run), \
             mock.patch.object(check, "check_jobs", return_value=2), \
             contextlib.redirect_stdout(io.StringIO()) as stdout, contextlib.redirect_stderr(io.StringIO()):
            self.assertFalse(check.run_commands("lint", check.LINT, {})[1])
        self.assertEqual(seen[:2], [command for command in check.LINT if command[:2] == ["cargo", "fmt"]])
        self.assertEqual([command for command in seen if "--all-targets" in command
                          and not check.is_harness_clippy(command)], check.LINT[1:3])
        self.assertIn("harness diagnostics", stdout.getvalue())

    def test_overlap_requires_complete_step_commands_cpu_budget_and_no_receipt_or_ci(self):
        plan = [(name, check.STEPS[name][1]) for name in check.DEFAULT]
        with mock.patch.dict(os.environ, {"BOARDSTUDIO_CHECK_JOBS": "2"}, clear=True), \
             mock.patch.object(check.os, "cpu_count", return_value=8):
            self.assertEqual(check.overlap_span(plan, None), (2, 5))
            self.assertIsNone(check.overlap_span(plan[:4], None))
            self.assertIsNone(check.overlap_span(plan, mock.Mock()))
            with mock.patch.dict(os.environ, {"CI": "true"}):
                self.assertIsNone(check.overlap_span(plan, None))
            with mock.patch.dict(os.environ, {"BOARDSTUDIO_CHECK_JOBS": "1"}):
                self.assertIsNone(check.overlap_span(plan, None))
            with mock.patch.object(check.os, "cpu_count", return_value=7):
                self.assertIsNone(check.overlap_span(plan, None))
            # Without the standalone CLI, `lint` generates the stylesheet after `build` instead.
            self.page_tailwind.return_value = None
            self.assertIsNone(check.overlap_span(plan, None))

    def test_overlap_requires_every_plain_page_tool_not_just_tailwind(self):
        plan = [(name, check.STEPS[name][1]) for name in check.DEFAULT]
        with mock.patch.dict(os.environ, {"BOARDSTUDIO_CHECK_JOBS": "2"}, clear=True), \
             mock.patch.object(check.os, "cpu_count", return_value=8):
            self.assertEqual(check.overlap_span(plan, None), (2, 5))
            # Standalone Tailwind alone still falls back to `dx build`, which would
            # regenerate the stylesheet WASM Clippy is reading.
            self.page_bindgen.return_value = None
            self.assertIsNone(check.overlap_span(plan, None))
            self.page_bindgen.return_value = Path("/tools/wasm-bindgen")
            self.page_dx.return_value = None
            self.assertIsNone(check.overlap_span(plan, None))

    def test_overlapped_steps_get_one_fifo_and_only_test_gets_the_baseline_flag(self):
        group = [(name, check.STEPS[name][1]) for name in ("build", "lint", "test")]

        def run(batch, environment, *, invocations, in_order, elapsed_by_command):
            self.assertTrue(in_order)
            self.assertEqual([command[2] for command in batch], ["build", "lint", "test"])
            self.assertEqual([command[3:] for command in batch], [[], [], ["--update-native-baseline"]])
            configs = [invocations[tuple(command)][2] for command in batch]
            self.assertEqual([config.get("CARGO_TARGET_DIR") for config in configs],
                             [None, str(ROOT / "target/lint"), None])
            self.assertEqual(len({config["MAKEFLAGS"] for config in configs}), 1)
            self.assertEqual([config[check.IN_OVERLAP_ENV] for config in configs], ["1"] * 3)
            self.assertEqual(css, [(TAILWIND_CSS, ROOT / "web")], "the stylesheet is written before the steps start")
            fifo = Path(configs[0]["MAKEFLAGS"].split("fifo:", 1)[1])
            self.assertTrue(fifo.is_fifo())
            fd = os.open(fifo, os.O_RDONLY | os.O_NONBLOCK)
            try:
                self.assertEqual(os.read(fd, 20), b"+" * 7)
            finally:
                os.close(fd)
            for seconds, command in enumerate(batch, 1):
                elapsed_by_command[tuple(command)] = float(seconds)
            return 3.0, batch[1:]

        css = []

        def serial(command, environment, cwd):
            css.append((command, cwd))
            return 0.5, 0, "", ""

        with mock.patch.object(check, "run_parallel", side_effect=run), \
             mock.patch.object(check, "run_serial", side_effect=serial), contextlib.redirect_stdout(io.StringIO()), \
             mock.patch.dict(os.environ, {}, clear=True), mock.patch.object(check.os, "cpu_count", return_value=8):
            self.assertEqual(check.run_overlapped_steps(group, True),
                             ({"test-css": 0.5, "build": 1.0, "lint": 2.0, "test": 3.0}, ["lint", "test"]))

    def test_a_failed_stylesheet_stops_the_overlapped_steps_before_they_start(self):
        group = [(name, check.STEPS[name][1]) for name in ("build", "lint", "test")]
        with mock.patch.object(check, "run_parallel") as parallel, \
             mock.patch.object(check, "run_serial", return_value=(0.2, 1, "", "")), \
             contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(check.run_overlapped_steps(group, False), ({"test-css": 0.2}, ["test-css"]))
        parallel.assert_not_called()

    def test_overlapped_children_skip_lint_css_and_share_the_outer_process_group(self):
        plans = []
        with mock.patch.dict(os.environ, {check.IN_OVERLAP_ENV: "1"}), \
                mock.patch.object(check, "run_plan", side_effect=lambda plan, *args: plans.append(plan) or 0):
            self.assertEqual(check.run(["lint"]), 0)
            self.assertEqual(check.run(["browser-wasm"]), 0)
        self.assertEqual([name for name, _ in plans[0]], ["lint"])
        self.assertEqual([name for name, _ in plans[1]], ["test-css", "browser-wasm"])
        for inside in (False, True):
            sessions = []

            def popen(command, **kwargs):
                sessions.append(kwargs["start_new_session"])
                raise OSError("not launched")

            with self.subTest(inside=inside), mock.patch.dict(os.environ), \
                    mock.patch.object(check.subprocess, "Popen", side_effect=popen), \
                    contextlib.redirect_stdout(io.StringIO()):
                os.environ.pop(check.IN_OVERLAP_ENV, None)
                if inside:
                    os.environ[check.IN_OVERLAP_ENV] = "1"
                check.run_parallel([["one"]], {}, workers=1)
            self.assertEqual(sessions, [not inside])

    def test_parallel_children_lead_their_own_group_outside_the_step_group(self):
        # The CAD capture runner keeps cad-capture inside its group exactly then, so the
        # scheduler's killpg reaches both; inside the step group it detaches instead.
        probe = [sys.executable, "-c", "import os; print(os.getpgrp() == os.getpid())"]
        for inside, expected in ((False, "True"), (True, "False")):
            with self.subTest(inside=inside), mock.patch.dict(os.environ), \
                    contextlib.redirect_stdout(io.StringIO()):
                os.environ.pop(check.IN_OVERLAP_ENV, None)
                if inside:
                    os.environ[check.IN_OVERLAP_ENV] = "1"
                outputs = []
                check.run_parallel([probe], {}, outputs=outputs, workers=1)
            self.assertEqual([text.strip() for _, text in outputs], [expected])

    def test_build_is_fast_and_release_is_opt_in(self):
        self.assertIn("--fast", check.STEPS["build"][1][0])
        self.assertNotIn("--fast", check.STEPS["release"][1][0])
        self.assertNotIn("release", check.DEFAULT)

    def test_test_css_build_runs_from_the_web_project(self):
        command = ["dx", "build"]
        with mock.patch.object(check.subprocess, "run", return_value=mock.Mock(returncode=0)) as run, \
                contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(check.run_plan([("test-css", [command])]), 0)
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

    def test_parts_importer_follows_workspace_tests_while_cad_tests_run_beside_them(self):
        commands = check.STEPS["test"][1]
        cad = [check.PY, "cad/scripts/test-cad.py"]
        self.assertEqual(commands, [*check.CARGO_TESTS, cad, check.KICAD_PARTS_TEST])
        for workspace_code in (0, 101):
            events, together = [], threading.Barrier(2, timeout=5)
            lock = threading.Lock()

            def launch(command, *, registry=None, **kwargs):
                self.assertIsNotNone(registry)
                with lock:
                    events.append(("start", command))
                if command in (check.CARGO_TESTS[0], cad):
                    together.wait()
                with lock:
                    events.append(("end", command))
                code = workspace_code if command == check.CARGO_TESTS[0] else 3 if command == check.KICAD_PARTS_TEST else 0
                return subprocess.CompletedProcess(command, code, stdout=f"output of {command[-1]}\n")

            captured, receipt = [], mock.Mock()
            with self.subTest(workspace_code=workspace_code), \
                    mock.patch.object(check, "check_jobs", return_value=6), \
                    mock.patch.object(check, "launch_command", side_effect=launch), \
                    contextlib.redirect_stdout(io.StringIO()) as output, contextlib.redirect_stderr(io.StringIO()) as errors:
                self.assertFalse(check.run_commands("test", commands, {}, captured=captured, receipt=receipt)[1])
            self.assertLess(events.index(("end", check.CARGO_TESTS[0])), events.index(("start", check.KICAD_PARTS_TEST)),
                            "the importer waits for the workspace tests even when they fail")
            self.assertEqual([command for command, _ in captured], check.CARGO_TESTS)
            self.assertCountEqual([call.args[1] for call in receipt.observe.call_args_list], commands)
            self.assertIn("$ " + shlex.join(check.KICAD_PARTS_TEST) + "\noutput of scripts/test-import-kicad-parts.py",
                          output.getvalue())
            failed = [check.KICAD_PARTS_TEST] + ([check.CARGO_TESTS[0]] if workspace_code else [])
            self.assertCountEqual(errors.getvalue().splitlines(),
                                  ["check: command failed: " + shlex.join(command) for command in failed])

    def test_parallel_followers_are_emitted_and_stop_at_a_failure_unless_told_to_continue(self):
        head, first, second = ["head"], ["first"], ["second"]
        for in_order in (False, True):
            for continue_on_failure, expected in ((False, [head, first]), (True, [head, first, second])):
                launched, outputs, receipt = [], [], mock.Mock()

                def launch(command, *, registry=None, **kwargs):
                    launched.append(command)
                    return subprocess.CompletedProcess(command, int(command == first), stdout=f"{command[0]} out\n")

                with self.subTest(in_order=in_order, continue_on_failure=continue_on_failure), \
                        mock.patch.object(check, "launch_command", side_effect=launch), \
                        contextlib.redirect_stdout(io.StringIO()) as output:
                    _, failed = check.run_parallel([head, first, second], {}, outputs=outputs, receipt=receipt,
                                                   workers=2, in_order=in_order,
                                                   sequential_after={tuple(head): [first, second]},
                                                   continue_on_failure=continue_on_failure)
                self.assertEqual(launched, expected)
                self.assertEqual(failed, [first])
                self.assertEqual([command for command, _ in outputs], expected)
                self.assertEqual([call.args[1] for call in receipt.observe.call_args_list], expected)
                for command in expected:
                    self.assertIn(f"$ {command[0]}\n{command[0]} out\n", output.getvalue())

    def browser_batch(self, commands, *, keep_going=False, fail=()):
        """Run `commands` as the `browser` step with six jobs; return the launch events and page config."""
        events, configs, lock = [], {}, threading.Lock()
        # With BOARDSTUDIO_CHECK_JOBS=6 and 2 page workers, five outer lanes start together.
        first_wave = {tuple(check.DRAG_SMOKE), tuple(check.CAD_BROWSER), tuple(check.PAGE_SUITE),
                      *(tuple(check.crate_launch(crate)) for crate in ("case", "parts"))
                      }.intersection(map(tuple, commands))
        concurrent = threading.Barrier(len(first_wave), timeout=5)

        def launch(command, *, registry=None, **kwargs):
            self.assertIsNotNone(registry, f"{command} ran outside the batch")
            with lock:
                events.append(("start", command))
                configs[tuple(command)] = kwargs["env"]
            if tuple(command) in first_wave:
                concurrent.wait()
            with lock:
                events.append(("end", command))
            return subprocess.CompletedProcess(command, int(command in fail), stdout=f"ran {command[-1]}\n")

        with mock.patch.object(check, "check_jobs", return_value=6), \
                mock.patch.object(check, "launch_command", side_effect=launch), \
                contextlib.redirect_stdout(io.StringIO()) as output, contextlib.redirect_stderr(io.StringIO()) as errors:
            passed = check.run_commands("browser", commands, {}, keep_going=keep_going)[1]
        return passed, events, configs, output.getvalue(), errors.getvalue()

    def test_packaged_checks_run_beside_crate_launches_and_harness_builds_never_overlap(self):
        commands = [command for command in check.STEPS["browser"][1] if not check.is_crate_build(command)]
        passed, events, configs, output, _ = self.browser_batch(commands)
        self.assertTrue(passed)
        self.assertCountEqual([command for kind, command in events if kind == "start"], commands)
        self.assertEqual(configs[tuple(check.PAGE_SUITE)]["BOARDSTUDIO_CHECK_JOBS"], "2")
        # The CAD browser check heads one lane, and each harness check starts after the previous ends.
        lane = [check.CAD_BROWSER, *check.HARNESS_CHECKS]
        for before, after in zip(lane, lane[1:]):
            self.assertLess(events.index(("end", before)), events.index(("start", after)))
        for command in commands:
            self.assertIn("$ " + shlex.join(command) + f"\nran {command[-1]}\n", output)

    def test_packaged_lane_failures_stop_the_harness_chain_unless_keep_going(self):
        commands = check.STEPS["browser-packaged"][1]
        self.assertEqual(commands, [check.DRAG_SMOKE, check.CAD_BROWSER, *check.HARNESS_CHECKS, check.PAGE_SUITE])
        broken = check.HARNESS_CHECKS[1]
        for keep_going, ran in ((False, check.HARNESS_CHECKS[:2]), (True, check.HARNESS_CHECKS)):
            with self.subTest(keep_going=keep_going):
                passed, events, _, _, errors = self.browser_batch(commands, keep_going=keep_going, fail=[broken])
                self.assertFalse(passed)
                started = [command for kind, command in events if kind == "start"]
                self.assertEqual([command for command in started if command in check.HARNESS_CHECKS], ran)
                self.assertIn(check.DRAG_SMOKE, started)
                self.assertIn(check.PAGE_SUITE, started)
                self.assertIn("check: command failed: " + shlex.join(broken), errors)

    def test_static_wasm_test_check_runs_in_repo_and_tooling_is_one_batch_then_serial(self):
        static = [check.PY, "scripts/check-wasm-tests.py"]
        self.assertIn(static, check.STEPS["repo"][1])
        self.assertNotIn(static, check.browser())
        tooling = check.STEPS["tooling"][1]
        serial = [index for index, command in enumerate(tooling) if check.SERIAL_TOOLING_SCRIPTS.intersection(command)]
        self.assertEqual(serial, [len(tooling) - 2, len(tooling) - 1])
        self.assertEqual(check.page_suite_jobs(6), 2)
        self.assertEqual([check.page_suite_jobs(jobs) for jobs in (2, 3, 4)], [1, 2, 2])

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
            ("host", "worker-host", "runtime", "ui-model", "ui-shared", "keycaps", "library",
             "keymap", "case", "parts", "pcb", "layout", "layout-objects"),
        )
        custom = [check.crate_launch("custom-crate")]
        with mock.patch.dict(check.STEPS, {"browser": (True, custom)}):
            self.assertEqual(check.browser_crates(), ("custom-crate",))

    def test_browser_only_selects_crate_launches_without_build_or_drag_smoke_and_reports_partial_run(self):
        self.mock_wasm_dir()
        seen = []

        def fake(command, **kwargs):
            seen.append(command)
            if result := self.fake_wasm_build(command):
                return result
            if Path(command[0]).name == "wasm-bindgen-test-runner":
                self.assertEqual(kwargs["cwd"], ROOT / "web/crates" / Path(command[1]).stem)
                self.assertEqual(kwargs["env"]["WASM_BINDGEN_TEST_ONLY_WEB"], "1")
                self.assertEqual(kwargs["env"]["CHROMEDRIVER"], str(Path("/bin/true").resolve()))
                self.assertEqual(kwargs["env"]["WASM_BINDGEN_TEST_TIMEOUT"], "120")
            return mock.Mock(returncode=0, stdout="ok\n")

        output = io.StringIO()
        with mock.patch.object(check.subprocess, "run", side_effect=fake), \
                mock.patch.object(check, "_parallel_process", side_effect=lambda command, registry, **kwargs: fake(command, **kwargs)), \
                contextlib.redirect_stdout(output), contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(check.main(["browser", "--only", "case", "ui-shared"]), 0)
        self.assertEqual(seen[0], TAILWIND_CSS)
        self.assertTrue(check.is_crate_build(seen[1]))
        self.assertEqual(seen[1][len(check.WASM_CRATE_BUILD):],
                         ["-p", "boardstudio-web-ui-shared", "-p", "boardstudio-web-case"])
        self.assertCountEqual(seen[2:], [
            [str(self.wasm_dir / "wasm-bindgen-test-runner"), str(self.wasm_dir / f"{crate}.wasm"), *filters]
            for crate, filters in [("case", []), ("ui-shared", [])]
        ])
        self.assertFalse(any("build-web.py" in " ".join(cmd) or "run-drag-browser.py" in " ".join(cmd)
                             for cmd in seen))
        text = output.getvalue()
        self.assertIn("== browser (only: case, ui-shared)", text)
        self.assertIn("check: partial steps passed: test-css, browser (only: case, ui-shared)", text)
        self.assertNotIn("check: all steps passed:", text)

    def test_browser_only_layout_runs_test_css_prerequisite_and_stops_if_it_fails(self):
        self.mock_wasm_dir()
        seen = []

        def fake_pass(command, **kwargs):
            seen.append((command, kwargs.get("cwd")))
            if result := self.fake_wasm_build(command):
                return result
            return mock.Mock(returncode=0, stdout="ok\n")

        with mock.patch.object(check.subprocess, "run", side_effect=fake_pass), \
                mock.patch.dict(os.environ, {"BOARDSTUDIO_CHECK_JOBS": "1"}), \
                contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(check.main(["browser", "--only", "layout"]), 0)
        self.assertEqual(seen[0], (TAILWIND_CSS, check.ROOT / "web"))
        self.assertEqual(len(seen), 3, "test-css, one build and one layout crate launch")
        self.assertTrue(check.is_crate_build(seen[1][0]))
        self.assertEqual(seen[2][1], ROOT / "web/crates/layout")
        self.assertEqual(seen[2][0][2:], [])

        seen.clear()
        error = io.StringIO()
        with mock.patch.object(check.subprocess, "run", return_value=mock.Mock(returncode=2, stdout="")), \
                contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(error):
            self.assertEqual(check.main(["browser", "--only", "layout", "--keep-going"]), 1)
        self.assertIn("step 'test-css' failed", error.getvalue())

    def test_keep_going_skips_crate_runners_if_the_build_fails_or_omits_an_artifact(self):
        build, *launches = check.with_crate_build([check.crate_launch("layout")])
        trailing = [check.PY, "-c", "pass"]
        for build_code in (0, 101):
            with self.subTest(build_code=build_code):
                seen = []

                def fake(command, **kwargs):
                    seen.append(command)
                    return mock.Mock(returncode=build_code if command == build else 0, stdout="")

                error = io.StringIO()
                with mock.patch.object(check.subprocess, "run", side_effect=fake), \
                        contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(error):
                    _, ok = check.run_commands("browser", [build, *launches, trailing], {}, keep_going=True)
                self.assertFalse(ok)
                self.assertEqual(seen, [build, trailing])
                if not build_code:
                    self.assertIn("missing test wasm artifact(s) for layout", error.getvalue())

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
        self.mock_wasm_dir()
        seen = []

        def fake(command, **kwargs):
            seen.append(command)
            if result := self.fake_wasm_build(command):
                return result
            failed = kwargs["cwd"].name in ("library", "case")
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
        self.assertEqual(seen[0], TAILWIND_CSS)
        self.assertEqual(
            [Path(cmd[1]).stem for cmd in seen[2:]],
            ["library", "case", "parts"],
        )
        stderr = error.getvalue()
        self.assertIn("web/crates/library", stderr)
        self.assertIn("web/crates/case", stderr)
        self.assertIn("step 'browser (only: library, case, parts)' failed", stderr)

        # In a parallel batch with --keep-going, a failed crate launch continues to trailing
        # independent browser commands unless the page `--list` prerequisite itself fails.
        seen.clear()
        launches = check.with_crate_build([check.crate_launch(name) for name in ("library", "case")])
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
            if result := self.fake_wasm_build(command):
                return result
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

        self.mock_wasm_dir()

        def fake(command, **kwargs):
            return self.fake_wasm_build(command) or mock.Mock(returncode=1, stdout=trap_log)

        error = io.StringIO()
        with mock.patch.object(check.subprocess, "run", side_effect=fake), \
                mock.patch.dict(os.environ, {"BOARDSTUDIO_CHECK_JOBS": "1"}), \
                contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(error):
            self.assertEqual(check.main(["browser", "--only", "library"]), 1)
        self.assertIn("first failure [raw WASM trap]", error.getvalue())


class CadCaptureControlsCleanupTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        spec = importlib.util.spec_from_file_location(
            "cad_capture_controls", ROOT / "cad/scripts/test-cad-capture-controls.py")
        cls.controls = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.controls)

    def test_sigterm_handler_exits_143(self):
        with self.assertRaises(SystemExit) as exited:
            self.controls._sigterm_exit(signal.SIGTERM, None)
        self.assertEqual(exited.exception.code, 143)

    def test_descendant_pids_finds_children_recursively(self):
        parent = subprocess.Popen(
            [sys.executable, "-c", "import subprocess, time; subprocess.Popen(['sleep', '120']); time.sleep(120)"])
        grandchildren = set()
        try:
            for _ in range(200):
                grandchildren = self.controls._descendant_pids(parent.pid)
                if grandchildren:
                    break
                time.sleep(0.05)
            self.assertEqual(len(grandchildren), 1)
            mine = self.controls._descendant_pids(os.getpid())
            self.assertIn(parent.pid, mine)
            self.assertTrue(grandchildren <= mine)
        finally:
            for pid in {parent.pid, *grandchildren}:
                try:
                    os.kill(pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
            parent.wait()

    def test_reap_capture_kills_a_shared_group_member_without_killpg(self):
        # Under the scheduler the capture shares this runner's group; a killpg here
        # would kill this test itself, so only the capture's tree may die.
        parent = subprocess.Popen(
            [sys.executable, "-c", "import subprocess, time; subprocess.Popen(['sleep', '120']); time.sleep(120)"])
        grandchildren = set()
        for _ in range(200):
            grandchildren = self.controls._descendant_pids(parent.pid)
            if grandchildren:
                break
            time.sleep(0.05)
        self.assertEqual(len(grandchildren), 1)
        self.controls._reap_capture(parent, detached=False)
        self.assertEqual(parent.returncode, -signal.SIGKILL)
        for pid in grandchildren:
            for _ in range(200):
                try:
                    os.kill(pid, 0)
                except ProcessLookupError:
                    break
                time.sleep(0.01)
            else:
                self.fail(f"capture descendant {pid} survived the reap")

    def test_reap_capture_killpgs_a_detached_group(self):
        parent = subprocess.Popen(["sleep", "120"], start_new_session=True)
        self.controls._reap_capture(parent, detached=True)
        self.assertEqual(parent.returncode, -signal.SIGKILL)


class CadBrowserSessionCleanupTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        spec = importlib.util.spec_from_file_location(
            "cad_browser", ROOT / "cad/scripts/test-cad-browser.py")
        cls.page = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.page)

    def test_session_delete_timeout_warns_without_failing_a_green_page(self):
        def timeout(*args, **kwargs):
            raise TimeoutError("timed out")

        errors = []
        with contextlib.redirect_stderr(io.StringIO()) as warning:
            self.page._delete_session(timeout, 9515, "session", None, errors)
        self.assertEqual(errors, [])
        self.assertIn("session deletion timed out", warning.getvalue())

    def test_session_delete_timeout_annotates_a_page_failure(self):
        def timeout(*args, **kwargs):
            raise TimeoutError("timed out")

        failed = RuntimeError("smoke page never finished")
        with contextlib.redirect_stderr(io.StringIO()) as warning:
            self.page._delete_session(timeout, 9515, "session", failed, [])
        self.assertEqual(warning.getvalue(), "")
        self.assertTrue(any("session deletion timed out" in note
                            for note in getattr(failed, "__notes__", [])))

    def test_session_delete_error_still_fails_a_green_page(self):
        def refused(*args, **kwargs):
            raise RuntimeError("ChromeDriver DELETE /session/session failed: gone")

        errors = []
        with contextlib.redirect_stderr(io.StringIO()):
            self.page._delete_session(refused, 9515, "session", None, errors)
        self.assertEqual(errors, ["session deletion failed: "
                                  "ChromeDriver DELETE /session/session failed: gone"])


if __name__ == "__main__":
    unittest.main()
