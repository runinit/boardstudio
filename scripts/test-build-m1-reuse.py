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
SOURCE_BYTES = {
    **PROOF,
    "web/src/presentation/panels.rs": b"panels before",
    "web/src/presentation/panels_scroll_tests.rs": b"scroll tests before",
    "web/assets/m1.css": b"css before",
    "web/src/runtime.rs": b"shared runtime",
    "web/Cargo.lock": b"locked dependencies",
    "ergogen/generated/catalogue.mjs": b"consumed generated catalogue",
    ".cargo/config.toml": b"[build]\nrustflags = ['-C', 'target-cpu=native']\n",
    ".npmrc": b"strict-peer-dependencies=true\n",
    ".node-version": b"22.0.0\n",
}
HEAD_BYTES = {**SOURCE_BYTES,
              "web/src/presentation/panels.rs": b"panels after",
              "web/src/presentation/panels_scroll_tests.rs": b"scroll tests after",
              "web/assets/m1.css": b"css after"}


def sha(data):
    return hashlib.sha256(data).hexdigest()


class PageOnlyReuseTests(TestCase):
    def make_baseline(self, root, *, extra_asset=None):
        build_root = root / "web/target/builds"
        baseline = build_root / "full-fixture"
        baseline.mkdir(parents=True)
        sources = {name: sha(body) for name, body in SOURCE_BYTES.items()}
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
            "source_commit": "a" * 40,
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

    def mock_environment(self, root, provenance, current=None):
        current = current or {name: sha(body) for name, body in SOURCE_BYTES.items()}
        def check_output(command, **kwargs):
            if command[:2] == ["git", "rev-parse"]:
                return "b" * 40 if kwargs.get("text") else b"b" * 40
            if command[:3] == ["git", "cat-file", "-e"]:
                return b""
            if command[:2] == ["git", "ls-files"]:
                if "--ignored" in command:
                    return b""
                return ("\0".join(current) + "\0").encode()
            if command[0] == "git" and command[1] == "show":
                revision, path = command[2].split(":", 1)
                return (HEAD_BYTES if revision == "HEAD" else SOURCE_BYTES)[path]
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
            with self.subTest("permitted delta"), self._patches(self.mock_environment(root, {}, current)):
                checked = BUILD.validate_reuse("candidate", "full-fixture")
                self.assertEqual(checked[6], ["web/assets/m1.css", "web/src/presentation/panels.rs", "web/src/presentation/panels_scroll_tests.rs"])
                self.assertFalse(checked[0].exists())

    def test_docs_only_source_identity_change_can_reuse_explicitly(self):
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.make_baseline(root)
            with self._patches(self.mock_environment(root, {})):
                checked = BUILD.validate_reuse("candidate", "full-fixture")
                self.assertEqual(checked[6], [])
                self.assertFalse(checked[0].exists())

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
                    with self.assertRaisesRegex(ValueError, "source path set"):
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
            current["web/src/runtime.rs"] = sha(b"case/fimware runtime wave")
            current["web/src/presentation/panels.rs"] = sha(b"panel also changed")
            with self._patches(self.mock_environment(root, {}, current)):
                with self.assertRaisesRegex(ValueError, "outside the page-only allowlist"):
                    BUILD.validate_reuse("candidate", "full-fixture")
            self.assertFalse((root / "web/target/builds/candidate").exists())


if __name__ == "__main__":
    main()
