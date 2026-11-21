#!/usr/bin/env python3
"""Verify pure-Rust CAD build stamp invalidation, cache reuse, and failure propagation."""
import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import tomllib
import unittest
from unittest.mock import MagicMock, patch

import cad_build as build

_spec = importlib.util.spec_from_file_location("cad_browser", Path(__file__).with_name("test-cad-browser.py"))
browser = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(browser)

REPO_ROOT = build.CAD_ROOT.parent
CAD_CRATES = ("core", "wasm")


class CadBuildTests(unittest.TestCase):
    PASSED = subprocess.CompletedProcess([], 0, stdout=(
        "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 99 filtered out;\n"))

    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.project = Path(self.directory.name)
        self.root = self.project / "cad"
        self.root.mkdir()
        self.patch = patch.multiple(build, CAD_ROOT=self.root)
        self.patch.start()
        self.addCleanup(self.patch.stop)
        self.enterContext(patch.dict(os.environ, {"BOARDSTUDIO_CHECK_JOBS": "1"}))

    def wasm_sources(self):
        (self.root / "wasm/src/model").mkdir(parents=True)
        (self.root / "wasm/src/lib.rs").write_text("// provider")
        (self.root / "wasm/src/model/source_validation_tests.rs").write_text("// test only")
        (self.root / "wasm/Cargo.toml").write_text('[package]\nname = "boardstudio-cad-wasm"\nversion = "0.1.0"\n')
        (self.root / "core/src").mkdir(parents=True)
        (self.root / "core/src/lib.rs").write_text("// library")
        (self.root / "core/src/tests.rs").write_text("// library test only")
        (self.root / "Cargo.toml").write_text('[workspace]\nresolver = "3"\nmembers = ["core", "wasm"]\n')
        (self.root / "Cargo.lock").write_text("version = 4\n")
        (self.root / "core/Cargo.toml").write_text('[package]\nname = "boardstudio-cad"\nversion = "0.1.0"\n')
        for crate in build.VENDOR_CRATES:
            vendor_dir = self.root / "vendor" / crate / "src"
            vendor_dir.mkdir(parents=True)
            (vendor_dir / "lib.rs").write_text(f"// {crate}")
            (vendor_dir.parent / "Cargo.toml").write_text(f'[package]\nname = "{crate}"\nversion = "0.4.1"\n')
        (self.project / "Cargo.toml").write_text('[workspace]\nresolver = "3"\nmembers = ["contracts/rust"]\n')
        contracts = self.project / "contracts/rust"
        (contracts / "src").mkdir(parents=True)
        (contracts / "src/lib.rs").write_text("// contracts")
        (contracts / "Cargo.toml").write_text('[package]\nname = "boardstudio-contracts"\nversion = "0.1.0"\n')

    def test_no_opt_is_parsed_by_wasm_pack_before_cargo_options(self):
        self.wasm_sources()
        with patch.object(build, "run") as run:
            build.build_wasm(no_opt=True)
        command = run.call_args.args[0]
        self.assertEqual(command[:2], ["wasm-pack", "build"])
        self.assertIn("boardstudio_cad_wasm", command)
        self.assertIn("--release", command)
        self.assertLess(command.index("--no-opt"), command.index("--locked"))
        self.assertEqual(run.call_args.kwargs["env"]["CARGO_TARGET_DIR"], str(self.root / "wasm/target"))

    def test_cgmath_source_changes_invalidate_wasm_cache(self):
        self.wasm_sources()
        source = self.project / "renderer/vendor/cgmath-next-0.18.1/src/lib.rs"
        source.parent.mkdir(parents=True)
        source.write_text("// reviewed fork")
        previous = build.wasm_inputs_digest()
        source.write_text("// updated fork")
        self.assertNotEqual(build.wasm_inputs_digest(), previous)

    def test_workspace_library_vendor_and_build_script_changes_invalidate_wasm_cache(self):
        self.wasm_sources()
        script = self.project / "cad_build.py"
        script.write_text("# build script")
        with patch.object(build, "__file__", str(script)):
            for path in (
                self.project / "Cargo.toml",
                script,
                self.root / "Cargo.toml",
                self.root / "Cargo.lock",
                self.root / "wasm/src/lib.rs",
                self.root / "core/src/lib.rs",
                self.root / "vendor/cadmpeg-tessellate/src/lib.rs",
                self.project / "contracts/rust/src/lib.rs",
            ):
                with self.subTest(path=path):
                    previous = build.wasm_inputs_digest()
                    path.write_text(path.read_text() + ("\n# changed" if path.suffix != ".rs" else "// changed"))
                    self.assertNotEqual(build.wasm_inputs_digest(), previous)

            for test_only in (
                self.root / "wasm/src/model/source_validation_tests.rs",
                self.root / "core/src/tests.rs",
            ):
                with self.subTest(test_only=test_only):
                    previous = build.wasm_inputs_digest()
                    test_only.write_text(test_only.read_text() + "\n// test change")
                    self.assertEqual(build.wasm_inputs_digest(), previous)

    def test_unchanged_provider_inputs_skip_rebuild_and_remove_stale_cadrum_files(self):
        self.wasm_sources()
        pkg = self.root / "wasm/pkg"
        pkg.mkdir(parents=True)
        (pkg / "package.json").write_text("generated")
        (pkg / "boardstudio_cadrum_wasm.js").write_text("stale")
        output = pkg / "boardstudio_cad_wasm_bg.wasm"
        with patch.object(build, "run", side_effect=lambda *a, **k: output.write_bytes(b"wasm")) as run:
            build.build_wasm()
        run.assert_called_once()
        self.assertFalse((pkg / "package.json").exists())
        self.assertFalse((pkg / "boardstudio_cadrum_wasm.js").exists())
        with patch.object(build, "run") as run, contextlib.redirect_stdout(io.StringIO()) as printed:
            build.build_wasm()
        run.assert_not_called()
        self.assertIn("skipping the WASM build", printed.getvalue())

    def production_targets(self):
        return [
            ("boardstudio-cad", "lib", "boardstudio_cad", self.root / "core"),
            ("boardstudio-cad", "test", "step_import", self.root / "core"),
            ("boardstudio-cad", "example", "step_replay", self.root / "core"),
            ("boardstudio-cad-wasm", "lib", "boardstudio_cad_wasm", self.root / "wasm"),
        ]

    def production_prebuild(self, *, omit=None, duplicate=None, status=0):
        artifacts = []
        keys = {(pkg, kind, name) for pkg, kind, name, _ in self.production_targets()}
        for package, kind, name in sorted(keys):
            key = (package, kind, name)
            if key == omit:
                continue
            artifact_kind = ["rlib"] if (package == "boardstudio-cad" and kind == "lib") else (
                ["cdylib", "rlib"] if (package == "boardstudio-cad-wasm" and kind == "lib") else [kind]
            )
            artifact = json.dumps({"reason": "compiler-artifact", "package_id": f"path+file:///cad/{package}#{package}@0.4.1",
                                   "target": {"kind": artifact_kind, "name": name},
                                   "profile": {"test": True},
                                   "executable": f"/built/{package}/{name}"}) + "\n"
            artifacts.append(artifact)
            if key == duplicate:
                artifacts.append(artifact)
        process = MagicMock()
        process.stdout = iter(['{"reason":"compiler-message","message":{"rendered":"diagnostic\\n"}}\n'] + artifacts)
        process.wait.return_value = status
        context = MagicMock()
        context.__enter__.return_value = process
        return context

    def prebuilds(self, **kwargs):
        # The production (cad-test) prebuild runs as a single cargo invocation.
        return [self.production_prebuild(**kwargs)]

    def test_native_tests_run_library_and_provider_and_propagate_failures(self):
        prod = self.production_targets()
        with patch.object(build, "_production_targets", return_value=prod), \
                patch.object(build.subprocess, "Popen", side_effect=self.prebuilds()) as prebuild, \
                patch.object(build, "run", return_value=self.PASSED) as run, \
                contextlib.redirect_stdout(io.StringIO()):
            build.test_cad()
        self.assertEqual([call.args[0] for call in prebuild.call_args_list], [
            [
                "cargo", "test", "--no-run", "--manifest-path", "Cargo.toml",
                "-p", "boardstudio-cad", "-p", "boardstudio-cad-wasm",
                "--lib",
                "--test", "step_import",
                "--example", "step_replay",
                "--profile", "cad-test", "--locked", "--message-format=json",
            ],
        ])
        for call in prebuild.call_args_list:
            self.assertEqual(call.kwargs["env"]["CARGO_TARGET_DIR"], str(self.root / "target"))
        self.assertEqual(len(run.call_args_list), len(prod))
        for call, (pkg, kind, name, pkg_dir) in zip(run.call_args_list, prod):
            self.assertEqual(call.args[0], [f"/built/{pkg}/{name}"])
            self.assertEqual(call.kwargs["cwd"], pkg_dir)
            self.assertEqual(call.kwargs["env"]["CARGO_MANIFEST_DIR"], str(pkg_dir))
        with patch.object(build, "_production_targets", return_value=prod), \
                patch.object(build.subprocess, "Popen", side_effect=self.prebuilds()), \
                patch.object(build, "run", side_effect=subprocess.CalledProcessError(1, "cargo")), \
                contextlib.redirect_stdout(io.StringIO()):
            with self.assertRaises(subprocess.CalledProcessError):
                build.test_cad()

    def test_prebuild_failure_missing_and_duplicate_binaries(self):
        key = ("boardstudio-cad", "test", "step_import")
        for kwargs, error in (({"status": 7}, subprocess.CalledProcessError),
                              ({"omit": key}, RuntimeError), ({"duplicate": key}, RuntimeError)):
            with self.subTest(kwargs=kwargs), \
                    patch.object(build, "_production_targets", return_value=self.production_targets()), \
                    patch.object(build.subprocess, "Popen", side_effect=self.prebuilds(**kwargs)), \
                    patch.object(build, "run", return_value=self.PASSED) as run, contextlib.redirect_stdout(io.StringIO()) as output:
                with self.assertRaises(error):
                    build.test_cad()
                self.assertEqual(run.call_count, 0)
                self.assertIn("diagnostic", output.getvalue())

    def test_each_production_failure_propagates_without_running_later_commands(self):
        prod = self.production_targets()
        for index in range(len(prod)):
            with self.subTest(index=index), \
                    patch.object(build, "_production_targets", return_value=prod), \
                    patch.object(build.subprocess, "Popen", side_effect=self.prebuilds()), \
                    patch.object(build, "run", side_effect=[*([self.PASSED] * index),
                                                           subprocess.CalledProcessError(7, "binary", output="failure\n")]) as run, \
                    contextlib.redirect_stdout(io.StringIO()) as output:
                with self.assertRaises(subprocess.CalledProcessError):
                    build.test_cad()
                self.assertEqual(run.call_count, index + 1)
                self.assertIn("failure", output.getvalue())

    def test_production_runs_retain_success_and_failure_output(self):
        prod = self.production_targets()
        with patch.object(build, "_production_targets", return_value=prod), \
                patch.object(build.subprocess, "Popen", side_effect=self.prebuilds()), \
                patch.object(build.subprocess, "run", return_value=self.PASSED) as run, contextlib.redirect_stdout(io.StringIO()) as printed:
            build.test_cad()
        self.assertEqual(printed.getvalue(), "diagnostic\n" + self.PASSED.stdout * len(prod))
        for call in run.call_args_list:
            self.assertEqual(call.kwargs["stdout"], subprocess.PIPE)
            self.assertEqual(call.kwargs["stderr"], subprocess.STDOUT)
            self.assertTrue(call.kwargs["text"])
            self.assertTrue(call.kwargs["check"])
        failure = subprocess.CalledProcessError(7, "binary", output="first production failure\n")
        with patch.object(build, "_production_targets", return_value=prod), \
                patch.object(build.subprocess, "Popen", side_effect=self.prebuilds()), \
                patch.object(build, "run", side_effect=[self.PASSED, failure]) as run, \
                contextlib.redirect_stdout(io.StringIO()) as printed:
            with self.assertRaises(subprocess.CalledProcessError) as raised:
                build.test_cad()
        self.assertIs(raised.exception, failure)
        self.assertEqual(run.call_count, 2)
        self.assertEqual(printed.getvalue(), "diagnostic\n" + self.PASSED.stdout + failure.stdout)

    def test_production_failure_propagates_immediately(self):
        prod = self.production_targets()
        failure = subprocess.CalledProcessError(7, "binary", output="production failure output\n")
        with patch.object(build, "_production_targets", return_value=prod), \
                patch.object(build.subprocess, "Popen", side_effect=self.prebuilds()), \
                patch.object(build, "run", side_effect=failure) as run, \
                contextlib.redirect_stdout(io.StringIO()) as stdout:
            with self.assertRaises(subprocess.CalledProcessError) as raised:
                build.test_cad()
        self.assertIs(raised.exception, failure)
        self.assertEqual(run.call_count, 1)
        self.assertEqual(stdout.getvalue(), "diagnostic\n" + "production failure output\n")

    def test_provider_tests_are_registered_or_accounted_for(self):
        scripts = {path.name for path in (REPO_ROOT / "cad/wasm/tests").glob("*.cjs")}
        opt_in = {
            "layered-adoption.cjs": "separate layered-adoption fixture corpus",
            "layered-adoption-reachability.cjs": "Core adoption disposition replay prerequisite",
            "keycap-print-legends.cjs": "separate printable keycap export regression",
            "step-native-analytic.cjs": "standalone packaged-provider STEP refinement and face identity regression",
        }
        mapped = {
            "step-face-completeness.cjs": "source for the required browser regression",
        }
        self.assertEqual(scripts, opt_in.keys() | mapped.keys() | {"load-provider.cjs"})
        self.assertTrue(all(opt_in.values()) and all(mapped.values()))


