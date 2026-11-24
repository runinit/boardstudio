#!/usr/bin/env python3
"""Focused tests for ordinary Dioxus release assembly."""

import hashlib
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
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)

    def write(self, relative, content):
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(content)
        return path

    def test_provider_commands_use_retained_build_steps(self):
        joined = "\n".join(" ".join(command) for command in BUILD.provider_commands())
        self.assertNotIn("wasm-pack build core", joined)
        for retained in ("--features cad-worker", "--features service-worker",
                         "cad/scripts/build-cad-wasm.py", "stage-ergogen-models.py"):
            self.assertIn(retained, joined)

    def test_fast_provider_commands_skip_wasm_opt_and_the_shipped_ones_keep_it(self):
        def builds(fast):
            return [command for command in BUILD.provider_commands(fast)
                    if command[:2] == ["wasm-pack", "build"]]

        fast, release = builds(True), builds(False)
        self.assertEqual(len(fast), 3)
        for command in fast:
            self.assertIn("--no-opt", command)
            self.assertNotIn("--release", command)
            profile = command.index("--profile")
            self.assertEqual(command[profile + 1], "wasm-check")
        for command in release:
            self.assertNotIn("--no-opt", command)
            self.assertIn("--release", command)
            self.assertNotIn("--profile", command)
        cad = [command for command in BUILD.provider_commands(True) if "cad/scripts/build-cad-wasm.py" in command]
        self.assertEqual(cad, [[BUILD.sys.executable, "cad/scripts/build-cad-wasm.py", "--no-opt"]])

    def test_bindgen_lookup_prefers_local_patched_pair(self):
        tools = BUILD.wasm_bindgen_tools
        root = self.root
        binary = self.write("patched/wasm-bindgen", b"")
        fallback = self.write("fallback-bindgen", b"")
        with patch.dict(os.environ, {"XDG_CACHE_HOME": str(root)}, clear=True), \
             patch.object(tools, "locked_version", return_value="1"), \
             patch.object(tools, "validate_binary") as validate, \
             patch.object(tools, "local_patched_pair", return_value=binary.parent) as local, \
             patch.object(tools.shutil, "which", return_value=None) as which:
            self.assertEqual(tools.find_bindgen(root), binary)
            local.assert_called_once_with(root, root, "1")
            validate.assert_called_once_with(binary, "1")
            validate.reset_mock()
            local.side_effect = ValueError("bad cache key")
            which.return_value = str(fallback)
            self.assertEqual(tools.find_bindgen(root), fallback)
            validate.assert_called_once_with(fallback, "1")

    def test_plain_page_matches_the_dx_runtime_contract(self):
        bindgen_js = ("let wasmModule, wasmInstance, wasm;\nfunction __wbg_get_imports() {}\n"
                      "function initSync(module) {}\nasync function __wbg_init(module_or_path) {}\n"
                      "export { initSync, __wbg_init as default };\n")
        for route, base, meta in (("subpath", "/boardstudio/", '<meta name="DIOXUS_ASSET_ROOT" content="boardstudio">'),
                                  ("root", "/", None)):
            with self.subTest(route=route):
                root = self.root / route
                cargo_wasm = self.write(f"{route}/target/boardstudio-web.wasm", b"cargo")
                tools = (root / "tailwindcss", root / "wasm-bindgen", str(root / "dx"))
                for tool in tools[1:]:
                    Path(tool).touch()
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
                        (out / "boardstudio-web_bg.wasm").write_bytes(b"bound")

                with patch.object(BUILD, "cargo_page_artifact", return_value=cargo_wasm), \
                     patch.object(BUILD, "run", fake_run):
                    BUILD.plain_page(route, tools, public)
                    BUILD.plain_page(route, tools, public)
                self.assertEqual(cargo_wasm.read_bytes(), b"cargo")
                self.assertEqual([command[0] for command in calls], [tools[2], str(tools[1])])
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
                self.assertEqual("DIOXUS_ASSET_ROOT" in index, meta is not None)
                self.assertIn(meta or '<div id="main"></div>', index)

                with patch.object(BUILD, "cargo_page_artifact", return_value=cargo_wasm), \
                     patch.object(BUILD, "run", fake_run):
                    def assert_miss(current_tools):
                        calls.clear()
                        BUILD.plain_page(route, current_tools, public)
                        self.assertEqual(len(calls), 2)
                    for product in ("assets/focus-trap-dxh0.js", "wasm/boardstudio-web.js",
                                    "wasm/boardstudio-web_bg.wasm", "index.html"):
                        with self.subTest(missing=product):
                            (public / product).unlink()
                            assert_miss(tools)
                    cargo_wasm.write_bytes(b"changed")
                    assert_miss(tools)
                    alternate = root / "alternate-bindgen"
                    alternate.touch()
                    assert_miss((tools[0], alternate, tools[2]))

    def test_worker_builds_use_a_separate_cargo_target(self):
        root = self.root
        with patch.object(BUILD, "ROOT", root), patch.object(BUILD, "WEB", root / "web"), \
             patch.object(BUILD, "OFFLINE_WORKER", root / "web/target/providers/offline-worker"), \
             patch.object(BUILD, "run") as run, \
             patch.object(BUILD, "locked_wasm_bindgen", return_value=None), \
             patch.object(BUILD, "bake_bundled_step"), \
             patch.object(BUILD, "_stage_provider_entries"):
            BUILD.prepare_runtime_providers(fast=True)
        self.assertEqual(run.call_count, 5)
        for call, target in zip(run.call_args_list[:2], ("wasm-workers", "wasm-offline-worker"), strict=True):
            self.assertEqual(call.kwargs["env"]["CARGO_TARGET_DIR"], str(root / "target" / target))
        self.assertNotIn("env", run.call_args_list[2].kwargs)

    def test_two_fast_builds_skip_staging_and_packaging_when_unchanged(self):
        root, web = self.root, self.root / "web"
        self.write(BUILD.MODULE_CATALOGUE, b"{}")
        public = root / "page"
        tools = (Path("tailwind"), Path("bindgen"), "dx")
        def stage(assets, provider_root, **kwargs):
            (assets / "cad").mkdir(parents=True, exist_ok=True)
            (assets / "cad/provider.js").write_text("provider")
        def page(route, tools, destination):
            destination.mkdir(parents=True, exist_ok=True)
            index = destination / "index.html"
            if not index.exists():
                index.write_text("page")
        with patch.object(BUILD, "ROOT", root), patch.object(BUILD, "WEB", web), \
             patch.object(BUILD, "OFFLINE_WORKER", web / "target/providers/offline-worker"), \
             patch.object(BUILD, "PAGE_TARGET", public), patch.object(BUILD, "page_tools", return_value=tools), \
             patch.object(BUILD, "locked_wasm_bindgen", return_value=None), \
             patch.object(BUILD, "bake_bundled_step"), patch.object(BUILD, "run"), \
             patch.object(BUILD, "plain_page", side_effect=page), \
             patch.object(BUILD, "_stage_provider_entries", side_effect=stage) as staged, \
             patch.object(BUILD, "embed_offline_worker") as packaged:
            for _ in range(2):
                BUILD.build(root / "output", release=False, fast=True)
            self.assertEqual(staged.call_count, 2)  # one dev stage and one package stage
            self.assertEqual(packaged.call_count, 1)

    def test_fast_provider_rebinds_only_when_wasm_tool_or_output_changes(self):
        root = self.root
        artifact = self.write("target/wasm32-unknown-unknown/wasm-check/boardstudio_web.wasm", b"one")
        bindgen = self.write("bindgen", b"tool")
        output = root / "out"
        calls = []
        def run(command, **kwargs):
            calls.append(command)
            if command[0] == str(bindgen):
                output.mkdir(exist_ok=True)
                (output / "worker.js").write_text("js")
                (output / "worker_bg.wasm").write_bytes(b"bound")
        with patch.object(BUILD, "WEB", root / "web"), patch.object(BUILD, "run", side_effect=run):
            def build():
                BUILD.fast_provider("worker", output, "boardstudio-web", "cad-worker", root / "target", bindgen)
            build()
            build()
            self.assertEqual(sum(call[0] == str(bindgen) for call in calls), 1)
            self.assertFalse((output.parent / ".worker.bindgen.sha256").exists())
            self.assertTrue((root / "web/target/providers/.worker.bindgen.sha256").is_file())
            artifact.write_bytes(b"two")
            build()
            bindgen.write_bytes(b"new tool")
            build()
            (output / "worker.js").write_text("stale")
            build()
            (output / "worker.js").unlink()
            build()
            self.assertEqual(sum(call[0] == str(bindgen) for call in calls), 5)

    def test_fast_packaging_reuses_unchanged_site_and_repairs_changes(self):
        root = self.root
        index = self.write("public/index.html", b"first")
        public = index.parent
        with patch.object(BUILD, "ROOT", root), patch.object(BUILD, "WEB", root / "web"), \
             patch.object(BUILD, "OFFLINE_WORKER", root / "worker"), \
             patch.object(BUILD, "_stage_provider_entries"), \
             patch.object(BUILD, "embed_offline_worker") as embed:
            for _ in range(2):
                BUILD.package_route(root / "output", public, "subpath", fast=True)
            self.assertEqual(embed.call_count, 1)
            index.write_text("second")
            BUILD.package_route(root / "output", public, "subpath", fast=True)
            self.assertEqual(embed.call_count, 2)
            (root / "output/site-subpath/boardstudio/index.html").unlink()
            BUILD.package_route(root / "output", public, "subpath", fast=True)
            self.assertEqual(embed.call_count, 3)

    def test_bake_stamp_checks_inputs_and_output_sidecars(self):
        root = self.root
        providers = root / "web/target/providers"
        baked = self.write("web/target/providers/bundled-step/sidecar.bstep", b"sidecar").parent
        model = self.write("models/one.step", b"step")
        (providers / "ergogen-models.json").write_text(json.dumps({"destination": str(model.parent),
            "entries": [{"mediaType": "model/step", "emittedPath": "one.step"}]}))
        for file in ("cad/scripts/bake-bundled-step.cjs", "cad/wasm/tests/load-provider.cjs",
                     "cad/wasm/pkg/boardstudio_cad_wasm.js", "cad/wasm/pkg/boardstudio_cad_wasm_bg.wasm"):
            self.write(file, b"input")
        (baked / "manifest.json").write_text(json.dumps({"assets": {"source": "sidecar"}}))
        with patch.object(BUILD, "ROOT", root), patch.object(BUILD, "WEB", root / "web"), \
             patch.object(BUILD, "run") as run:
            BUILD.bake_bundled_step()
            BUILD.bake_bundled_step()
            self.assertEqual(run.call_count, 1)
            model.write_bytes(b"changed")
            BUILD.bake_bundled_step()
            (baked / "sidecar.bstep").write_bytes(b"changed")
            BUILD.bake_bundled_step()
            self.assertEqual(run.call_count, 3)

    def test_site_and_dev_assets_stage_the_same_providers_and_worker_entries(self):
        root = self.root
        for path in ("web/assets/workbench-base.css", "web/assets/nested/modules/keep.txt",
                     "cad/wasm/pkg/cad.js", "renderer/pkg/renderer.js",
                     "web/target/providers/cad-worker/m1_cad_worker.js"):
            self.write(path, path.encode())
        (catalogue := root / BUILD.MODULE_CATALOGUE).parent.mkdir(parents=True)
        catalogue.symlink_to(BUILD.ROOT / BUILD.MODULE_CATALOGUE)
        site, dev = root / "site", root / "web/assets"
        for stale in ("imported-modules.json", "modules/stale.json"):
            self.write("web/assets/" + stale, b"stale")
        BUILD._stage_provider_entries(site / "assets", root, include_site_assets=True)
        BUILD._stage_provider_entries(dev, root)
        for assets in (dev, site / "assets"):
            self.assertFalse(any((assets / stale).exists() for stale in ("imported-modules.json", "modules/stale.json")))
        self.assertTrue(all((site / "assets" / p).is_file() for p in ("nested/modules/keep.txt", "workbench-base.css")))
        index = (dev / "modules-index.json").read_bytes()
        self.assertEqual(index, (site / "assets/modules-index.json").read_bytes())
        self.assertLess(len(index), 200 * 1024)
        index, source = json.loads(index), json.loads(catalogue.read_text())
        self.assertEqual(index["formatVersion"], source["formatVersion"])
        for entry, record in zip(index["modules"], source["modules"], strict=True):
            self.assertEqual(set(entry), {"id", "row", "name", "family", "variant", "asset", "bounds",
                                          "protocol", "pinoutSummary", "gateCount"})
            self.assertEqual(entry["id"], record["definition"]["id"])
            self.assertEqual(json.loads((root / "web" / entry["asset"]).read_text()), record)
        host = json.loads((root / "web" / index["hostConnectorAsset"]).read_text())
        self.assertEqual(host["hardwareProfile"]["vikRole"], "host")
        self.assertIn("horizontal", host["name"].lower())
        for relative in ("cad/cad.js", "renderer/renderer.js", "cad-worker/m1_cad_worker.js",
                         "cad-worker/entry.js"):
            self.assertEqual((site / "assets" / relative).read_text(), (dev / relative).read_text(), relative)
        self.assertEqual((dev / "cad-worker/entry.js").read_text(),
                         'import init, { start_cad_worker } from "./m1_cad_worker.js";\nawait init();\n'
                         'start_cad_worker(new URL("../cad/boardstudio_cad_wasm.js", import.meta.url).href);\n')
        defn = {"id": "m0", "name": "n", "family": "f", "variant": "v", "board": {"contours": []}, "electrical": {"protocol": "i2c"}}
        host = {"definitions": [{"name": "Horizontal", "hardwareProfile": {"vikRole": "host"}}]}
        for mods, exc in (([{"row": "r", "definition": defn}], ValueError),
                          ([{"row": "r", "definition": {**defn, "circuit": host}}] * 2, FileExistsError),
                          ([{"row": "r", "definition": {**defn, "id": f"m{i}", "name": "x" * 600, "circuit": host}} for i in range(400)], ValueError)):
            (src := root / "bad.json").write_text(json.dumps({"formatVersion": 1, "modules": mods}))
            self.assertRaises(exc, BUILD.stage_module_catalogue, src, root / "bad-out")

    def test_baked_step_staging_uses_only_current_manifest_files(self):
        root = self.root
        source = self.write("web/target/providers/bundled-step/old.bstep", b"old").parent
        worker = self.write("site/assets/cad-worker/bundled-step/old.bstep", b"old").parent.parent
        original, digest = (hashlib.sha256(data).hexdigest() for data in (b"original STEP", b"preview payload"))
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
        site = self.write("site/index.html", b"shell").parent
        self.write("site/assets/app.wasm", b"app")
        self.write("site/assets/modules-index.json", b"{}")
        optional = ["assets/ergogen-models/m.step", "assets/cad/kernel.wasm",
                    "assets/cad-worker/bundled-step/m.bstep", "assets/modules/m.json"]
        for relative in optional:
            self.write("site/" + relative, b"on demand")
        worker = self.write("offline-worker/boardstudio_offline_worker.js", b"export function initSync(){}").parent
        self.write("offline-worker/boardstudio_offline_worker_bg.wasm", b"wasm")
        BUILD.embed_offline_worker(site, worker)
        bootstrap = (site / "service-worker.js").read_text()
        self.assertIn('import { initSync, start_offline_worker } from "./boardstudio_offline_worker.js"',
                      bootstrap)
        call = bootstrap.splitlines()[-1]
        self.assertTrue(call.startswith("start_offline_worker(") and call.endswith(");"), call)
        version, assets, prefixes = json.loads("[" + call[len("start_offline_worker("):-2] + "]")
        self.assertEqual(prefixes, [relative.rsplit("/", 1)[0] + "/" for relative in optional])
        self.assertRegex(version, r"^boardstudio-[0-9a-f]{16}-[0-9]+$")
        self.assertEqual(assets, sorted({"index.html", "assets/app.wasm", "assets/modules-index.json",
                                         "service-worker.js", "boardstudio_offline_worker.js"}))
        digest = hashlib.sha256(b"wasm")
        for relative in sorted([*optional, *(path for path in assets if path != "service-worker.js")]):
            digest.update(relative.encode())
            digest.update((site / relative).read_bytes())
        self.assertEqual(version.split("-")[1], digest.hexdigest()[:16])
        self.assertTrue((site / "boardstudio_offline_worker.js").is_file())

    def test_offline_manifest_validation_matches_the_worker(self):
        valid = {"version": "boardstudio-1", "shellAssets": ["index.html", "assets/a.wasm"],
                 "runtimeCachePrefixes": ["assets/modules/"]}
        BUILD.validate_offline_manifest(valid)
        for patch_fields in ({"version": "a b"}, {"shellAssets": ["app.js"]},
                             *({"shellAssets": ["index.html", path]} for path in
                               ("index.html", "../x", "a?b", "assets/modules/m.json")),
                             *({"runtimeCachePrefixes": prefixes} for prefixes in
                               (["assets"], ["../"], ["assets/", "assets/"]))):
            with self.assertRaises(ValueError, msg=patch_fields):
                BUILD.validate_offline_manifest(valid | patch_fields)

    def test_first_paint_head_matches_the_runtime_theme_contract(self):
        head = BUILD.FIRST_PAINT_THEME_HEAD
        self.assertIn("<!--boardstudio:first-paint-theme-->", head)
        for token in ("boardstudio:v2:appearance", "boardstudio:v2:theme",
                      "boardstudio:v2:workbench-theme", "prefers-color-scheme: dark",
                      'setAttribute("data-appearance"', 'setAttribute("data-theme"',
                      "colorScheme", "try{", "}catch(e){}"):
            self.assertIn(token, head)
        # The script resolves appearance first, falls back to the legacy theme
        # key, then to the media query; the workbench theme defaults to original.
        script = head.split("<script>")[1]
        self.assertLess(script.index("boardstudio:v2:appearance"),
                        script.index("prefers-color-scheme"))
        index = BUILD.index_html("/boardstudio/")
        self.assertIn(head, index)
        self.assertLess(index.index("boardstudio:first-paint-theme"), index.index('<div id="main">'))

    def test_inject_first_paint_head_targets_stylesheets_and_stays_idempotent(self):
        dx_html = ("<!DOCTYPE html><html><head>"
                   '<meta charset="UTF-8"><link rel="stylesheet" href="assets/tailwind.css">'
                   "</head><body><div id=\"main\"></div></body></html>")
        once = BUILD.inject_first_paint_head(dx_html)
        self.assertEqual(once.count("boardstudio:first-paint-theme"), 1)
        self.assertLess(once.index("boardstudio:first-paint-theme"), once.index('rel="stylesheet"'))
        self.assertEqual(BUILD.inject_first_paint_head(once), once)
        self.assertEqual(BUILD.inject_first_paint_head(BUILD.index_html("/")), BUILD.index_html("/"))
        self.assertEqual(BUILD.inject_first_paint_head("<p>no head</p>"), "<p>no head</p>")

    def test_package_route_injects_first_paint_head_into_release_html(self):
        root = self.root
        public = self.write("public/index.html",
                            ("<!DOCTYPE html><html><head><title>BoardStudio</title>"
                             '<link rel="stylesheet" href="assets/app.css">'
                             "</head><body></body></html>").encode()).parent
        with patch.object(BUILD, "ROOT", root), patch.object(BUILD, "WEB", root / "web"), \
             patch.object(BUILD, "OFFLINE_WORKER", root / "worker"), \
             patch.object(BUILD, "_stage_provider_entries"), \
             patch.object(BUILD, "embed_offline_worker"):
            for _ in range(2):
                BUILD.package_route(root / "output", public, "subpath")
        for site in ("site-subpath/boardstudio",):
            packaged = (root / "output" / site / "index.html").read_text()
            self.assertEqual(packaged.count("boardstudio:first-paint-theme"), 1)
            self.assertLess(packaged.index("boardstudio:first-paint-theme"),
                            packaged.index('rel="stylesheet"'))

    def test_routes_option_rejects_unknown_routes(self):
        with self.assertRaises(SystemExit):
            BUILD.main(["--routes", "elsewhere"])

    def test_checked_build_stages_providers_and_css_before_both_routes(self):
        root = self.root
        public = root / "public"
        events = []
        tools = (Path("tailwindcss"), Path("wasm-bindgen"), "dx")
        def page(route, tools, public):
            events.append(route)
            public.mkdir(parents=True)
            (public / "index.html").write_text(route)
        with patch.object(BUILD, "PAGE_TARGET", public), \
             patch.object(BUILD, "page_tools", return_value=tools), \
             patch.dict(os.environ), \
             patch.object(BUILD, "prepare_runtime_providers", side_effect=lambda fast: events.append(fast)), \
             patch.object(BUILD, "run", side_effect=lambda command, **kw: events.append(command)), \
             patch.object(BUILD, "plain_page", page), \
             patch.object(BUILD, "_stage_provider_entries"), patch.object(BUILD, "embed_offline_worker"):
            os.environ.pop(BUILD.IN_OVERLAP_ENV, None)
            BUILD.build(root / "output", release=False, fast=True, routes=("subpath", "root"))
        self.assertEqual(events, [True, ["tailwindcss", "-i", "tailwind.css", "-o", "assets/tailwind.css"],
                                  "subpath", "root"])
        for route, site in (("subpath", "site-subpath/boardstudio"), ("root", "site-root")):
            self.assertEqual((root / "output" / site / "index.html").read_text(), route)


if __name__ == "__main__":
    unittest.main()
