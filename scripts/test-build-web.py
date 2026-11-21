#!/usr/bin/env python3
"""Focused tests for ordinary Dioxus release assembly."""

import importlib.util
import io
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location("build_web", Path(__file__).with_name("build-web.py"))
BUILD = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BUILD)
import tailwind_tools  # noqa: E402  (build-web.py put the scripts directory on sys.path)


class BuildWebTests(unittest.TestCase):
    def test_offline_worker_waits_for_old_clients_before_activation(self):
        worker = (BUILD.ROOT / "web/src/service_worker.rs").read_text()
        self.assertNotIn("skip_waiting", worker)
        self.assertIn("clients().claim()", worker)

    def test_provider_commands_use_retained_build_steps(self):
        commands = [" ".join(command) for command, _ in BUILD.provider_commands()]
        joined = "\n".join(commands)
        self.assertNotIn("wasm-pack build core", joined)
        for retained in (
            "--features core-worker", "--features cad-worker", "--features service-worker",
            "cad/scripts/build-cad-wasm.py",
            "stage-ergogen-models.py",
        ):
            self.assertIn(retained, joined)

    def test_fast_provider_commands_skip_wasm_opt_and_the_shipped_ones_keep_it(self):
        def builds(fast):
            return [command for command, _ in BUILD.provider_commands(fast)
                    if command[:2] == ["wasm-pack", "build"]]

        self.assertEqual(len(builds(True)), 4)
        self.assertTrue(all("--no-opt" in command for command in builds(True)))
        self.assertTrue(all("--no-opt" not in command for command in builds(False)))
        for command in builds(True):
            self.assertNotIn("--release", command)
            profile = command.index("--profile")
            self.assertEqual(command[profile + 1], "wasm-check")
        self.assertTrue(all("--release" in command and "--profile" not in command for command in builds(False)))
        cad = [command for command, _ in BUILD.provider_commands(True) if "cad/scripts/build-cad-wasm.py" in command]
        self.assertEqual(cad, [[BUILD.sys.executable, "cad/scripts/build-cad-wasm.py", "--no-opt"]])

    def overlapped_build(self, tools, page_build):
        """Run a fast single-route build whose provider lane and CAD lane must overlap."""
        import threading
        barrier = threading.Barrier(2, timeout=5)
        output = Path(self.enterContext(tempfile.TemporaryDirectory())) / "site"
        public = output / "public"
        finished = []

        def fake_run(command, *, cwd, output=None):
            if "scripts/stage-ergogen-models.py" in command:
                finished.append("ergogen")
                return
            if command[0] == "tailwindcss":
                finished.append("css")
                return
            self.assertIn("ergogen", finished)
            if "m1_core_worker" in command or "cad/scripts/build-cad-wasm.py" in command:
                barrier.wait()
            if command[0] == "dx":
                page_build(command)
                public.mkdir()
            finished.append(command)

        def fake_plain_page(route, page_tools, page_public, lane_output):
            self.assertEqual((route, page_tools, page_public), ("subpath", tools, public))
            self.assertIsInstance(lane_output, io.StringIO)
            finished.append("plain page")
            public.mkdir()

        def stage():
            self.assertEqual(len([step for step in finished if step != "css"]), 8)
            cad = next(i for i, step in enumerate(finished) if "cad/scripts/build-cad-wasm.py" in step)
            bake = next(i for i, step in enumerate(finished) if "cad/scripts/bake-bundled-step.py" in step)
            self.assertLess(cad, bake)

        with patch.object(BUILD, "run", fake_run), patch.object(BUILD, "check_jobs", return_value=3), \
             patch.object(BUILD, "page_tools", return_value=tools), patch.object(BUILD, "PAGE_PUBLIC", public), \
             patch.object(BUILD, "plain_page", fake_plain_page), \
             patch.object(BUILD, "generate_css", lambda tailwind: fake_run([tailwind], cwd=BUILD.WEB)), \
             patch.object(BUILD, "dx_public", return_value=public), patch.object(BUILD, "stage_runtime_assets", stage), \
             patch.object(BUILD, "copy_runtime_assets"), patch.object(BUILD, "embed_offline_worker"), \
             patch.object(BUILD.shutil, "copy2"):
            BUILD.build(output, release=False, fast=True)
        self.assertTrue((output / "site-subpath/boardstudio").is_dir())
        return finished

    def test_fast_single_route_runs_the_plain_page_after_the_wasm_check_providers(self):
        finished = self.overlapped_build(("tailwindcss", Path("wasm-bindgen"), "dx"),
                                         lambda command: self.fail(f"dx build ran: {command}"))
        # The stylesheet comes first, so check.py's WASM Clippy needn't wait for any build.
        self.assertEqual(finished[:2], ["css", "ergogen"])
        lane = [step for step in finished[2:] if step == "plain page" or "wasm-pack" in step]
        self.assertEqual(len(lane), 5)
        self.assertEqual(lane[-1], "plain page")

    def test_fast_single_route_falls_back_to_dx_without_debug_symbols(self):
        def dx_build(command):
            self.assertIn("--wasm-js-cfg", command)
            profile = command.index("--profile")
            self.assertEqual(command[profile:profile + 4], ["--profile", "wasm-check", "--debug-symbols", "false"])

        finished = self.overlapped_build(None, dx_build)
        self.assertEqual(finished[0], "ergogen")
        self.assertNotIn("css", finished)
        lane = [step for step in finished[1:] if step[0] in ("dx", "wasm-pack")]
        self.assertEqual([step[0] for step in lane], ["wasm-pack"] * 4 + ["dx"])

    def test_shipped_multi_route_and_single_job_never_launch_build_lanes(self):
        with tempfile.TemporaryDirectory() as directory, patch.object(BUILD, "run_lanes") as lanes, \
             patch.object(BUILD, "prepare_runtime_providers") as providers, patch.object(BUILD, "run"), \
             patch.object(BUILD, "page_tools", return_value=None), \
             patch.object(BUILD, "dx_public", return_value=Path(directory) / "missing"):
            for release, fast, routes, jobs in (
                (True, False, ("subpath",), 3),
                (False, True, ("subpath", "root"), 3),
                (False, True, ("subpath",), 1),
            ):
                with self.subTest(release=release, fast=fast, routes=routes, jobs=jobs), \
                     patch.object(BUILD, "check_jobs", return_value=jobs), self.assertRaises(FileNotFoundError):
                    BUILD.build(Path(directory), release=release, routes=routes, fast=fast)
                self.assertIsNone(providers.call_args.kwargs["page_build"])
            lanes.assert_not_called()

    def test_shipped_build_keeps_dx_release_and_never_looks_for_page_tools(self):
        with tempfile.TemporaryDirectory() as directory, \
             patch.object(BUILD, "page_tools", side_effect=AssertionError("page tools")), \
             patch.object(BUILD, "plain_page", side_effect=AssertionError("plain page")), \
             patch.object(BUILD, "run") as run, \
             patch.object(BUILD, "dx_public", return_value=Path(directory) / "missing"), \
             self.assertRaises(FileNotFoundError):
            BUILD.build(Path(directory), prepare_providers=False)
        command = run.call_args.args[0]
        self.assertEqual(command[:4], ["dx", "build", "--web", "--release"])
        self.assertNotIn("--debug-symbols", command)
        self.assertNotIn("wasm-check", command)

    def test_multi_route_plain_pages_build_each_route_after_one_stylesheet(self):
        calls = []
        tools = ("tailwindcss", Path("wasm-bindgen"), "dx")

        def fake_plain_page(route, page_tools, public):
            calls.append(route)
            public.mkdir(parents=True)

        with tempfile.TemporaryDirectory() as directory, \
             patch.object(BUILD, "page_tools", return_value=tools), \
             patch.object(BUILD, "PAGE_PUBLIC", Path(directory) / "public"), \
             patch.object(BUILD, "generate_css", lambda tailwind: calls.append(tailwind)), \
             patch.object(BUILD, "plain_page", fake_plain_page), patch.object(BUILD, "run_lanes") as lanes, \
             patch.object(BUILD, "copy_runtime_assets"), patch.object(BUILD, "embed_offline_worker"):
            BUILD.build(Path(directory) / "out", release=False, fast=True, prepare_providers=False,
                        routes=("subpath", "root"))
        self.assertEqual(calls, ["tailwindcss", "subpath", "root"])
        lanes.assert_not_called()

    def test_generated_stylesheet_runs_the_standalone_cli_from_the_web_project(self):
        commands = []
        with patch.object(BUILD, "run", lambda command, *, cwd: commands.append((command, cwd))), \
             patch.object(BUILD.sys, "stdout", io.StringIO()):
            BUILD.generate_css(Path("/tools/tailwindcss"))
        self.assertEqual(commands, [(["/tools/tailwindcss", "-i", "tailwind.css", "-o", "assets/tailwind.css"],
                                     BUILD.WEB)])

    def test_overlapped_build_keeps_the_stylesheet_check_py_already_wrote(self):
        # check.py writes it before starting build, lint and test together; WASM Clippy may be reading it.
        tools = ("tailwindcss", Path("wasm-bindgen"), "dx")
        for overlapped, expected in ((False, ["tailwindcss"]), (True, [])):
            calls = []
            with self.subTest(overlapped=overlapped), tempfile.TemporaryDirectory() as directory, \
                 patch.dict(os.environ), patch.object(BUILD, "page_tools", return_value=tools), \
                 patch.object(BUILD, "PAGE_PUBLIC", Path(directory) / "public"), \
                 patch.object(BUILD, "generate_css", calls.append), \
                 patch.object(BUILD, "plain_page", lambda _route, _tools, public: public.mkdir(parents=True)), \
                 patch.object(BUILD, "copy_runtime_assets"), patch.object(BUILD, "embed_offline_worker"):
                os.environ.pop(BUILD.IN_OVERLAP_ENV, None)
                if overlapped:
                    os.environ[BUILD.IN_OVERLAP_ENV] = "1"
                BUILD.build(Path(directory) / "out", release=False, fast=True, prepare_providers=False)
            self.assertEqual(calls, expected)

    def test_page_tools_fall_back_to_dx_when_any_tool_is_missing(self):
        for missing in ("tailwind_cli", "locked_wasm_bindgen", "matching_dx"):
            found = {"tailwind_cli": Path("/t/tailwindcss"), "locked_wasm_bindgen": Path("/t/wasm-bindgen")}
            with self.subTest(missing=missing), \
                 patch.object(BUILD, "tailwind_cli", return_value=None if missing == "tailwind_cli" else found["tailwind_cli"]), \
                 patch.object(BUILD, "locked_wasm_bindgen",
                              return_value=None if missing == "locked_wasm_bindgen" else found["locked_wasm_bindgen"]), \
                 patch.object(BUILD, "matching_dx", return_value=None if missing == "matching_dx" else "/t/dx"), \
                 patch.object(BUILD.sys, "stderr", io.StringIO()):
                self.assertIsNone(BUILD.page_tools())
        with patch.object(BUILD, "tailwind_cli", return_value=Path("/t/tailwindcss")), \
             patch.object(BUILD, "locked_wasm_bindgen", return_value=Path("/t/wasm-bindgen")), \
             patch.object(BUILD, "matching_dx", return_value="/t/dx"):
            self.assertEqual(BUILD.page_tools(), (Path("/t/tailwindcss"), Path("/t/wasm-bindgen"), "/t/dx"))

    def test_page_tools_quiet_probe_reports_without_stderr(self):
        with patch.object(BUILD, "tailwind_cli", return_value=Path("/t/tailwindcss")), \
             patch.object(BUILD, "locked_wasm_bindgen", return_value=None), \
             patch.object(BUILD, "matching_dx", return_value="/t/dx"), \
             patch.object(BUILD.sys, "stderr", io.StringIO()) as stderr:
            self.assertIsNone(BUILD.page_tools(quiet=True))
            self.assertEqual(stderr.getvalue(), "")

    def test_matching_dx_requires_the_pinned_plain_page_version(self):
        with patch.object(BUILD.shutil, "which", return_value=None):
            self.assertIsNone(BUILD.matching_dx())
        for returncode, stdout, expected in (
            (0, f"dioxus {BUILD.PLAIN_PAGE_DX_VERSION} (57d6794)\n", "/t/dx"),
            (0, "dioxus 0.8.0 (abc1234)\n", None),
            (1, f"dioxus {BUILD.PLAIN_PAGE_DX_VERSION}\n", None),
        ):
            with self.subTest(returncode=returncode, stdout=stdout), \
                 patch.object(BUILD.shutil, "which", return_value="/t/dx"), \
                 patch.object(BUILD.subprocess, "run",
                              return_value=subprocess.CompletedProcess(["/t/dx", "--version"], returncode, stdout, "")):
                self.assertEqual(BUILD.matching_dx(), expected)

    def test_tailwind_cli_prefers_the_override_then_path_then_newest_dx_tool(self):
        def tool(path, banner):
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(f"#!/bin/sh\necho '{banner}'\n")
            path.chmod(0o755)
            return path

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            old = tool(root / "dx/tailwindcss-v4.0.0/tailwindcss", "tailwindcss v4.0.0")
            newest = tool(root / "dx/tailwindcss-v4.1.5/tailwindcss", "tailwindcss v4.1.5")
            os.utime(old, (1, 1))
            v3 = tool(root / "bin/tailwindcss", "tailwindcss v3.4.15")
            with patch.object(tailwind_tools, "DX_TOOLS", root / "dx"), patch.dict(os.environ, {}, clear=False):
                os.environ.pop(tailwind_tools.TAILWIND_ENV, None)
                with patch.object(BUILD.shutil, "which", return_value=str(v3)):
                    self.assertEqual(BUILD.tailwind_cli(), newest)
                v4 = tool(root / "bin/tailwindcss", "tailwindcss v4.1.4")
                with patch.object(BUILD.shutil, "which", return_value=str(v4)):
                    self.assertEqual(BUILD.tailwind_cli(), v4)
                os.environ[tailwind_tools.TAILWIND_ENV] = str(old)
                self.assertEqual(BUILD.tailwind_cli(), old)
                # An explicit pin is trusted as-is without probing: the --help probe
                # rejects CI's sha-verified install on hosted runners.
                os.environ[tailwind_tools.TAILWIND_ENV] = str(v3)
                tool(v3, "tailwindcss v3.4.15")
                self.assertEqual(BUILD.tailwind_cli(), v3)
            with patch.object(tailwind_tools, "DX_TOOLS", root / "none"), patch.object(BUILD.shutil, "which", return_value=None), \
                 patch.dict(os.environ, {}, clear=False):
                os.environ.pop(tailwind_tools.TAILWIND_ENV, None)
                self.assertIsNone(BUILD.tailwind_cli())

    def test_cargo_page_artifact_needs_exactly_one_page_wasm(self):
        with tempfile.TemporaryDirectory() as directory:
            wasm = Path(directory) / "boardstudio-web.wasm"
            wasm.write_bytes(b"\0asm")

            def artifact(**changes):
                message = {"reason": "compiler-artifact", "target": {"kind": ["bin"], "name": "boardstudio-web"},
                           "manifest_path": str(BUILD.WEB / "Cargo.toml"), "executable": str(wasm)}
                message.update(changes)
                return json.dumps(message)

            warning = json.dumps({"reason": "compiler-message", "message": {"rendered": "warning: rendered\n"}})
            other = artifact(target={"kind": ["lib"], "name": "boardstudio-web"}, executable=None)
            for lines, returncode, expected in (
                ([warning, other, artifact()], 0, wasm),
                ([other], 0, FileNotFoundError),
                ([artifact(), artifact()], 0, FileNotFoundError),
                ([artifact(manifest_path="/elsewhere/Cargo.toml")], 0, FileNotFoundError),
                ([warning], 101, subprocess.CalledProcessError),
            ):
                with self.subTest(lines=lines, returncode=returncode):
                    output = io.StringIO()
                    result = subprocess.CompletedProcess(BUILD.PAGE_CARGO, returncode, "\n".join(lines), "stderr\n")
                    with patch.object(BUILD.subprocess, "run", return_value=result) as run:
                        if isinstance(expected, Path):
                            self.assertEqual(BUILD.cargo_page_artifact(output), expected)
                        else:
                            with self.assertRaises(expected):
                                BUILD.cargo_page_artifact(output)
                    command = run.call_args.args[0]
                    self.assertEqual(command[:2], ["cargo", "build"])
                    for flag in ("--locked", "--message-format=json", "--no-default-features"):
                        self.assertIn(flag, command)
                    self.assertEqual(command[command.index("--profile") + 1], "wasm-check")
                    self.assertEqual(command[command.index("--features") + 1], "page")
                    self.assertNotIn("RAYON_NUM_THREADS", run.call_args.kwargs.get("env") or {})
                    if warning in lines:
                        self.assertIn("warning: rendered", output.getvalue())
                    self.assertIn("stderr", output.getvalue())

    def test_plain_page_matches_the_dx_runtime_contract(self):
        bindgen_js = ("let wasmModule, wasmInstance, wasm;\nfunction __wbg_get_imports() {}\n"
                      "function initSync(module) {}\nasync function __wbg_init(module_or_path) {}\n"
                      "export { initSync, __wbg_init as default };\n")
        for route, base, meta in (("subpath", "/boardstudio/", '<meta name="DIOXUS_ASSET_ROOT" content="boardstudio">'),
                                  ("root", "/", None)):
            with self.subTest(route=route), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                cargo_wasm = root / "target/boardstudio-web.wasm"
                cargo_wasm.parent.mkdir()
                cargo_wasm.write_bytes(b"cargo")
                public = root / "page/public"
                calls = []

                def fake_run(command, *, cwd=BUILD.ROOT, env=None, output=None):
                    calls.append(command)
                    if command[1:3] == ["tools", "assets"]:
                        self.assertEqual(cwd, BUILD.WEB)
                        module = Path(command[3])
                        self.assertNotEqual(module, cargo_wasm)
                        module.write_bytes(b"patched")
                        Path(command[4]).mkdir(parents=True)
                        (Path(command[4]) / "focus-trap-dxh0.js").write_text("export {}")
                    else:
                        self.assertEqual(env["RAYON_NUM_THREADS"], "1")
                        self.assertEqual(Path(command[-1]).read_bytes(), b"patched")
                        out = Path(command[command.index("--out-dir") + 1])
                        out.mkdir(parents=True)
                        (out / "boardstudio-web.js").write_text(bindgen_js)

                with patch.object(BUILD, "cargo_page_artifact", return_value=cargo_wasm), \
                     patch.object(BUILD, "run", fake_run):
                    BUILD.plain_page(route, (Path("/t/tailwindcss"), Path("/t/wasm-bindgen"), "/t/dx"), public)

                self.assertEqual(cargo_wasm.read_bytes(), b"cargo")
                self.assertEqual([command[0] for command in calls], ["/t/dx", "/t/wasm-bindgen"])
                bindgen = calls[1]
                for flag in ("--target", "web", "--no-typescript", "--remove-name-section"):
                    self.assertIn(flag, bindgen)
                self.assertEqual(bindgen[bindgen.index("--out-name") + 1], "boardstudio-web")
                self.assertTrue((public / "assets/focus-trap-dxh0.js").is_file())
                script = (public / "wasm/boardstudio-web.js").read_text()
                self.assertTrue(script.startswith(bindgen_js + "export { wasm as __wasm }\n"))
                self.assertIn(f'__wbg_init({{module_or_path: "{base}wasm/boardstudio-web_bg.wasm"}})', script)
                for line in ("globalThis.__wasm_split_main_initSync = initSync;",
                             "globalThis.__dx_mainWasm = wasm;", "globalThis.__dx_mainInit = __wbg_init;",
                             "globalThis.__dx_mainInitSync = initSync;",
                             "globalThis.__dx___wbg_get_imports = __wbg_get_imports;", "wasm.main();"):
                    self.assertIn(line, script)
                index = (public / "index.html").read_text()
                for element in ("<title>BoardStudio</title>",
                                '<meta content="text/html;charset=utf-8" http-equiv="Content-Type">',
                                '<meta name="viewport" content="width=device-width, initial-scale=1">',
                                '<meta charset="UTF-8">', '<div id="main"></div>',
                                f'<script type="module" async src="{base}wasm/boardstudio-web.js"></script>'):
                    self.assertIn(element, index)
                if meta:
                    self.assertIn(meta, index)
                else:
                    self.assertNotIn("DIOXUS_ASSET_ROOT", index)

    def test_plain_page_rejects_unfamiliar_wasm_bindgen_output(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "page.wasm").write_bytes(b"cargo")

            def fake_run(command, *, cwd=BUILD.ROOT, env=None, output=None):
                if command[1:3] == ["tools", "assets"]:
                    assets = Path(command[4])
                    assets.mkdir(parents=True)
                    (assets / "focus-trap-dxh0.js").write_text("export {}")
                elif "--out-dir" in command:
                    out = Path(command[command.index("--out-dir") + 1])
                    out.mkdir(parents=True)
                    (out / "boardstudio-web.js").write_text("function initSync(module) {}\n")

            with patch.object(BUILD, "cargo_page_artifact", return_value=root / "page.wasm"), \
                 patch.object(BUILD, "run", fake_run), self.assertRaisesRegex(ValueError, "__wbg_init"):
                BUILD.plain_page("subpath", (Path("t"), Path("b"), "dx"), root / "public")

    def test_plain_page_rejects_empty_extracted_assets(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "page.wasm").write_bytes(b"cargo")

            def fake_run(command, *, cwd=BUILD.ROOT, env=None, output=None):
                if command[1:3] == ["tools", "assets"]:
                    Path(command[4]).mkdir(parents=True)

            with patch.object(BUILD, "cargo_page_artifact", return_value=root / "page.wasm"), \
                 patch.object(BUILD, "run", fake_run), self.assertRaisesRegex(FileNotFoundError, "extracted no files"):
                BUILD.plain_page("subpath", (Path("t"), Path("b"), "dx"), root / "public")

    def test_model_staging_command_runs_with_the_staging_cli(self):
        import subprocess
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "ergogen/library/vendor/example/3d_models"
            source.mkdir(parents=True)
            (source / "switch.step").write_text("fixture model")
            command = next(command[:] for command, _ in BUILD.provider_commands()
                           if "stage-ergogen-models.py" in " ".join(command))
            command[1] = str(Path(__file__).with_name("stage-ergogen-models.py"))
            result = subprocess.run(command, cwd=root, capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertTrue(list((root / "web/assets/ergogen-models").iterdir()))

    def test_build_assembles_requested_routes_from_dirty_worktree(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            web = root / "web"
            public = web / "target/dx/boardstudio-web/release/web/public"
            (web / "assets").mkdir(parents=True)
            (web / "assets/workbench-base.css").write_text("body{}")
            (web / "assets/layout-generators.js").write_text("obsolete generator")
            for name in ("layout-generators", "preview-generator"):
                (web / "assets" / name).mkdir()
                (web / "assets" / name / "worker.mjs").write_text("obsolete generator")
            (root / "catalogue/modules").mkdir(parents=True)
            (root / "catalogue/modules/imported-modules.json").write_text('{"modules":[]}')
            (root / "cad/wasm/pkg").mkdir(parents=True)
            (root / "cad/wasm/pkg/cad.js").write_text("export {}")
            (root / "renderer/pkg").mkdir(parents=True)
            (root / "renderer/pkg/renderer.js").write_text("export {}")
            for name in ("core-worker", "cad-worker"):
                provider = web / "target/providers" / name
                provider.mkdir(parents=True)
                (provider / f"{name}.js").write_text("export {}")
            calls = []

            def fake_run(command, *, cwd=BUILD.ROOT, env=None):
                calls.append(command)
                prefix = command[command.index("--base-path") + 1]
                public.mkdir(parents=True, exist_ok=True)
                (public / "index.html").write_text(prefix)

            def fake_embed(site):
                (site / "service-worker.js").write_text("worker")

            with patch.object(BUILD, "ROOT", root), patch.object(BUILD, "WEB", web), \
                 patch.object(BUILD, "dx_public", lambda _release: public), patch.object(BUILD, "run", fake_run), \
                 patch.object(BUILD, "embed_offline_worker", fake_embed):
                default = BUILD.build(root / "default", prepare_providers=False)
                output = BUILD.build(root / "output", prepare_providers=False,
                                     routes=("subpath", "root"))
                (output / "site-root/stale.txt").write_text("preserved")
                (output / "site-subpath/boardstudio/stale.txt").write_text("removed")
                BUILD.build(output, prepare_providers=False)
                self.assertTrue((output / "site-root/stale.txt").is_file())
                self.assertFalse((output / "site-subpath/boardstudio/stale.txt").exists())

            self.assertEqual(len(calls), 4)
            self.assertFalse((default / "site-root").exists())
            self.assertEqual((default / "site-subpath/boardstudio/index.html").read_text(), "/boardstudio/")
            root_site = output / "site-root"
            subpath_site = output / "site-subpath/boardstudio"
            self.assertEqual((root_site / "index.html").read_text(), "/")
            self.assertEqual((subpath_site / "index.html").read_text(), "/boardstudio/")
            for site in (root_site, subpath_site):
                self.assertTrue((site / "assets/workbench-base.css").is_file())
                for name in ("layout-generators.js", "layout-generators", "preview-generator"):
                    self.assertFalse((site / "assets" / name).exists(), name)
                self.assertTrue((site / "assets/imported-modules.json").is_file())
                self.assertTrue((site / "assets/cad/cad.js").is_file())
                self.assertTrue((site / "assets/renderer/renderer.js").is_file())
                self.assertTrue((site / "assets/core-worker/entry.js").is_file())
                self.assertTrue((site / "assets/cad-worker/entry.js").is_file())
                self.assertTrue((site / "service-worker.js").is_file())

    def test_site_and_dev_assets_stage_the_same_providers_and_worker_entries(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for path in ("web/assets/workbench-base.css", "catalogue/modules/imported-modules.json",
                         "cad/wasm/pkg/cad.js", "renderer/pkg/renderer.js",
                         "web/target/providers/core-worker/m1_core_worker.js",
                         "web/target/providers/cad-worker/m1_cad_worker.js"):
                (root / path).parent.mkdir(parents=True, exist_ok=True)
                (root / path).write_text(path)
            site = root / "site"
            BUILD.copy_runtime_assets(site, root)
            BUILD.stage_runtime_assets(root)
            dev = root / "web/assets"
            self.assertTrue((site / "assets/workbench-base.css").is_file())
            self.assertTrue((site / "assets/imported-modules.json").is_file())
            self.assertFalse((dev / "imported-modules.json").exists(), "dx serve's copy is prepare_runtime_providers'")
            for relative in ("cad/cad.js", "renderer/renderer.js", "core-worker/m1_core_worker.js",
                             "cad-worker/m1_cad_worker.js", "core-worker/entry.js", "cad-worker/entry.js"):
                self.assertEqual((site / "assets" / relative).read_text(), (dev / relative).read_text(), relative)
            self.assertEqual((dev / "core-worker/entry.js").read_text(),
                             'import init, { start_core_worker } from "./m1_core_worker.js";\n'
                             'await init();\nstart_core_worker();\n')
            self.assertEqual((dev / "cad-worker/entry.js").read_text(),
                             'import init, { start_cad_worker } from "./m1_cad_worker.js";\nawait init();\n'
                             'start_cad_worker(new URL("../cad/boardstudio_cad_wasm.js", import.meta.url).href);\n')

    def test_baked_step_staging_uses_only_current_manifest_files(self):
        import hashlib
        import json
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "web/target/providers/bundled-step"
            source.mkdir(parents=True)
            worker = root / "site/assets/cad-worker"
            (worker / "bundled-step").mkdir(parents=True)
            (worker / "bundled-step/old.bstep").write_bytes(b"old")
            (source / "old.bstep").write_bytes(b"old")
            original = hashlib.sha256(b"original STEP").hexdigest()
            digest = hashlib.sha256(b"preview payload").hexdigest()
            (source / f"{digest}.bstep").write_bytes(b"preview payload")
            (source / "manifest.json").write_text(json.dumps({"assets": {original: digest}}))
            loader = BUILD._stage_baked_step(worker, root)
            self.assertIn(json.dumps({original: digest}), loader)
            self.assertEqual([p.name for p in (worker / "bundled-step").iterdir()], [f"{digest}.bstep"])
            self.assertEqual((worker / f"bundled-step/{digest}.bstep").read_bytes(), b"preview payload")
            (source / "manifest.json").unlink()
            self.assertEqual(BUILD._stage_baked_step(worker, root), "")
            self.assertFalse((worker / "bundled-step").exists())

    def test_offline_worker_bootstrap_passes_route_manifest(self):
        import json
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            site = root / "site"
            (site / "assets").mkdir(parents=True)
            (site / "index.html").write_text("shell")
            (site / "assets/app.wasm").write_bytes(b"app")
            worker = root / "offline-worker"
            worker.mkdir()
            (worker / "boardstudio_offline_worker.js").write_text("export function initSync(){}")
            (worker / "boardstudio_offline_worker_bg.wasm").write_bytes(b"wasm")

            BUILD.embed_offline_worker(site, worker)

            bootstrap = (site / "service-worker.js").read_text()
            self.assertIn('import { initSync, start_offline_worker } from "./boardstudio_offline_worker.js"',
                          bootstrap)
            call = bootstrap.splitlines()[-1]
            self.assertTrue(call.startswith("start_offline_worker(") and call.endswith(");"), call)
            version, assets = json.loads("[" + call[len("start_offline_worker("):-2] + "]")
            self.assertRegex(version, r"^boardstudio-[0-9a-f]{16}-[0-9]+$")
            self.assertEqual(assets, sorted({"index.html", "assets/app.wasm", "service-worker.js",
                                             "boardstudio_offline_worker.js"}))
            self.assertTrue((site / "boardstudio_offline_worker.js").is_file())

    def test_offline_manifest_validation_matches_the_worker(self):
        valid = {"version": "boardstudio-1", "assets": ["index.html", "assets/a.wasm"]}
        BUILD.validate_offline_manifest(valid)
        for bad in ({"version": "a b", "assets": ["index.html"]},
                    {"version": "v", "assets": ["app.js"]},
                    {"version": "v", "assets": ["index.html", "index.html"]},
                    {"version": "v", "assets": ["index.html", "../x"]},
                    {"version": "v", "assets": ["index.html", "a?b"]}):
            with self.assertRaises(ValueError, msg=bad):
                BUILD.validate_offline_manifest(bad)

    def test_routes_option_rejects_unknown_routes(self):
        with self.assertRaises(SystemExit):
            BUILD.main(["--routes", "elsewhere"])

    def test_plain_page_is_cargo_then_stage(self):
        calls = []
        tools = ("tailwindcss", Path("wasm-bindgen"), "dx")
        with patch.object(BUILD, "plain_page_cargo",
                          lambda public, output: calls.append(("cargo", public, output))), \
             patch.object(BUILD, "plain_page_stage",
                          lambda route, tools, public, output: calls.append(("stage", route, tools, public, output))):
            BUILD.plain_page("subpath", tools, Path("/fake/public"), output="out")
        self.assertEqual(calls, [("cargo", Path("/fake/public"), "out"),
                                 ("stage", "subpath", tools, Path("/fake/public"), "out")])

    def test_phase_cargo_runs_providers_and_page_compile_without_staging(self):
        import threading
        barrier = threading.Barrier(2, timeout=5)
        root = Path(self.enterContext(tempfile.TemporaryDirectory()))
        output, public = root / "site", root / "public"
        tools = ("tailwindcss", Path("wasm-bindgen"), "dx")
        finished = []
        artifact = root / "cargo.wasm"
        artifact.write_bytes(b"cargo")

        def fake_run(command, *, cwd, output=None):
            if "scripts/stage-ergogen-models.py" in command:
                finished.append("ergogen")
                return
            if command[0] == "tailwindcss":
                finished.append("css")
                return
            if command[0] == "dx" or "wasm-bindgen" in command[0]:
                self.fail(f"staging ran during the cargo phase: {command}")
            if "m1_core_worker" in command or "cad/scripts/build-cad-wasm.py" in command:
                barrier.wait()
            finished.append(command)

        def fail_staging(*args, **kwargs):
            self.fail(f"staging ran during the cargo phase: {args}")

        stdout = io.StringIO()
        with patch.object(BUILD, "run", fake_run), patch.object(BUILD, "check_jobs", return_value=3), \
             patch.object(BUILD, "page_tools", return_value=tools), patch.object(BUILD, "PAGE_PUBLIC", public), \
             patch.object(BUILD, "cargo_page_artifact", return_value=artifact), \
             patch.object(BUILD, "generate_css", lambda tailwind: fake_run([tailwind], cwd=BUILD.WEB)), \
             patch.object(BUILD, "finish_provider_staging", fail_staging), \
             patch.object(BUILD, "copy_runtime_assets", fail_staging), \
             patch.object(BUILD, "embed_offline_worker", fail_staging), \
             patch.object(BUILD.sys, "stdout", stdout), patch.dict(os.environ):
            os.environ.pop(BUILD.IN_OVERLAP_ENV, None)
            self.assertEqual(BUILD.build(output, release=False, fast=True, phase="cargo"), output)
        self.assertEqual(finished[:2], ["css", "ergogen"])
        self.assertEqual(len([step for step in finished[2:] if step[0] == "wasm-pack"]), 4)
        self.assertTrue(any("cad/scripts/build-cad-wasm.py" in step for step in finished[2:]))
        module = public.parent / "boardstudio-web.wasm"
        self.assertTrue(module.is_file())
        self.assertEqual(module.read_bytes(), b"cargo")
        self.assertIn(str(module), stdout.getvalue())
        self.assertFalse((output / "site-subpath/boardstudio").exists())

    def test_phase_stage_runs_only_post_cargo_staging(self):
        root = Path(self.enterContext(tempfile.TemporaryDirectory()))
        output, public = root / "site", root / "public"
        tools = ("tailwindcss", Path("wasm-bindgen"), "dx")
        (public.parent / "boardstudio-web.wasm").parent.mkdir(parents=True, exist_ok=True)
        (public.parent / "boardstudio-web.wasm").write_bytes(b"cargo")
        bindgen_js = ("let wasmModule, wasmInstance, wasm;\nfunction __wbg_get_imports() {}\n"
                      "function initSync(module) {}\nasync function __wbg_init(module_or_path) {}\n")
        calls = []

        def fake_run(command, *, cwd=BUILD.ROOT, env=None, output=None):
            calls.append(command)
            if "wasm-pack" in command[0] or "build-cad-wasm" in " ".join(command) \
                    or "stage-ergogen-models" in " ".join(command):
                self.fail(f"cargo ran during the stage phase: {command}")
            if command[1:3] == ["tools", "assets"]:
                assets = Path(command[4])
                assets.mkdir(parents=True)
                (assets / "focus-trap-dxh0.js").write_text("export {}")
            else:
                out = Path(command[command.index("--out-dir") + 1])
                out.mkdir(parents=True)
                (out / "boardstudio-web.js").write_text(bindgen_js)

        staged = []
        with patch.object(BUILD, "run", fake_run), \
             patch.object(BUILD, "page_tools", return_value=tools), patch.object(BUILD, "PAGE_PUBLIC", public), \
             patch.object(BUILD, "cargo_page_artifact", side_effect=AssertionError("cargo ran")), \
             patch.object(BUILD, "generate_css", side_effect=AssertionError("css ran")), \
             patch.object(BUILD, "finish_provider_staging", lambda: staged.append("providers")), \
             patch.object(BUILD, "copy_runtime_assets", lambda site: staged.append("copy")), \
             patch.object(BUILD, "embed_offline_worker", lambda site: staged.append("offline")), \
             patch.object(BUILD.sys, "stdout", io.StringIO()):
            self.assertEqual(BUILD.build(output, release=False, fast=True, phase="stage"), output)
        self.assertEqual([command[0] for command in calls], ["dx", "wasm-bindgen"])
        self.assertEqual(staged, ["providers", "copy", "offline"])
        site = output / "site-subpath/boardstudio"
        self.assertTrue((site / "index.html").is_file())
        self.assertTrue((site / "assets/focus-trap-dxh0.js").is_file())
        self.assertTrue((site / "wasm/boardstudio-web.js").is_file())

    def test_phase_rejects_non_splittable_builds(self):
        for argv in (["--fast", "--phase", "cargo", "--providers-only"],
                    ["--fast", "--phase", "cargo", "--skip-providers"],
                    ["--phase", "cargo"],
                    ["--fast", "--phase", "stage", "--routes", "subpath,root"]):
            with self.subTest(argv=argv), self.assertRaises(SystemExit):
                BUILD.main(argv)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            tools = ("tailwindcss", Path("wasm-bindgen"), "dx")
            with patch.object(BUILD, "page_tools", return_value=tools):
                for kwargs in ({"release": True, "fast": False},
                               {"release": False, "fast": True, "prepare_providers": False},
                               {"release": False, "fast": True, "routes": ("subpath", "root")},
                               {"release": False, "fast": True, "phase": "bogus"}):
                    with self.subTest(kwargs=kwargs), self.assertRaises(ValueError):
                        BUILD.build(root / "out", **{"phase": "cargo", **kwargs})
            with patch.object(BUILD, "page_tools", return_value=None), \
                 self.assertRaisesRegex(ValueError, "plain-page tools"):
                BUILD.build(root / "out", release=False, fast=True, phase="cargo")

    def test_phase_stage_without_cargo_phase_fails_closed(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            tools = ("tailwindcss", Path("wasm-bindgen"), "dx")
            with patch.object(BUILD, "page_tools", return_value=tools), \
                 patch.object(BUILD, "PAGE_PUBLIC", root / "public"), \
                 self.assertRaisesRegex(FileNotFoundError, "--phase cargo first"):
                BUILD.build(root / "out", release=False, fast=True, phase="stage")

if __name__ == "__main__":
    unittest.main()
