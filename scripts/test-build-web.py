#!/usr/bin/env python3
"""Focused tests for ordinary Dioxus release assembly."""

import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location("build_web", Path(__file__).with_name("build-web.py"))
BUILD = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BUILD)


class BuildWebTests(unittest.TestCase):
    def test_provider_commands_use_retained_build_steps(self):
        commands = [" ".join(command) for command in BUILD.provider_commands()]
        joined = "\n".join(commands)
        self.assertNotIn("wasm-pack build core", joined)
        for retained in (
            "--features cad-worker", "--features service-worker",
            "cad/scripts/build-cad-wasm.py",
            "stage-ergogen-models.py",
        ):
            self.assertIn(retained, joined)

    def test_fast_provider_commands_skip_wasm_opt_and_the_shipped_ones_keep_it(self):
        def builds(fast):
            return [command for command in BUILD.provider_commands(fast)
                    if command[:2] == ["wasm-pack", "build"]]

        self.assertEqual(len(builds(True)), 3)
        self.assertTrue(all("--no-opt" in command for command in builds(True)))
        self.assertTrue(all("--no-opt" not in command for command in builds(False)))
        for command in builds(True):
            self.assertNotIn("--release", command)
            profile = command.index("--profile")
            self.assertEqual(command[profile + 1], "wasm-check")
        self.assertTrue(all("--release" in command and "--profile" not in command for command in builds(False)))
        cad = [command for command in BUILD.provider_commands(True) if "cad/scripts/build-cad-wasm.py" in command]
        self.assertEqual(cad, [[BUILD.sys.executable, "cad/scripts/build-cad-wasm.py", "--no-opt"]])

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

    def test_site_and_dev_assets_stage_the_same_providers_and_worker_entries(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for path in ("web/assets/workbench-base.css", "catalogue/modules/imported-modules.json",
                         "cad/wasm/pkg/cad.js", "renderer/pkg/renderer.js",
                         "web/target/providers/cad-worker/m1_cad_worker.js"):
                (root / path).parent.mkdir(parents=True, exist_ok=True)
                (root / path).write_text(path)
            site = root / "site"
            BUILD._stage_provider_entries(site / "assets", root, include_site_assets=True)
            BUILD._stage_provider_entries(root / "web/assets", root)
            dev = root / "web/assets"
            self.assertTrue((site / "assets/workbench-base.css").is_file())
            self.assertTrue((site / "assets/imported-modules.json").is_file())
            self.assertFalse((dev / "imported-modules.json").exists(), "dx serve's copy is prepare_runtime_providers'")
            for relative in ("cad/cad.js", "renderer/renderer.js", "cad-worker/m1_cad_worker.js",
                             "cad-worker/entry.js"):
                self.assertEqual((site / "assets" / relative).read_text(), (dev / relative).read_text(), relative)
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

    def test_checked_build_stages_providers_and_css_before_both_routes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            public = root / "public"
            events = []
            tools = (Path("tailwindcss"), Path("wasm-bindgen"), "dx")

            def page(route, tools, public):
                events.append(route)
                public.mkdir()
                (public / "index.html").write_text(route)

            with patch.object(BUILD, "PAGE_PUBLIC", public), \
                 patch.object(BUILD, "page_tools", return_value=tools), \
                 patch.dict(os.environ), \
                 patch.object(BUILD, "prepare_runtime_providers", side_effect=lambda fast: events.append(fast)), \
                 patch.object(BUILD, "run", side_effect=lambda command, **kw: events.append(command)), \
                 patch.object(BUILD, "plain_page", page), \
                 patch.object(BUILD, "_stage_provider_entries"), patch.object(BUILD, "embed_offline_worker"):
                os.environ.pop(BUILD.IN_OVERLAP_ENV, None)
                BUILD.build(root / "output", release=False, fast=True, routes=("subpath", "root"))
            self.assertEqual(events, [
                True, ["tailwindcss", "-i", "tailwind.css", "-o", "assets/tailwind.css"],
                "subpath", "root",
            ])
            self.assertEqual((root / "output/site-subpath/boardstudio/index.html").read_text(), "subpath")
            self.assertEqual((root / "output/site-root/index.html").read_text(), "root")


if __name__ == "__main__":
    unittest.main()