class CadWorkspaceTests(unittest.TestCase):
    """Cargo must find the merged CAD root even in a nested checkout."""

    def test_cad_crates_declare_the_merged_workspace(self):
        root = tomllib.loads((build.CAD_ROOT / "Cargo.toml").read_text())
        self.assertEqual(root["workspace"]["resolver"], "3")
        self.assertEqual(root["workspace"]["members"], [
            "core",
            "wasm",
        ])
        for crate in CAD_CRATES:
            manifest = tomllib.loads((build.CAD_ROOT / crate / "Cargo.toml").read_text())
            self.assertNotIn("workspace", manifest)
            self.assertNotIn("patch", manifest)
            self.assertNotIn("profile", manifest)

    def test_nested_checkout_resolves_cad_crates_without_the_parent_workspace(self):
        exclude = tomllib.loads((REPO_ROOT / "Cargo.toml").read_text())["workspace"]["exclude"]
        with tempfile.TemporaryDirectory() as directory:
            outer = Path(directory)
            (outer / "app/src").mkdir(parents=True)
            (outer / "Cargo.toml").write_text('[workspace]\nresolver = "3"\nmembers = ["app"]\n')
            (outer / "app/Cargo.toml").write_text('[package]\nname = "app"\nversion = "0.1.0"\nedition = "2021"\n')
            (outer / "app/src/lib.rs").write_text("")
            nested = outer / ".worktrees/nested"
            (nested / "contracts/rust/src").mkdir(parents=True)
            (nested / "contracts/rust/src/lib.rs").write_text("")
            (nested / "contracts/rust/Cargo.toml").write_text(
                '[package]\nname = "boardstudio-contracts"\nversion = "0.1.0"\nedition = "2021"\n')
            (nested / "Cargo.toml").write_text('[workspace]\nresolver = "3"\nmembers = ["contracts/rust"]\n'
                                               f"exclude = {json.dumps(exclude)}\n")
            (nested / "cad").mkdir(parents=True)
            shutil.copy(build.CAD_ROOT / "Cargo.toml", nested / "cad/Cargo.toml")
            for vendor_crate in build.VENDOR_CRATES:
                src_vendor = build.CAD_ROOT / "vendor" / vendor_crate
                dst_vendor = nested / "cad/vendor" / vendor_crate
                (dst_vendor / "src").mkdir(parents=True)
                (dst_vendor / "src/lib.rs").write_text("")
                shutil.copy(src_vendor / "Cargo.toml", dst_vendor / "Cargo.toml")
            for crate in CAD_CRATES:
                (nested / "cad" / crate / "src").mkdir(parents=True)
                (nested / "cad" / crate / "src/lib.rs").write_text("")
                shutil.copy(build.CAD_ROOT / crate / "Cargo.toml", nested / "cad" / crate / "Cargo.toml")
            expected_packages = {
                "boardstudio-cad",
                "boardstudio-cad-wasm",
            }
            for crate in CAD_CRATES:
                with self.subTest(crate=crate):
                    manifest = nested / "cad" / crate / "Cargo.toml"
                    done = subprocess.run(["cargo", "metadata", "--no-deps", "--format-version", "1",
                                           "--manifest-path", str(manifest)], capture_output=True, text=True)
                    self.assertEqual(done.returncode, 0, done.stderr)
                    metadata = json.loads(done.stdout)
                    self.assertEqual(metadata["workspace_root"], str(nested / "cad"))
                    self.assertEqual({package["name"] for package in metadata["packages"]}, expected_packages)


