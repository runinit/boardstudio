#!/usr/bin/env python3
"""Focused tests for ordinary Dioxus release assembly."""

import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location("build_web", Path(__file__).with_name("build-web.py"))
BUILD = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BUILD)


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
        self.assertTrue(all("--release" in command for command in builds(True)))

    def test_provider_lanes_keep_wasm_pack_serial_and_cad_alone(self):
        lane_a, lane_b = BUILD.provider_lanes(True)
        self.assertEqual(len(lane_a), 4)
        self.assertTrue(all(command[:2] == ["wasm-pack", "build"] for command, _ in lane_a))
        self.assertEqual(len(lane_b), 1)
        self.assertIn("build-cad-wasm.py", " ".join(lane_b[0][0]))
        with patch.object(BUILD, "provider_commands", return_value=[(["mystery", "cmd"], BUILD.ROOT)]):
            with self.assertRaisesRegex(ValueError, "unassigned provider command"):
                BUILD.provider_lanes()

    def test_provider_lanes_overlap_but_stage_only_after_all(self):
        import threading
        lane_a, lane_b = BUILD.provider_lanes(True)
        barrier = threading.Barrier(2, timeout=10)
        completed = []

        def fake_captured(command, cwd, env):
            text = " ".join(command)
            if "m1_core_worker" in text or "build-cad-wasm" in text:
                barrier.wait()
            completed.append(text)
            return text

        with patch.object(BUILD, "_run_captured", side_effect=fake_captured), \
             patch.object(BUILD, "shutil") as shutil, \
             patch.object(BUILD, "stage_runtime_assets") as stage:
            BUILD.prepare_runtime_providers(True)
        self.assertEqual(len(completed), 5)
        shutil.copy2.assert_called_once()
        stage.assert_called_once()

    def test_provider_failure_does_not_stage_assets(self):
        def fake_captured(command, cwd, env):
            if "m1_core_worker" in " ".join(command):
                raise ValueError("provider failed")
            return "ok"

        with patch.object(BUILD, "_run_captured", side_effect=fake_captured), \
             patch.object(BUILD, "shutil") as shutil, \
             patch.object(BUILD, "stage_runtime_assets") as stage:
            with self.assertRaisesRegex(ValueError, "provider failed"):
                BUILD.prepare_runtime_providers()
        shutil.copy2.assert_not_called()
        stage.assert_not_called()

    def test_dx_lane_overlaps_providers_only_for_fast_single_route(self):
        import threading
        barrier = threading.Barrier(2, timeout=10)
        seen = []

        def fake_captured(command, cwd, env):
            text = " ".join(command)
            if command[0] == "dx" or "m1_core_worker" in text:
                barrier.wait()
            seen.append(text)
            return text

        dx = [("dx", [(["dx", "build"], BUILD.WEB, None)])]
        with patch.object(BUILD, "_run_captured", side_effect=fake_captured), \
             patch.object(BUILD, "shutil"), \
             patch.object(BUILD, "stage_runtime_assets"):
            BUILD.prepare_runtime_providers(True, dx_lanes=dx)
        self.assertTrue(any(part.startswith("dx build") for part in seen))

    def test_shipped_and_multi_route_builds_keep_dx_serial(self):
        import tempfile
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "site"
            with patch.object(BUILD, "prepare_runtime_providers") as prepare, \
                 patch.object(BUILD, "run") as run, \
                 patch.object(BUILD, "package_route") as package:
                BUILD.build(output, release=False, routes=("subpath", "root"), fast=True)
                prepare.assert_called_once()
                self.assertEqual(prepare.call_args[1].get("dx_lanes"), [])
                self.assertEqual(run.call_count, 2)
                self.assertEqual(package.call_count, 2)
            with patch.object(BUILD, "prepare_runtime_providers") as prepare, \
                 patch.object(BUILD, "run") as run, \
                 patch.object(BUILD, "package_route"):
                BUILD.build(output, release=True, fast=False)
                self.assertEqual(prepare.call_args[1].get("dx_lanes"), [])
                run.assert_called_once()

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
            (web / "assets/m1.css").write_text("body{}")
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
                self.assertTrue((site / "assets/m1.css").is_file())
                for name in ("layout-generators.js", "layout-generators", "preview-generator"):
                    self.assertFalse((site / "assets" / name).exists(), name)
                self.assertTrue((site / "assets/imported-modules.json").is_file())
                self.assertTrue((site / "assets/cad/cad.js").is_file())
                self.assertTrue((site / "assets/renderer/renderer.js").is_file())
                self.assertTrue((site / "assets/core-worker/entry.js").is_file())
                self.assertTrue((site / "assets/cad-worker/entry.js").is_file())
                self.assertTrue((site / "service-worker.js").is_file())

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

if __name__ == "__main__":
    unittest.main()
