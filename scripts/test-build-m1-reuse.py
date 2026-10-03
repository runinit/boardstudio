#!/usr/bin/env python3
"""Fail-closed tests for guarded page-only provider reuse."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
from tempfile import TemporaryDirectory
from unittest import TestCase, main
from unittest.mock import patch

from importlib.util import module_from_spec, spec_from_file_location

SPEC = spec_from_file_location("build_m1_reuse", Path(__file__).with_name("build-m1.py"))
BUILD = module_from_spec(SPEC)
SPEC.loader.exec_module(BUILD)

TOOLS = {
    "rustc-version": "rustc 1.88.0",
    "cargo-version": "cargo 1.88.0",
    "dx-version": "dx 0.7.3",
    "wasm-pack-version": "wasm-pack 0.13.1",
    "node-version": "v22.0.0",
    "pnpm-version": "10.0.0",
}
PROOF = {path: b"unchanged page-only dependency proof" for path in BUILD.PAGE_ONLY_PROOF_PATHS}
PROOF["web/Cargo.toml"] = b'''\
[package]
name = "boardstudio-web"

[[bin]]
name = "boardstudio-web"
required-features = ["page"]

[features]
page = []
core-worker = []
cad-worker = []
service-worker = []
'''
PROOF["web/src/presentation.rs"] = b"mod keymap;\nmod objects;\nmod layout_workspace;\nmod library;\n"
PROOF["web/src/main.rs"] = b'#[cfg(feature = "page")]\nmod presentation;\n'
PROOF["web/src/lib.rs"] = (
    b'#[cfg(feature = "page")]\nmod presentation;\n'
    b'#[cfg(feature = "page")]\nmod runtime;\n'
    b'#[cfg(any(feature = "page", feature = "cad-worker"))]\nmod cad_jobs;\n'
    b'#[cfg(feature = "core-worker")]\nmod core_worker;\n'
    b'#[cfg(feature = "cad-worker")]\nmod cad_worker;\n'
    b'#[cfg(feature = "service-worker")]\nmod service_worker;\n'
    b'mod offline;\n'
)
PROOF["web/src/objects.rs"] = b""
SOURCE_BYTES = {
    **PROOF,
    "web/src/runtime.rs": b"pub fn runtime() {}\n",
    "web/src/cad_jobs.rs": b"pub fn jobs() {}\n",
    "web/src/core_worker.rs": b"pub fn worker() {}\n",
    "web/src/cad_worker.rs": b"pub fn worker() {}\n",
    "web/src/service_worker.rs": b"pub fn worker() {}\n",
    "web/src/offline.rs": b"pub fn offline() {}\n",
    "core/tests/electrical_wiring.rs": b"#[test]\nfn test_only() {}\n",
    "core/src/lib.rs": b"pub fn core_runtime() {}\n",
    "web/src/presentation/panels.rs": b"panels before",
    "web/src/presentation/panels_scroll_tests.rs": b"scroll tests before",
    "web/src/presentation/layout_workspace.rs": b'#[cfg(target_arch = "wasm32")]\n#[path = "layout.rs"]\nmod existing;\ninclude!("leaf.rs");\n',
    "web/src/presentation/layout.rs": b"pub fn layout() {}\n",
    "web/src/presentation/leaf.rs": b"pub fn included_leaf() {}\n",
    "web/src/presentation/objects/layout_toolbar.rs": b"layout toolbar before",
    "web/src/presentation/objects/layout_transform_toolbar.rs": b"layout transform toolbar before",
    "web/src/presentation/keymap.rs": b"mod binding_editor;\n",
    "web/src/presentation/keymap/binding_editor.rs": b"pub fn binding_editor() { let label = \"before\"; }\n",
    "web/src/presentation/objects.rs": b"mod layout_align_geometry;\nmod layout_toolbar;\n",
    "web/src/presentation/objects/layout_align_geometry.rs": b"pub fn geometry() {}\n",
    "web/src/presentation/objects/layout_toolbar.rs": b"pub fn toolbar() {}\n",
    "web/src/presentation/workspace_composition.rs": b"workspace composition before",
    "web/src/presentation/pcb_wiring/part_input_settings.rs": b"part input inspector before",
    "web/src/parts_definition_name.rs": b"parts definition name editor before",
    "web/assets/m1.css": b"css before",
    "web/src/runtime.rs": b"shared runtime",
    "web/Cargo.lock": b"locked dependencies",
    "ergogen/generated/catalogue.mjs": b"consumed generated catalogue",
    ".cargo/config.toml": b"[build]\nrustflags = ['-C', 'target-cpu=native']\n",
    ".npmrc": b"strict-peer-dependencies=true\n",
    ".node-version": b"22.0.0\n",
    BUILD.REUSE_HELPER_PATH: b"current fixture helper source",
    BUILD.FIXTURE_PREPARATION_PATH: b"fixture preparation before",
    "web/src/presentation/library.rs": b"pub fn library() {}\n",
}
HEAD_BYTES = {**SOURCE_BYTES,
              "web/src/presentation/panels.rs": b"panels after",
              "web/src/presentation/panels_scroll_tests.rs": b"scroll tests after",
              "web/src/presentation/objects/layout_toolbar.rs": b"layout toolbar after",
              "web/assets/m1.css": b"css after"}
HEAD_BYTES["web/src/presentation/keymap/binding_editor.rs"] = b"pub fn binding_editor() { let label = \"after\"; }\n"
HEAD_BYTES[BUILD.REUSE_HELPER_PATH] = b"fixture refresh helper source"
HEAD_BYTES[BUILD.FIXTURE_PREPARATION_PATH] = b"fixture preparation after"
HEAD_BYTES["web/src/presentation/library.rs"] = b"pub fn library() { let vik = true; }\n"


def sha(data):
    return hashlib.sha256(data).hexdigest()


class PageOnlyReuseTests(TestCase):
    def make_baseline(self, root, *, extra_asset=None):
        build_root = root / "web/target/builds"
        baseline = build_root / "full-fixture"
        baseline.mkdir(parents=True)
        sources = {name: sha(body) for name, body in SOURCE_BYTES.items()}
        sources[BUILD.REUSE_HELPER_PATH] = next(iter(BUILD.COMPATIBLE_FULL_BUILD_HELPERS))[1]
        commands = []
        for label in (
            "rustc-version", "cargo-version", "dx-version", "wasm-pack-version", "node-version", "pnpm-version",
            "core", "core-worker", "cad-worker", "renderer", "cad", "fixtures", "ergogen-catalogue",
            "layout-generators", "preview-generator", "ergogen-models", "page-root", "offline-worker-root",
            "embed-offline-root", "page-subpath", "offline-worker-subpath", "embed-offline-subpath",
        ):
            log = baseline / f"{label}.log"
            log.write_text(TOOLS[label] if label in TOOLS else f"{label} succeeded\n")
            command = {"argv": self.full_argv(label, root, baseline), "exit": 0, "log": str(log),
                       "cwd": str(root / "web" if label in ("page-root", "page-subpath") else root), "environment": {}}
            if label.startswith("offline-worker-"):
                mode = label.removeprefix("offline-worker-")
                command["environment"] = {"BOARDSTUDIO_OFFLINE_MANIFEST": str(baseline / f"offline-manifest-{mode}.json")}
            commands.append(command)
        routes = {}
        for mode, prefix in (("root", "/"), ("subpath", "/boardstudio/")):
            site = baseline / ("site-root" if mode == "root" else "site-subpath/boardstudio")
            (site / "assets").mkdir(parents=True)
            (site / "index.html").write_text(f"<base href='{prefix}'>")
            (site / "assets/provider.js").write_text("provider")
            (site / "assets/stale-unrelated.js").write_text("stale baseline file")
            (site / "assets/m1.css").write_text("css before")
            (site / "assets/layout-generators").mkdir()
            (site / "assets/layout-generators/obsolete.js").write_text("stale generated output")
            (site / "assets/layout-generators.js").write_text("stale entrypoint")
            (site / "assets/preview-generator").mkdir()
            (site / "assets/preview-generator/obsolete.mjs").write_text("stale preview")
            required = (
                "assets/core-worker/entry.js", "assets/core-worker/m1_core_worker_bg.wasm",
                "assets/cad-worker/entry.js", "assets/cad-worker/m1_cad_worker_bg.wasm",
                "assets/cad/boardstudio_cadrum_wasm_bg.wasm", "assets/renderer/boardstudio_renderer_wasm_bg.wasm",
                "assets/fixtures/provenance.json", "service-worker.js", "boardstudio_offline_worker.js",
            )
            for name in required:
                path = site / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("provider")
            if extra_asset:
                (site / extra_asset).parent.mkdir(parents=True, exist_ok=True)
                (site / extra_asset).write_text("unexpected")
            routes[mode] = {
                "prefix": prefix,
                "site": str(site),
                "assets": {p.relative_to(site).as_posix(): sha(p.read_bytes()) for p in site.rglob("*") if p.is_file()},
            }
        provenance = {
            "source_commit": next(iter(BUILD.COMPATIBLE_FULL_BUILD_HELPERS))[0],
            "status": "complete",
            "sources": sources,
            "commands": commands,
            "root": routes["root"],
            "subpath": routes["subpath"],
        }
        (baseline / "provenance.json").write_text(json.dumps(provenance))
        for name, body in SOURCE_BYTES.items():
            path = root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(body)
        return baseline, provenance

    @staticmethod
    def full_argv(label, root, baseline):
        web = root / "web"
        if label.endswith("-version"):
            return {
                "rustc-version": ["rustc", "--version"], "cargo-version": ["cargo", "--version"],
                "dx-version": ["dx", "--version"], "wasm-pack-version": ["wasm-pack", "--version"],
                "node-version": ["node", "--version"], "pnpm-version": ["pnpm", "--version"],
            }[label]
        if label == "page-root" or label == "page-subpath":
            prefix = "/" if label.endswith("root") else "/boardstudio/"
            return ["dx", "build", "--web", "--release", "--base-path", prefix, "--no-default-features", "--features", "page", "--cargo-args=--locked"]
        if label.startswith("offline-worker-"):
            mode = label.removeprefix("offline-worker-")
            return ["wasm-pack", "build", str(web), "--target", "web", "--out-name", "boardstudio_offline_worker", "--out-dir", str(baseline / f"offline-{mode}"), "--release", "--locked", "--no-default-features", "--features", "service-worker"]
        if label.startswith("embed-offline-"):
            mode = label.removeprefix("embed-offline-")
            site = baseline / f"site-{mode}"
            if mode == "subpath":
                site /= "boardstudio"
            return ["node", str(root / "scripts/web/embed-worker-wasm.mjs"), str(baseline / f"offline-{mode}"), str(baseline / f"offline-manifest-{mode}.json"), str(site / "service-worker.js")]
        if label == "core-worker":
            return ["wasm-pack", "build", str(web), "--target", "web", "--out-name", "m1_core_worker", "--out-dir", str(baseline / "core-worker"), "--release", "--locked", "--no-default-features", "--features", "core-worker"]
        if label == "cad-worker":
            return ["wasm-pack", "build", str(web), "--target", "web", "--out-name", "m1_cad_worker", "--out-dir", str(baseline / "cad-worker"), "--release", "--locked", "--no-default-features", "--features", "cad-worker"]
        if label == "core":
            return ["wasm-pack", "build", str(root / "core"), "--target", "web", "--release", "--locked"]
        if label == "renderer":
            return ["wasm-pack", "build", str(root / "renderer"), "--target", "web", "--out-dir", str(baseline / "renderer"), "--out-name", "boardstudio_renderer_wasm", "--release", "--locked"]
        if label == "cad":
            return ["pnpm", "--dir", "cad", "run", "build:wasm"]
        if label == "fixtures":
            return ["node", str(root / "scripts/prepare-m1-fixtures.mjs"), str(baseline / "fixtures")]
        if label == "ergogen-catalogue":
            return ["pnpm", "--dir", "ergogen", "run", "prepare:catalog"]
        if label in ("layout-generators", "preview-generator"):
            script = "build-layout-generators.mjs" if label == "layout-generators" else "build-preview-generator.mjs"
            return ["node", str(root / "scripts/web" / script), str(web / "assets")]
        if label == "ergogen-models":
            return [sys.executable, str(root / "scripts/stage-ergogen-models.py"), "--source-root", str(root / "ergogen/library/vendor"), "--destination", str(web / "assets/ergogen-models"), "--manifest", str(baseline / "ergogen-models-catalog.json")]
        raise AssertionError(label)

    def mock_environment(self, root, provenance, current=None, head=None, *, helper_changed=False):
        current = current or {name: sha(body) for name, body in SOURCE_BYTES.items()}
        current = dict(current)
        if not helper_changed:
            current[BUILD.REUSE_HELPER_PATH] = next(iter(BUILD.COMPATIBLE_FULL_BUILD_HELPERS))[1]
        head = head or HEAD_BYTES
        for name, body in head.items():
            path = root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(body)

        def check_output(command, **kwargs):
            if command[:2] == ["git", "rev-parse"]:
                if len(command) == 3 and ":" in command[2]:
                    return next(iter(BUILD.COMPATIBLE_FULL_BUILD_HELPERS.values())) + "\n"
                return "b" * 40 if kwargs.get("text") else b"b" * 40
            if command[:3] == ["git", "cat-file", "-e"]:
                return b""
            if command[:2] == ["git", "ls-files"]:
                if "--ignored" in command:
                    return b""
                return ("\0".join(current) + "\0").encode()
            if command[0] == "git" and command[1] == "show":
                revision, path = command[2].split(":", 1)
                return (head if revision == "HEAD" else SOURCE_BYTES)[path]
            raise AssertionError(f"unexpected command in guard fixture: {command}")
        return [
            patch.object(BUILD, "REPO", root),
            patch.object(BUILD, "WEB", root / "web"),
            patch.object(BUILD, "BUILD_ROOT", root / "web/target/builds"),
            patch.object(BUILD, "sources", return_value=current),
            patch.object(BUILD, "current_tools", return_value=TOOLS),
            patch.object(BUILD.subprocess, "check_output", side_effect=check_output),
        ]

    def test_cli_help_and_invalid_arguments_have_no_output_or_build_side_effects(self):
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            script = root / "scripts/build-m1.py"
            script.parent.mkdir(parents=True)
            shutil.copy2(Path(__file__).with_name("build-m1.py"), script)
            bin_dir = root / "bin"
            bin_dir.mkdir()
            sentinel = root / "build-command-called"
            stub = bin_dir / "rustc"
            stub.write_text(f"#!/bin/sh\ntouch '{sentinel}'\nexit 0\n")
            stub.chmod(0o755)
            environment = dict(os.environ, PATH=f"{bin_dir}:{os.environ.get('PATH', '')}")
            for args, expected in ((["--help"], 0), (["build", "--unknown"], 2), (["../escape"], 2), (["build", "--reuse-providers-from"], 2)):
                result = subprocess.run([sys.executable, str(script), *args], capture_output=True, text=True, env=environment)
                self.assertEqual(result.returncode, expected, result.stderr)
                self.assertFalse((root / "web/target/builds").exists())
                self.assertFalse(sentinel.exists())
            optimized = subprocess.run([sys.executable, "-O", str(script), "../escape"], capture_output=True, text=True, env=environment)
            self.assertEqual(optimized.returncode, 2)
            self.assertFalse((root / "web/target/builds").exists())
            self.assertFalse(sentinel.exists())

    def test_full_build_retains_twenty_two_command_sequence_and_complete_provenance(self):
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            web = root / "web"
            (web / "assets").mkdir(parents=True)
            (root / "cad/wasm/pkg").mkdir(parents=True)
            (root / "cad/wasm/pkg/boardstudio_cadrum_wasm_bg.wasm").write_bytes(b"cad wasm")
            calls = []

            def executor(argv, cwd, env, stdout, stderr):
                calls.append(argv)
                if argv[0] == "dx":
                    public = web / "target/dx/boardstudio-web/release/web/public"
                    (public / "assets").mkdir(parents=True, exist_ok=True)
                    (public / "index.html").write_text("fresh page")
                elif argv[0] == "wasm-pack" and "--out-dir" in argv:
                    out = Path(argv[argv.index("--out-dir") + 1])
                    out.mkdir(parents=True, exist_ok=True)
                    name = argv[argv.index("--out-name") + 1]
                    (out / f"{name}.js").write_text("provider js")
                    (out / f"{name}_bg.wasm").write_bytes(b"provider wasm")
                elif argv[0] == "node" and Path(argv[1]).name == "prepare-m1-fixtures.mjs":
                    out = Path(argv[2])
                    (out / "provenance.json").parent.mkdir(parents=True, exist_ok=True)
                    (out / "provenance.json").write_text("fixture")
                elif argv[0] == "node" and Path(argv[1]).name == "build-layout-generators.mjs":
                    out = Path(argv[2]) / "layout-generators"
                    out.mkdir(parents=True, exist_ok=True)
                    (out / "layout.js").write_text("layout")
                    (Path(argv[2]) / "layout-generators.js").write_text("entry")
                elif argv[0] == "node" and Path(argv[1]).name == "build-preview-generator.mjs":
                    out = Path(argv[2]) / "preview-generator"
                    out.mkdir(parents=True, exist_ok=True)
                    (out / "preview.mjs").write_text("preview")
                elif Path(argv[1]).name == "stage-ergogen-models.py":
                    out = Path(argv[argv.index("--destination") + 1])
                    out.mkdir(parents=True, exist_ok=True)
                    (out / "models.json").write_text("models")
                    Path(argv[argv.index("--manifest") + 1]).write_text("manifest")
                elif argv[0] == "node" and Path(argv[1]).name == "embed-worker-wasm.mjs":
                    out = Path(argv[4])
                    out.parent.mkdir(parents=True, exist_ok=True)
                    out.write_text("service worker")
                    (out.parent / "boardstudio_offline_worker.js").write_text("offline js")
                elif argv[0] == "pnpm" and "build:wasm" in argv:
                    out = root / "cad/wasm/pkg"
                    out.mkdir(parents=True, exist_ok=True)
                elif argv[0] == "pnpm" or (argv[0] == "node" and Path(argv[1]).name == "prepare-m1-fixtures.mjs"):
                    pass
                elif argv[0] in ("rustc", "cargo", "dx", "wasm-pack", "node"):
                    pass
                else:
                    raise AssertionError(argv)
                return subprocess.CompletedProcess(argv, 0)

            with patch.object(BUILD, "REPO", root), patch.object(BUILD, "WEB", web), \
                 patch.object(BUILD, "BUILD_ROOT", web / "target/builds"), \
                 patch.object(BUILD.subprocess, "check_output", return_value="b" * 40), \
                 patch.object(BUILD, "sources", return_value={"web/src/lib.rs": sha(b"source")}), \
                 patch.object(BUILD.subprocess, "run", side_effect=executor):
                BUILD.build_full("full-stub")

            receipt = json.loads((web / "target/builds/full-stub/provenance.json").read_text())
            self.assertEqual(len(calls), 22)
            self.assertEqual(len(receipt["commands"]), 22)
            self.assertEqual(receipt["status"], "complete")
            self.assertEqual(receipt["commands"][0]["argv"], ["rustc", "--version"])
            self.assertEqual(receipt["commands"][-1]["argv"][0], "node")

    def test_only_exact_panel_and_css_delta_passes_source_preflight(self):
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.make_baseline(root)
            current = {name: sha(body) for name, body in SOURCE_BYTES.items()}
            current["web/src/presentation/panels.rs"] = sha(b"panels after")
            current["web/src/presentation/panels_scroll_tests.rs"] = sha(b"scroll tests after")
            current["web/assets/m1.css"] = sha(b"css after")
            current["web/src/presentation/objects/layout_toolbar.rs"] = sha(b"layout toolbar after")
            with self.subTest("permitted delta"), self._patches(self.mock_environment(root, {}, current)):
                checked = BUILD.validate_reuse("candidate", "full-fixture")
                self.assertEqual(checked[6], [
                    "web/assets/m1.css",
                    "web/src/presentation/objects/layout_toolbar.rs",
                    "web/src/presentation/panels.rs",
                    "web/src/presentation/panels_scroll_tests.rs",
                ])
                self.assertTrue(set(checked[6]).issubset(BUILD.PAGE_ONLY_ALLOWLIST))
                self.assertTrue(set(checked[6]).isdisjoint(BUILD.PAGE_ONLY_PROOF_PATHS))
                self.assertFalse(any(path.startswith("web/src/core_worker/") for path in checked[6]))
                self.assertFalse(checked[0].exists())

    def test_ordinary_page_ui_module_body_change_is_provider_reusable(self):
        path = "web/src/presentation/keymap/binding_editor.rs"
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.make_baseline(root)
            current = {name: sha(body) for name, body in SOURCE_BYTES.items()}
            current[path] = sha(HEAD_BYTES[path])
            with self._patches(self.mock_environment(root, {}, current, HEAD_BYTES)):
                checked = BUILD.validate_reuse("candidate", "full-fixture")
                self.assertEqual(checked[6], [path])
                self.assertFalse(checked[0].exists())

    def test_page_only_source_addition_and_cfg_main_delta_are_owned(self):
        added = "web/src/presentation/keymap/new_editor.rs"
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.make_baseline(root)
            (root / "web/src/presentation/keymap.rs").write_bytes(b"mod binding_editor;\nmod new_editor;\n")
            (root / added).parent.mkdir(parents=True, exist_ok=True)
            (root / added).write_bytes(b"pub fn new_editor() {}\n")
            current = {name: sha(body) for name, body in SOURCE_BYTES.items()}
            current["web/src/presentation/keymap.rs"] = sha(b"mod binding_editor;\nmod new_editor;\n")
            current[added] = sha(b"pub fn new_editor() {}\n")
            current[BUILD.PAGE_ONLY_MAIN_PATH] = sha(
                b'#[cfg(feature = "page")]\nmod presentation;\n'
                b'#[cfg(feature = "page")]\nmod new_page_shell;\n'
            )
            page_shell = "web/src/new_page_shell.rs"
            page_shell_body = b"pub fn shell() {}\n"
            current[page_shell] = sha(page_shell_body)
            head = dict(HEAD_BYTES)
            head["web/src/presentation/keymap.rs"] = b"mod binding_editor;\nmod new_editor;\n"
            head[added] = b"pub fn new_editor() {}\n"
            head[page_shell] = page_shell_body
            head[BUILD.PAGE_ONLY_MAIN_PATH] = (
                b'#[cfg(feature = "page")]\nmod presentation;\n'
                b'#[cfg(feature = "page")]\nmod new_page_shell;\n'
            )
            source = self.mock_environment(root, {}, current, head)
            with self._patches(source):
                checked = BUILD.validate_reuse("candidate", "full-fixture")
                self.assertIn(added, checked[6])
                self.assertIn(BUILD.PAGE_ONLY_MAIN_PATH, checked[6])

    def test_worker_overlap_feature_drift_and_nonpage_main_edit_reject(self):
        cases = (
            ("worker-overlap", "web/src/cad_jobs.rs", b"pub fn jobs() { let changed = true; }\n"),
            ("feature-input", "web/Cargo.toml", b"changed page/worker feature matrix"),
            ("nonpage-main", BUILD.PAGE_ONLY_MAIN_PATH, b"fn main() { panic!(\"changed\"); }\n"),
            ("core-runtime", "core/src/lib.rs", b"changed provider runtime"),
        )
        for label, path, body in cases:
            with self.subTest(negative=label), TemporaryDirectory() as temporary:
                root = Path(temporary)
                self.make_baseline(root)
                current = {name: sha(data) for name, data in SOURCE_BYTES.items()}
                current[path] = sha(body)
                head = dict(HEAD_BYTES)
                head[path] = body
                with self._patches(self.mock_environment(root, {}, current, head)):
                    with self.assertRaises(ValueError):
                        BUILD.validate_reuse("candidate", "full-fixture")
                self.assertFalse((root / "web/target/builds/candidate").exists())

    def test_transitive_feature_and_legacy_allowlist_cannot_hide_provider_overlap(self):
        binding = "web/src/presentation/keymap/binding_editor.rs"
        toolbar = "web/src/presentation/objects/layout_toolbar.rs"
        cases = (
            ({"web/Cargo.toml": SOURCE_BYTES["web/Cargo.toml"].replace(
                b'core-worker = []', b'core-worker = ["page"]')}, binding),
            ({"web/src/lib.rs": SOURCE_BYTES["web/src/lib.rs"] +
                b'#[cfg(feature = "core-worker")]\n'
                b'#[path = "presentation/objects/layout_toolbar.rs"]\nmod shared_toolbar;\n'}, toolbar),
        )
        for overrides, path in cases:
            with self.subTest(path=path, overrides=list(overrides)), TemporaryDirectory() as temporary:
                root = Path(temporary)
                with patch.dict(SOURCE_BYTES, overrides):
                    self.make_baseline(root)
                    head = dict(SOURCE_BYTES)
                    head[path] = b"pub fn changed_provider_input() {}\n"
                    current = {name: sha(body) for name, body in head.items()}
                    with self._patches(self.mock_environment(root, {}, current, head)):
                        with self.assertRaisesRegex(ValueError, "provider|outside page"):
                            BUILD.validate_reuse("candidate", "full-fixture")
                self.assertFalse((root / "web/target/builds/candidate").exists())

    def test_transitive_feature_closure_preserves_provider_labels(self):
        manifest = SOURCE_BYTES["web/Cargo.toml"].replace(
            b'core-worker = []', b'core-worker = ["bridge"]\nbridge = ["page"]')
        with TemporaryDirectory() as temporary, patch.dict(SOURCE_BYTES, {"web/Cargo.toml": manifest}):
            root = Path(temporary)
            self.make_baseline(root)
            with self._patches(self.mock_environment(root, {}, head=dict(SOURCE_BYTES))):
                features = BUILD.page_build_feature_configs(root)
                self.assertEqual(features[1], frozenset({"core-worker", "bridge", "page"}))
                ownership = BUILD.page_feature_ownership(root)
            self.assertEqual(set(ownership["provider_rust_inputs"]), {"core-worker", "cad-worker", "service-worker"})
            self.assertIn("web/src/presentation/keymap/binding_editor.rs", ownership["provider_rust_inputs"]["core-worker"])

    def test_literal_include_and_nested_inline_paths_are_owned_page_inputs(self):
        path = "web/src/presentation/keymap/binding_editor.rs"
        include = "web/src/presentation/keymap/included.rs"
        nested = "web/src/presentation/keymap/binding_editor/outer/inner/leaf.rs"
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.make_baseline(root)
            head = dict(SOURCE_BYTES)
            head[path] += (
                b'include!("included.rs");\n'
                b'mod outer { mod inner { #[path = "leaf.rs"] mod leaf; } }\n'
            )
            head[include] = b"pub fn included() {}\n"
            head[nested] = b"pub fn nested_leaf() {}\n"
            current = {name: sha(body) for name, body in head.items()}
            with self._patches(self.mock_environment(root, {}, current, head)):
                checked = BUILD.validate_reuse("candidate", "full-fixture")
                self.assertEqual(set(checked[6]), {path, include, nested})
                self.assertTrue({include, nested}.issubset(checked[10]["page_feature_rust_inputs"]))
            self.assertFalse((root / "web/target/builds/candidate").exists())

    def test_literal_include_alias_is_a_provider_input(self):
        path = "web/src/presentation/keymap/binding_editor.rs"
        lib = SOURCE_BYTES["web/src/lib.rs"] + (
            b'#[cfg(feature = "core-worker")]\n'
            b'include!("presentation/keymap/binding_editor.rs");\n'
        )
        with TemporaryDirectory() as temporary, patch.dict(SOURCE_BYTES, {"web/src/lib.rs": lib}):
            root = Path(temporary)
            self.make_baseline(root)
            head = dict(SOURCE_BYTES)
            head[path] = b"pub fn changed_provider_input() {}\n"
            current = {name: sha(body) for name, body in head.items()}
            with self._patches(self.mock_environment(root, {}, current, head)):
                with self.assertRaisesRegex(ValueError, "provider|outside page"):
                    BUILD.validate_reuse("candidate", "full-fixture")
            self.assertFalse((root / "web/target/builds/candidate").exists())

    def test_unresolved_include_nested_module_and_opaque_registration_fail_closed(self):
        path = "web/src/presentation/keymap/binding_editor.rs"
        cases = (
            b'include!("missing.rs");\n',
            b'include!("../../../../outside.rs");\n',
            b'mod outer { mod inner { mod missing; } }\n',
            b'macro_rules! register { () => { mod hidden; }; } register!();\n',
        )
        for registration in cases:
            with self.subTest(registration=registration), TemporaryDirectory() as temporary:
                root = Path(temporary)
                self.make_baseline(root)
                head = dict(SOURCE_BYTES)
                head[path] = SOURCE_BYTES[path] + registration
                current = {name: sha(body) for name, body in head.items()}
                with self._patches(self.mock_environment(root, {}, current, head)):
                    with self.assertRaisesRegex(ValueError, "missing|escapes|unsupported|module graph"):
                        BUILD.validate_reuse("candidate", "full-fixture")
                self.assertFalse((root / "web/target/builds/candidate").exists())

    def test_exact_core_integration_test_delta_is_not_a_provider_input(self):
        path = "core/tests/electrical_wiring.rs"
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.make_baseline(root)
            body = b"#[test]\nfn additional_integration_test() {}\n"
            current = {name: sha(data) for name, data in SOURCE_BYTES.items()}
            current[path] = sha(body)
            head = dict(HEAD_BYTES)
            head[path] = body
            with self._patches(self.mock_environment(root, {}, current, head)):
                checked = BUILD.validate_reuse("candidate", "full-fixture")
                self.assertEqual(checked[6], [path])

    def test_page_leaf_inventory_is_exact_and_excludes_worker_test_alias(self):
        expected = {
            "layout-command-pill": {
                "web/src/presentation/layout_workspace.rs",
                "web/src/presentation/objects/layout_toolbar.rs",
                "web/src/presentation/objects/layout_transform_toolbar.rs",
                "web/src/presentation/workspace_composition.rs",
            },
            "pcb-part-input-inspector": {
                "web/src/presentation/pcb_wiring/part_input_settings.rs",
            },
            "parts-definition-name-editor": {
                "web/src/parts_definition_name.rs",
            },
        }
        self.assertEqual({name: set(paths) for name, paths in BUILD.PAGE_ONLY_LEAF_PATHS.items()}, expected)
        self.assertEqual(
            BUILD.PAGE_ONLY_ALLOWLIST,
            frozenset({
                "web/src/presentation/panels.rs",
                "web/src/presentation/panels_scroll_tests.rs",
                "web/assets/m1.css",
                *(path for paths in expected.values() for path in paths),
            }),
        )
        self.assertNotIn("web/src/presentation/objects/layout_align_geometry.rs", BUILD.PAGE_ONLY_ALLOWLIST)

    def test_new_page_module_paths_must_resolve_from_the_committed_source_graph(self):
        path = "web/src/presentation/layout_workspace.rs"
        baseline = SOURCE_BYTES[path]
        cases = (
            ("module", baseline + b"\nmod\nadded;\n"),
            ("split-pub-module", baseline + b"\npub\nmod added;\n"),
            ("comment-prefix-module", baseline + b"\n// keep this comment\nmod added;\n"),
            ("path", baseline.replace(b'layout.rs', b'worker.rs')),
        )
        for label, mutated in cases:
            with self.subTest(registration=label), TemporaryDirectory() as temporary:
                root = Path(temporary)
                baseline_dir, _ = self.make_baseline(root)
                head = dict(HEAD_BYTES)
                head[path] = mutated
                current = {name: sha(data) for name, data in SOURCE_BYTES.items()}
                current[path] = sha(mutated)
                with self._patches(self.mock_environment(root, {}, current, head)):
                    with self.assertRaisesRegex(ValueError, "missing|module graph"):
                        BUILD.validate_reuse("candidate", "full-fixture")
                self.assertFalse((root / "web/target/builds/candidate").exists())
                self.assertTrue((baseline_dir / "provenance.json").exists())

    def test_unbalanced_function_and_rsx_groups_reject_before_candidate_creation(self):
        path = "web/src/presentation/layout_workspace.rs"
        baseline = SOURCE_BYTES[path]
        cases = (
            ("unmatched-brace", baseline + b"\nfn unfinished() {\n"),
            ("mismatched-group", baseline + b"\nfn malformed() ]\n"),
            ("unmatched-rsx-group", baseline + b'\nfn render() { rsx! { button { "x" } }\n'),
            ("mismatched-rsx-group", baseline + b'\nfn render() { rsx! { button { "x" ) } } }\n'),
        )
        for label, mutated in cases:
            with self.subTest(group=label), TemporaryDirectory() as temporary:
                root = Path(temporary)
                baseline_dir, _ = self.make_baseline(root)
                head = dict(HEAD_BYTES)
                head[path] = mutated
                current = {name: sha(data) for name, data in SOURCE_BYTES.items()}
                current[path] = sha(mutated)
                with self._patches(self.mock_environment(root, {}, current, head)):
                    with self.assertRaisesRegex(ValueError, "malformed Rust (token stream|module graph)"):
                        BUILD.validate_reuse("candidate", "full-fixture")
                self.assertFalse((root / "web/target/builds/candidate").exists())
                self.assertTrue((baseline_dir / "provenance.json").exists())

    def test_registration_signature_allows_ordinary_rsx_copy_edits(self):
        before = b'''\
#[component]
fn CommandPill() -> Element {
    // unmatched delimiters in comments are not Rust token groups: } ] )
    rsx! { button { "mod include! cfg [({" } }
}
'''
        after = before.replace(b"mod include! cfg [({", b"workers still compile )]}")
        self.assertEqual(
            BUILD.rust_module_registration_signature(before),
            BUILD.rust_module_registration_signature(after),
        )

    def test_docs_only_source_identity_change_can_reuse_explicitly(self):
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.make_baseline(root)
            with self._patches(self.mock_environment(root, {})):
                checked = BUILD.validate_reuse("candidate", "full-fixture")
                self.assertEqual(checked[6], [])
                self.assertFalse(checked[0].exists())

    def test_feature_ownership_comes_from_manifest_and_full_command_matrix(self):
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.make_baseline(root)
            with self._patches(self.mock_environment(root, {})):
                ownership = BUILD.page_feature_ownership(root)
            self.assertIn("web/src/presentation/keymap/binding_editor.rs", ownership["page_feature_rust_inputs"])
            self.assertIn("web/src/cad_jobs.rs", ownership["provider_rust_inputs"]["cad-worker"])
            self.assertIn("web/src/presentation/objects/layout_align_geometry.rs", ownership["explicit_non_page_aliases"])

            cargo = root / "web/Cargo.toml"
            cargo.write_text(cargo.read_text().replace('required-features = ["page"]', 'required-features = ["page", "core-worker"]'))
            with self.assertRaisesRegex(ValueError, "binary is no longer gated"):
                BUILD.page_feature_ownership(root)

    def test_feature_command_drift_rejects_graph_ownership(self):
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.make_baseline(root)
            with self._patches(self.mock_environment(root, {})):
                original = BUILD.expected_full_commands

                def wrong_page_command(baseline):
                    commands = original(baseline)
                    return [
                        (label, ["--features", "core-worker"] + argv[2:] if label == "page-root" else argv, cwd, env)
                        for label, argv, cwd, env in commands
                    ]

                with patch.object(BUILD, "expected_full_commands", side_effect=wrong_page_command):
                    with self.assertRaisesRegex(ValueError, "command feature differs"):
                        BUILD.page_feature_ownership(root)

    def test_stubbed_reuse_build_runs_eight_commands_and_emits_fresh_routes(self):
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            baseline, base = self.make_baseline(root)
            base_route_hashes = {
                mode: {name: sha((Path(base[mode]["site"]) / name).read_bytes()) for name in base[mode]["assets"]}
                for mode in ("root", "subpath")
            }
            current = {name: sha(body) for name, body in SOURCE_BYTES.items()}
            current["web/src/presentation/panels.rs"] = sha(b"panels after")
            current["web/src/presentation/panels_scroll_tests.rs"] = sha(b"scroll tests after")
            current["web/assets/m1.css"] = sha(b"css after")
            (root / "web/assets/m1.css").write_bytes(b"css after")
            run_calls = []

            def executor(argv, cwd, env, stdout, stderr):
                run_calls.append(argv)
                if argv[0] == "node" and Path(argv[1]).name in ("build-layout-generators.mjs", "build-preview-generator.mjs"):
                    destination = Path(argv[2])
                    if "layout" in argv[1]:
                        generated = destination / "layout-generators"
                        (generated / "src").mkdir(parents=True, exist_ok=True)
                        (generated / "src/index.js").write_text("fresh layout")
                        (destination / "layout-generators.js").write_text("fresh entrypoint")
                    else:
                        generated = destination / "preview-generator"
                        generated.mkdir(parents=True, exist_ok=True)
                        (generated / "worker.mjs").write_text("fresh preview")
                elif argv[0] == "dx":
                    public = root / "web/target/dx/boardstudio-web/release/web/public"
                    (public / "assets").mkdir(parents=True, exist_ok=True)
                    (public / "index.html").write_text("fresh page")
                    (public / "assets/boardstudio-web-current.js").write_text("fresh page js")
                elif argv[0] == "wasm-pack":
                    out_dir = Path(argv[argv.index("--out-dir") + 1])
                    out_dir.mkdir(parents=True, exist_ok=True)
                    (out_dir / "boardstudio_offline_worker.js").write_text("fresh offline worker")
                elif argv[0] == "node" and Path(argv[1]).name == "embed-worker-wasm.mjs":
                    Path(argv[4]).write_text("fresh embedded worker")
                else:
                    raise AssertionError(argv)
                return subprocess.CompletedProcess(argv, 0)

            patches = self.mock_environment(root, base, current)
            patches.append(patch.object(BUILD.subprocess, "run", side_effect=executor))
            with self._patches(patches):
                BUILD.build_reuse("candidate", "full-fixture")

            output = root / "web/target/builds/candidate"
            receipt = json.loads((output / "provenance.json").read_text())
            self.assertEqual(len(run_calls), 8)
            self.assertEqual(len(receipt["commands"]), 8)
            self.assertEqual(receipt["inherited_full_build_commands"], 22)
            self.assertEqual(len(receipt["inherited_full_build_lineage"]), 22)
            self.assertEqual(receipt["status"], "complete")
            for mode, prefix in (("root", "/"), ("subpath", "/boardstudio/")):
                site = Path(receipt[mode]["site"])
                manifest = json.loads((output / f"offline-manifest-{mode}.json").read_text())
                self.assertEqual(receipt[mode]["prefix"], prefix)
                self.assertEqual(manifest["version"], f"candidate-{mode}")
                self.assertIn("assets/boardstudio-web-current.js", manifest["assets"])
                self.assertEqual((site / "assets/m1.css").read_bytes(), b"css after")
                self.assertEqual((site / "service-worker.js").read_text(), "fresh embedded worker")
                self.assertFalse((site / "assets/layout-generators/obsolete.js").exists())
                self.assertFalse((site / "assets/preview-generator/obsolete.mjs").exists())
                self.assertFalse((site / "assets/stale-unrelated.js").exists())
                self.assertFalse((site / "assets/provider.js").exists())
                self.assertIn("assets/layout-generators.js", manifest["assets"])
            for mode in ("root", "subpath"):
                self.assertEqual(
                    {name: sha((Path(base[mode]["site"]) / name).read_bytes()) for name in base[mode]["assets"]},
                    base_route_hashes[mode],
                )

    def test_fixture_refresh_uses_exact_helper_proof_and_seven_fresh_commands(self):
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            baseline, base = self.make_baseline(root, extra_asset="assets/fixtures/old.json")
            current = {name: sha(body) for name, body in SOURCE_BYTES.items()}
            current[BUILD.FIXTURE_PREPARATION_PATH] = sha(HEAD_BYTES[BUILD.FIXTURE_PREPARATION_PATH])
            current["web/src/presentation/library.rs"] = sha(HEAD_BYTES["web/src/presentation/library.rs"])
            current[BUILD.REUSE_HELPER_PATH] = sha(HEAD_BYTES[BUILD.REUSE_HELPER_PATH])
            calls = []

            def executor(argv, cwd, env, stdout, stderr):
                calls.append(argv)
                if argv[0] == "node" and Path(argv[1]).name == "prepare-m1-fixtures.mjs":
                    fixtures = Path(argv[2])
                    fixtures.mkdir(parents=True, exist_ok=True)
                    (fixtures / "sofle.boardstudio").write_bytes(b"unchanged Sofle fixture")
                    (fixtures / "vik.boardstudio").write_bytes(b"new VIK fixture")
                    (fixtures / "provenance.json").write_text('{"fixture":"vik"}\n')
                elif argv[0] == "dx":
                    public = root / "web/target/dx/boardstudio-web/release/web/public"
                    (public / "assets").mkdir(parents=True, exist_ok=True)
                    (public / "index.html").write_text("fresh fixture-refresh page")
                    (public / "assets/fresh-page.js").write_text("new page")
                elif argv[0] == "wasm-pack":
                    out = Path(argv[argv.index("--out-dir") + 1])
                    out.mkdir(parents=True, exist_ok=True)
                    (out / "boardstudio_offline_worker.js").write_text("fresh offline worker")
                elif argv[0] == "node" and Path(argv[1]).name == "embed-worker-wasm.mjs":
                    Path(argv[4]).write_text("fresh embedded worker")
                else:
                    raise AssertionError(argv)
                return subprocess.CompletedProcess(argv, 0)

            patches = self.mock_environment(root, base, current, HEAD_BYTES, helper_changed=True)
            patches.append(patch.object(BUILD.subprocess, "run", side_effect=executor))
            with self._patches(patches):
                BUILD.build_reuse("fixture-candidate", "full-fixture", refresh_fixtures=True)

            output = root / "web/target/builds/fixture-candidate"
            receipt = json.loads((output / "provenance.json").read_text())
            self.assertEqual(len(calls), 7)
            self.assertEqual(len(receipt["commands"]), 7)
            self.assertEqual([Path(command["log"]).stem for command in receipt["commands"]], [
                "fixtures", "page-root", "offline-worker-root", "embed-offline-root",
                "page-subpath", "offline-worker-subpath", "embed-offline-subpath",
            ])
            self.assertEqual(receipt["inherited_full_build_commands"], 22)
            self.assertEqual(len(receipt["inherited_full_build_lineage"]), 22)
            self.assertEqual(receipt["reuse_mode"], "fixture-refresh-provider-reuse")
            self.assertEqual(receipt["helper_compatibility"]["compatibility"], "pinned-bbd4-full-build-helper")
            self.assertEqual(set(receipt["changed_allowlisted_inputs"]), {
                BUILD.REUSE_HELPER_PATH, BUILD.FIXTURE_PREPARATION_PATH,
                "web/src/presentation/library.rs",
            })
            self.assertEqual(
                receipt["root"]["reused_provider_assets"],
                {name: data for name, data in base["root"]["assets"].items()
                 if name.startswith(BUILD.FIXTURE_REFRESH_INHERITED_PREFIXES)},
            )
            root_fixtures = receipt["root"]["refreshed_fixture_assets"]
            self.assertEqual(root_fixtures, receipt["subpath"]["refreshed_fixture_assets"])
            self.assertIn("assets/fixtures/vik.boardstudio", root_fixtures)
            self.assertNotIn("assets/fixtures/old.json", root_fixtures)
            self.assertEqual(
                {name: data for name, data in receipt["root"]["reused_provider_assets"].items()
                 if name.startswith(("assets/core-worker/", "assets/cad-worker/", "assets/cad/", "assets/renderer/"))},
                {name: data for name, data in base["root"]["assets"].items()
                 if name.startswith(("assets/core-worker/", "assets/cad-worker/", "assets/cad/", "assets/renderer/"))},
            )

            baseline_provenance_path = baseline / "provenance.json"
            baseline_provenance = json.loads(baseline_provenance_path.read_text())
            baseline_provenance["sources"][BUILD.REUSE_HELPER_PATH] = "f" * 64
            baseline_provenance_path.write_text(json.dumps(baseline_provenance))
            rejected_patches = self.mock_environment(root, base, current, HEAD_BYTES, helper_changed=True)
            with self._patches(rejected_patches):
                with self.assertRaisesRegex(ValueError, "neither unchanged nor the pinned compatible"):
                    BUILD.validate_reuse("rejected", "full-fixture", refresh_fixtures=True)

    def test_source_asset_and_provenance_drift_during_build_are_retained_as_failures(self):
        expected_errors = {
            "source": "Source changed during build",
            "asset": "Baseline changed during build",
            "provenance": "Baseline provenance changed during build",
            "provider-addition": "Reused provider path set or bytes changed",
        }
        for mutation, expected_error in expected_errors.items():
            with self.subTest(mutation=mutation), TemporaryDirectory() as temporary:
                root = Path(temporary)
                baseline, base = self.make_baseline(root)
                current = {name: sha(body) for name, body in SOURCE_BYTES.items()}
                current["web/src/presentation/panels.rs"] = sha(b"panels after")
                run_calls = []

                def executor(argv, cwd, env, stdout, stderr):
                    run_calls.append(argv)
                    if argv[0] == "node" and Path(argv[1]).name in ("build-layout-generators.mjs", "build-preview-generator.mjs"):
                        destination = Path(argv[2])
                        destination.mkdir(parents=True, exist_ok=True)
                        (destination / "fresh.js").write_text("fresh")
                    elif argv[0] == "dx":
                        public = root / "web/target/dx/boardstudio-web/release/web/public"
                        (public / "assets").mkdir(parents=True, exist_ok=True)
                        (public / "index.html").write_text("fresh page")
                        (public / "assets/boardstudio-web-current.js").write_text("fresh page js")
                        if mutation == "provider-addition":
                            unexpected = public / "assets/core-worker/additional.js"
                            unexpected.parent.mkdir(parents=True, exist_ok=True)
                            unexpected.write_text("unexpected provider")
                    elif argv[0] == "wasm-pack":
                        output = Path(argv[argv.index("--out-dir") + 1])
                        output.mkdir(parents=True, exist_ok=True)
                        (output / "boardstudio_offline_worker.js").write_text("fresh worker")
                    elif argv[0] == "node" and Path(argv[1]).name == "embed-worker-wasm.mjs":
                        Path(argv[4]).write_text("fresh service worker")
                        if "site-root" in argv[4]:
                            if mutation == "asset":
                                (Path(base["root"]["site"]) / "assets/provider.js").write_text("tampered during build")
                            elif mutation == "provenance":
                                (baseline / "provenance.json").write_text("{}")
                    else:
                        raise AssertionError(argv)
                    return subprocess.CompletedProcess(argv, 0)

                patches = self.mock_environment(root, base, current)
                if mutation == "source":
                    drift = {**current, "web/src/runtime.rs": sha(b"runtime changed during build")}
                    patches[3] = patch.object(BUILD, "sources", side_effect=[current, drift])
                patches.append(patch.object(BUILD.subprocess, "run", side_effect=executor))
                with self._patches(patches):
                    with self.assertRaisesRegex(SystemExit, expected_error):
                        BUILD.build_reuse("candidate", "full-fixture")
                receipt = json.loads((root / "web/target/builds/candidate/provenance.json").read_text())
                self.assertNotEqual(receipt["status"], "complete")
                self.assertEqual(len(run_calls), 5 if mutation == "provider-addition" else 8)
                self.assertTrue((baseline / "provenance.json").is_file())

    def _patches(self, patches):
        class PatchGroup:
            def __enter__(self):
                for item in patches:
                    item.__enter__()
                return self
            def __exit__(self, *exc):
                for item in reversed(patches):
                    item.__exit__(*exc)
        return PatchGroup()

    def test_provider_shared_and_config_changes_reject_before_output_creation(self):
        cases = (
            ("web/src/runtime.rs", b"changed runtime"),
            ("web/src/lib.rs", b"changed core-worker alias registration"),
            ("web/Cargo.lock", b"changed lock"),
            ("ergogen/generated/catalogue.mjs", b"changed consumed catalogue"),
            ("web/src/presentation/panels.rs", b"allowed panel"),
        )
        for path, body in cases:
            with self.subTest(path=path), TemporaryDirectory() as temporary:
                root = Path(temporary)
                baseline, _ = self.make_baseline(root)
                current = {name: sha(data) for name, data in SOURCE_BYTES.items()}
                current[path] = sha(body)
                if path == "web/src/presentation/panels.rs":
                    current["web/src/main.rs"] = sha(b"changed page graph")
                with self._patches(self.mock_environment(root, {}, current)):
                    with self.assertRaises(ValueError):
                        BUILD.validate_reuse("candidate", "full-fixture")
                self.assertFalse((root / "web/target/builds/candidate").exists())
                self.assertTrue((baseline / "provenance.json").exists())

    def test_source_addition_or_deletion_rejects(self):
        for mutation in ("added", "removed"):
            with self.subTest(mutation=mutation), TemporaryDirectory() as temporary:
                root = Path(temporary)
                self.make_baseline(root)
                current = {name: sha(data) for name, data in SOURCE_BYTES.items()}
                if mutation == "added":
                    current["web/src/presentation/new_shared.rs"] = sha(b"new module")
                else:
                    del current["web/src/runtime.rs"]
                with self._patches(self.mock_environment(root, {}, current)):
                    with self.assertRaisesRegex(ValueError, "outside page/test-only ownership|removals require a fresh full build"):
                        BUILD.validate_reuse("candidate", "full-fixture")
                self.assertFalse((root / "web/target/builds/candidate").exists())

    def test_source_inventory_includes_untracked_inputs_and_skips_only_declared_outputs(self):
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            names = (
                "web/src/presentation/new_shared.rs",
                "scripts/new-build-input.py",
                "ergogen/generated/catalogue.mjs",
                ".cargo/config.toml",
                ".npmrc",
                ".node-version",
                "web/target/generated.js",
                "cad/wasm/pkg/generated.js",
                "web/assets/layout-generators.js",
                "scripts/__pycache__/generated.pyc",
            )
            for name in names:
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(name.encode())
            def files(command, **kwargs):
                if "--ignored" in command:
                    self.assertEqual(command, ["git", "ls-files", "--others", "--ignored", "--exclude-standard", "-z"])
                    return ("\0".join(names) + "\0").encode()
                self.assertEqual(command, ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"])
                return ("\0".join(names) + "\0").encode()
            with patch.object(BUILD, "REPO", root), patch.object(BUILD.subprocess, "check_output", side_effect=files):
                result = BUILD.sources()
            self.assertIn("web/src/presentation/new_shared.rs", result)
            self.assertIn("scripts/new-build-input.py", result)
            self.assertIn("ergogen/generated/catalogue.mjs", result)
            self.assertIn(".cargo/config.toml", result)
            self.assertIn(".npmrc", result)
            self.assertIn(".node-version", result)
            excluded = [name for name in names if name.startswith(("web/target/", "cad/wasm/pkg/", "web/assets/layout-generators.js", "scripts/__pycache__/"))]
            self.assertTrue(all(name not in result for name in excluded))

    def test_actual_inventory_rejects_root_build_config_change_addition_and_deletion(self):
        for mutation in ("changed", "added", "removed"):
            with self.subTest(mutation=mutation), TemporaryDirectory() as temporary:
                root = Path(temporary)
                baseline, _ = self.make_baseline(root)
                names = set(SOURCE_BYTES)
                if mutation == "changed":
                    (root / ".cargo/config.toml").write_bytes(b"[build]\nrustflags = ['-C', 'opt-level=3']\n")
                elif mutation == "added":
                    path = root / ".cargo/config"
                    path.parent.mkdir(parents=True, exist_ok=True)
                    path.write_bytes(b"[build]\nrustflags = ['-C', 'debuginfo=0']\n")
                    names.add(".cargo/config")
                else:
                    (root / ".npmrc").unlink(missing_ok=True)
                    names.remove(".npmrc")

                current_names = sorted(names)
                def check_output(command, **kwargs):
                    if command[:3] == ["git", "cat-file", "-e"]:
                        return b""
                    if command[:2] == ["git", "ls-files"]:
                        return ("\0".join(current_names) + "\0").encode()
                    raise AssertionError(f"unexpected command in actual inventory fixture: {command}")

                patches = [
                    patch.object(BUILD, "current_tools", return_value=TOOLS),
                    patch.object(BUILD.subprocess, "check_output", side_effect=check_output),
                ]
                with patch.object(BUILD, "REPO", root), patch.object(BUILD, "WEB", root / "web"), \
                     patch.object(BUILD, "BUILD_ROOT", root / "web/target/builds"), self._patches(patches):
                    with self.assertRaises(ValueError):
                        BUILD.validate_reuse("candidate", "full-fixture")
                self.assertFalse((root / "web/target/builds/candidate").exists())
                self.assertTrue((baseline / "provenance.json").exists())

    def test_tampered_or_extra_baseline_asset_rejects(self):
        for mutation in ("tampered", "extra"):
            with self.subTest(mutation=mutation), TemporaryDirectory() as temporary:
                root = Path(temporary)
                baseline, provenance = self.make_baseline(root)
                site = Path(provenance["root"]["site"])
                if mutation == "tampered":
                    (site / "assets/provider.js").write_text("tampered")
                else:
                    (site / "assets/extra.js").write_text("unexpected")
                with self._patches(self.mock_environment(root, provenance)):
                    with self.assertRaises(ValueError):
                        BUILD.validate_reuse("candidate", "full-fixture")
                self.assertFalse((root / "web/target/builds/candidate").exists())

    def test_incomplete_reuse_derived_tool_mismatched_and_command_malformed_baselines_reject(self):
        for mutation in ("reuse-derived", "tool-mismatch", "command-feature"):
            with self.subTest(mutation=mutation), TemporaryDirectory() as temporary:
                root = Path(temporary)
                baseline, provenance = self.make_baseline(root)
                if mutation == "reuse-derived":
                    provenance["reuse_mode"] = "page-only-provider-reuse"
                elif mutation == "command-feature":
                    next(row for row in provenance["commands"] if Path(row["log"]).name == "page-root.log")["argv"][9] = "core-worker"
                (baseline / "provenance.json").write_text(json.dumps(provenance))
                patches = self.mock_environment(root, provenance)
                if mutation == "tool-mismatch":
                    patches[4] = patch.object(BUILD, "current_tools", return_value={**TOOLS, "dx-version": "dx different"})
                with self._patches(patches):
                    with self.assertRaises(ValueError):
                        BUILD.validate_reuse("candidate", "full-fixture")
                self.assertFalse((root / "web/target/builds/candidate").exists())

    def test_current_case_firmware_shared_drift_is_rejected(self):
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.make_baseline(root)
            current = {name: sha(data) for name, data in SOURCE_BYTES.items()}
            current["web/src/cad_jobs.rs"] = sha(b"case/firmware provider wave")
            current["web/src/presentation/panels.rs"] = sha(b"panel also changed")
            head = dict(HEAD_BYTES)
            head["web/src/cad_jobs.rs"] = b"case/firmware provider wave"
            head["web/src/presentation/panels.rs"] = b"panel also changed"
            with self._patches(self.mock_environment(root, {}, current, head)):
                with self.assertRaisesRegex(ValueError, "outside page/test-only ownership"):
                    BUILD.validate_reuse("candidate", "full-fixture")
            self.assertFalse((root / "web/target/builds/candidate").exists())


if __name__ == "__main__":
    main()
