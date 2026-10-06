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
    def test_provider_commands_use_retained_build_steps(self):
        commands = [" ".join(command) for command, _ in BUILD.provider_commands()]
        joined = "\n".join(commands)
        for retained in (
            "wasm-pack build core", "--features core-worker", "--features cad-worker",
            "pnpm --dir cad run build:wasm",
            "--example prepare_demo_projects", "stage-ergogen-models.py",
        ):
            self.assertIn(retained, joined)

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

    def test_build_assembles_root_and_pages_subpath_from_dirty_worktree(self):
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

            def fake_embed(site, route, _temp):
                (site / "service-worker.js").write_text(route)

            with patch.object(BUILD, "ROOT", root), patch.object(BUILD, "WEB", web), \
                 patch.object(BUILD, "dx_public", lambda _release: public), patch.object(BUILD, "run", fake_run), \
                 patch.object(BUILD, "embed_offline_worker", fake_embed):
                output = BUILD.build(root / "output", prepare_providers=False)

            self.assertEqual(len(calls), 2)
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

    def test_offline_worker_manifest_contains_module_glue_and_scoped_assets(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            site = root / "site"
            site.mkdir()
            (site / "index.html").write_text("shell")
            worker_output = root / "offline-root"

            def fake_run(command, *, cwd=BUILD.ROOT, env=None):
                worker_output.mkdir(parents=True, exist_ok=True)
                (worker_output / "boardstudio_offline_worker.js").write_text("export function initSync(){}")
                (worker_output / "boardstudio_offline_worker_bg.wasm").write_bytes(b"wasm")
                manifest = Path(env["BOARDSTUDIO_OFFLINE_MANIFEST"])
                payload = __import__("json").loads(manifest.read_text())
                self.assertIn("index.html", payload["assets"])
                self.assertIn("service-worker.js", payload["assets"])
                self.assertIn("boardstudio_offline_worker.js", payload["assets"])

            with patch.object(BUILD, "ROOT", root), patch.object(BUILD, "run", fake_run):
                BUILD.embed_offline_worker(site, "root", root)
            bootstrap = (site / "service-worker.js").read_text()
            self.assertIn('from "./boardstudio_offline_worker.js"', bootstrap)
            self.assertIn("initSync", bootstrap)
            self.assertTrue((site / "boardstudio_offline_worker.js").is_file())


if __name__ == "__main__":
    unittest.main()