class BrowserSmokeResultTests(unittest.TestCase):
    def test_result_reads_text_instead_of_html_entities(self):
        with patch.object(browser.browser_tools, "WebDriverSession") as session:
            page = session.return_value.__enter__.return_value
            page.execute_async.return_value = '{"error":null,"checks":[{"name":"a & b","ok":true,"detail":"1 < 2"}]}'
            result = browser.run_page("http://127.0.0.1:8000/")
        self.assertEqual(result["checks"][0], {"name": "a & b", "ok": True, "detail": "1 < 2"})
        page.check_page_errors.assert_called_once()

    def test_missing_result_is_an_error(self):
        with patch.object(browser.browser_tools, "WebDriverSession") as session:
            session.return_value.__enter__.return_value.execute_async.return_value = None
            with self.assertRaisesRegex(RuntimeError, "did not render"):
                browser.run_page("http://127.0.0.1:8000/")

    def test_a_failed_check_makes_the_run_fail(self):
        failing = {"error": None, "checks": [{"name": "x", "ok": False, "detail": "d"}]}
        with patch.object(browser.browser_tools, "serve"), patch.object(browser, "run_page", return_value=failing), \
                patch.object(browser.Path, "is_file", return_value=True), contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(browser.main(), 1)


if __name__ == "__main__":
    unittest.main()
