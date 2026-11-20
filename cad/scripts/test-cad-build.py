#!/usr/bin/env python3
"""Verify pure-Rust CAD build stamp invalidation, cache reuse, and failure propagation."""
import contextlib
import http.server
import importlib.util
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import threading
import time
import tomllib
import unittest
import urllib.request
from unittest.mock import MagicMock, patch

import cad_build as build

_spec = importlib.util.spec_from_file_location("cad_browser", Path(__file__).with_name("test-cad-browser.py"))
browser = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(browser)

REPO_ROOT = build.CAD_ROOT.parent
CAD_CRATES = ("monstertruck", "wasm")


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
        (self.root / "monstertruck/src").mkdir(parents=True)
        (self.root / "monstertruck/src/lib.rs").write_text("// library")
        (self.root / "monstertruck/src/tests.rs").write_text("// library test only")
        (self.root / "Cargo.toml").write_text('[workspace]\nresolver = "3"\nmembers = ["monstertruck", "wasm"]\n')
        (self.root / "Cargo.lock").write_text("version = 4\n")
        (self.root / "monstertruck/Cargo.toml").write_text('[package]\nname = "boardstudio-cad"\nversion = "0.1.0"\n')
        for crate in build.VENDOR_CRATES:
            vendor_dir = self.root / "monstertruck-qualification/vendor" / crate / "src"
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
                self.root / "monstertruck/src/lib.rs",
                self.root / "monstertruck-qualification/vendor/monstertruck-core/src/lib.rs",
                self.project / "contracts/rust/src/lib.rs",
            ):
                with self.subTest(path=path):
                    previous = build.wasm_inputs_digest()
                    path.write_text(path.read_text() + ("\n# changed" if path.suffix != ".rs" else "// changed"))
                    self.assertNotEqual(build.wasm_inputs_digest(), previous)

            for test_only in (
                self.root / "wasm/src/model/source_validation_tests.rs",
                self.root / "monstertruck/src/tests.rs",
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
            ("boardstudio-cad", "lib", "boardstudio_cad", self.root / "monstertruck"),
            ("boardstudio-cad", "test", "step_import", self.root / "monstertruck"),
            ("boardstudio-cad", "example", "step_replay", self.root / "monstertruck"),
            ("boardstudio-cad-wasm", "lib", "boardstudio_cad_wasm", self.root / "wasm"),
        ]

    def owner_prebuild(self, *, omit=None, duplicate=None, status=0):
        artifacts = []
        keys = {
            *((pkg, kind, name) for pkg, kind, name, _ in self.production_targets()),
            *(build._owner_binary_key(package, selection) for package, *selection in build.OWNER_TESTS),
        }
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

    def test_native_tests_run_library_and_provider_and_propagate_failures(self):
        prod = self.production_targets()
        with patch.object(build, "_production_targets", return_value=prod), \
                patch.object(build.subprocess, "Popen", return_value=self.owner_prebuild()) as prebuild, \
                patch.object(build, "run", return_value=self.PASSED) as run, \
                contextlib.redirect_stdout(io.StringIO()):
            build.test_cad()
        self.assertEqual(build.OWNER_TESTS, (
            ("monstertruck-io", "--test", "input", "repair_movement"),
            ("monstertruck-io", "--lib", "repair_movement_sink"),
            ("monstertruck-io", "--test", "input", "source_pcurve_active_view"),
            ("monstertruck-io", "--test", "input", "preview_tolerance"),
            ("monstertruck-meshing", "--lib", "repair_"),
            ("monstertruck-meshing", "--test", "closed_trim_projection"),
            ("monstertruck-meshing", "--lib", "seam_branch_tests"),
        ))
        command = prebuild.call_args.args[0]
        self.assertEqual(command, [
            "cargo", "test", "--no-run", "--manifest-path", "Cargo.toml",
            "-p", "boardstudio-cad", "-p", "boardstudio-cad-wasm",
            "-p", "monstertruck-io", "-p", "monstertruck-meshing",
            "--lib",
            "--test", "closed_trim_projection", "--test", "input", "--test", "step_import",
            "--example", "step_replay",
            "--profile", "cad-test", "--locked", "--message-format=json",
        ])
        self.assertEqual(prebuild.call_args.kwargs["env"]["CARGO_TARGET_DIR"], str(self.root / "target"))
        self.assertEqual(len(run.call_args_list), len(prod) + len(build.OWNER_TESTS))
        for call, (pkg, kind, name, pkg_dir) in zip(run.call_args_list[:len(prod)], prod):
            self.assertEqual(call.args[0], [f"/built/{pkg}/{name}"])
            self.assertEqual(call.kwargs["cwd"], pkg_dir)
            self.assertEqual(call.kwargs["env"]["CARGO_MANIFEST_DIR"], str(pkg_dir))
        for call, (package, *selection) in zip(run.call_args_list[len(prod):], build.OWNER_TESTS):
            key = build._owner_binary_key(package, selection)
            args = selection[1:] if selection[0] == "--lib" else selection[2:]
            self.assertEqual(call.args[0], [f"/built/{package}/{key[2]}", *args])
            cwd = self.root / "monstertruck-qualification/vendor" / package
            self.assertEqual(call.kwargs["cwd"], cwd)
            self.assertEqual(call.kwargs["env"]["CARGO_MANIFEST_DIR"], str(cwd))
        with patch.object(build, "_production_targets", return_value=prod), \
                patch.object(build.subprocess, "Popen", return_value=self.owner_prebuild()), \
                patch.object(build, "run", side_effect=subprocess.CalledProcessError(1, "cargo")), \
                contextlib.redirect_stdout(io.StringIO()):
            with self.assertRaises(subprocess.CalledProcessError):
                build.test_cad()

    def test_prebuild_failure_missing_and_duplicate_binaries(self):
        key = build._owner_binary_key(build.OWNER_TESTS[0][0], build.OWNER_TESTS[0][1:])
        for kwargs, error in (({"status": 7}, subprocess.CalledProcessError),
                              ({"omit": key}, RuntimeError), ({"duplicate": key}, RuntimeError)):
            with self.subTest(kwargs=kwargs), \
                    patch.object(build, "_production_targets", return_value=self.production_targets()), \
                    patch.object(build.subprocess, "Popen", return_value=self.owner_prebuild(**kwargs)), \
                    patch.object(build, "run", return_value=self.PASSED) as run, contextlib.redirect_stdout(io.StringIO()) as output:
                with self.assertRaises(error):
                    build.test_cad()
                self.assertEqual(run.call_count, 0)
                self.assertIn("diagnostic", output.getvalue())

    def test_each_owner_failure_propagates_without_running_later_commands(self):
        prod = self.production_targets()
        for index in range(len(build.OWNER_TESTS)):
            with self.subTest(index=index), \
                    patch.object(build, "_production_targets", return_value=prod), \
                    patch.object(build.subprocess, "Popen", return_value=self.owner_prebuild()), \
                    patch.object(build, "run", side_effect=[*([self.PASSED] * (len(prod) + index)),
                                                           subprocess.CalledProcessError(7, "binary", output="failure\n")]) as run, \
                    contextlib.redirect_stdout(io.StringIO()) as output:
                with self.assertRaises(subprocess.CalledProcessError):
                    build.test_cad()
                self.assertEqual(run.call_count, len(prod) + index + 1)
                self.assertIn("failure", output.getvalue())

    def test_each_required_owner_selection_rejects_zero_or_all_ignored_tests(self):
        prod = self.production_targets()
        empty = ("test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 100 filtered out;\n",
                 "test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 99 filtered out;\n", "running 1 test\n")
        for index, (package, *selection) in enumerate(build.OWNER_TESTS):
            for output in empty:
                results = [subprocess.CompletedProcess([], 0, stdout=output if step == len(prod) + index else self.PASSED.stdout)
                           for step in range(len(prod) + len(build.OWNER_TESTS))]
                with self.subTest(package=package, selection=selection, output=output), \
                        patch.object(build, "_production_targets", return_value=prod), \
                        patch.object(build.subprocess, "Popen", return_value=self.owner_prebuild()), \
                        patch.object(build, "run", side_effect=results) as run, contextlib.redirect_stdout(io.StringIO()):
                    with self.assertRaisesRegex(RuntimeError, "no executed tests"):
                        build.test_cad()
                    self.assertEqual(run.call_count, len(prod) + index + 1)

    def test_required_owner_runs_retain_success_and_failure_output(self):
        prod = self.production_targets()
        total = len(prod) + len(build.OWNER_TESTS)
        with patch.object(build, "_production_targets", return_value=prod), \
                patch.object(build.subprocess, "Popen", return_value=self.owner_prebuild()), \
                patch.object(build.subprocess, "run", return_value=self.PASSED) as run, contextlib.redirect_stdout(io.StringIO()) as printed:
            build.test_cad()
        self.assertEqual(printed.getvalue(), "diagnostic\n" + self.PASSED.stdout * total)
        for call in run.call_args_list:
            self.assertEqual(call.kwargs["stdout"], subprocess.PIPE)
            self.assertEqual(call.kwargs["stderr"], subprocess.STDOUT)
            self.assertTrue(call.kwargs["text"])
            self.assertTrue(call.kwargs["check"])
        failure = subprocess.CalledProcessError(7, "binary", output="first owner failure\n")
        with patch.object(build, "_production_targets", return_value=prod), \
                patch.object(build.subprocess, "Popen", return_value=self.owner_prebuild()), \
                patch.object(build, "run", side_effect=[*([self.PASSED] * len(prod)), failure]) as run, \
                contextlib.redirect_stdout(io.StringIO()) as printed:
            with self.assertRaises(subprocess.CalledProcessError) as raised:
                build.test_cad()
        self.assertIs(raised.exception, failure)
        self.assertEqual(run.call_count, len(prod) + 1)
        self.assertEqual(printed.getvalue(), "diagnostic\n" + self.PASSED.stdout * len(prod) + failure.stdout)

    def test_production_failure_stops_before_owner_runs(self):
        prod = self.production_targets()
        failure = subprocess.CalledProcessError(7, "binary", output="production failure output\n")
        with patch.object(build, "_production_targets", return_value=prod), \
                patch.object(build.subprocess, "Popen", return_value=self.owner_prebuild()), \
                patch.object(build, "run", side_effect=failure) as run, \
                contextlib.redirect_stdout(io.StringIO()) as stdout:
            with self.assertRaises(subprocess.CalledProcessError) as raised:
                build.test_cad()
        self.assertIs(raised.exception, failure)
        self.assertEqual(run.call_count, 1)
        self.assertEqual(stdout.getvalue(), "diagnostic\nproduction failure output\n")

    def test_provider_tests_are_registered_or_accounted_for(self):
        scripts = {path.name for path in (REPO_ROOT / "cad/wasm/tests").glob("*.cjs")}
        opt_in = {
            "layered-adoption.cjs": "separate layered-adoption fixture corpus",
            "layered-adoption-reachability.cjs": "Core adoption disposition replay prerequisite",
            "keycap-print-legends.cjs": "separate printable keycap export regression",
        }
        mapped = {"step-face-completeness.cjs": "source for the required browser regression"}
        self.assertEqual(scripts, opt_in.keys() | mapped.keys() | {"load-provider.cjs"})
        self.assertTrue(all(opt_in.values()) and all(mapped.values()))


class CadWorkspaceTests(unittest.TestCase):
    """Cargo must find the merged CAD root even in a nested checkout."""

    def test_cad_crates_declare_the_merged_workspace(self):
        root = tomllib.loads((build.CAD_ROOT / "Cargo.toml").read_text())
        self.assertEqual(root["workspace"]["resolver"], "3")
        self.assertEqual(root["workspace"]["members"], [
            "monstertruck",
            "wasm",
            "monstertruck-qualification",
            "monstertruck-qualification/vendor/monstertruck-io",
            "monstertruck-qualification/vendor/monstertruck-meshing",
            "monstertruck-qualification/vendor/monstertruck-mesh",
            "monstertruck-qualification/vendor/monstertruck-traits",
            "monstertruck-qualification/vendor/monstertruck-solid",
        ])
        for crate in (*CAD_CRATES, "monstertruck-qualification"):
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
            for vendor_crate in (*build.VENDOR_CRATES, "monstertruck-solid"):
                src_vendor = build.CAD_ROOT / "monstertruck-qualification/vendor" / vendor_crate
                dst_vendor = nested / "cad/monstertruck-qualification/vendor" / vendor_crate
                (dst_vendor / "src").mkdir(parents=True)
                (dst_vendor / "src/lib.rs").write_text("")
                shutil.copy(src_vendor / "Cargo.toml", dst_vendor / "Cargo.toml")
            for crate in (*CAD_CRATES, "monstertruck-qualification"):
                (nested / "cad" / crate / "src").mkdir(parents=True)
                (nested / "cad" / crate / "src/lib.rs").write_text("")
                shutil.copy(build.CAD_ROOT / crate / "Cargo.toml", nested / "cad" / crate / "Cargo.toml")
            expected_packages = {
                "boardstudio-cad",
                "boardstudio-cad-wasm",
                "monstertruck-qualification",
                "monstertruck-io",
                "monstertruck-meshing",
                "monstertruck-mesh",
                "monstertruck-traits",
                "monstertruck-solid",
            }
            for crate in (*CAD_CRATES, "monstertruck-qualification"):
                with self.subTest(crate=crate):
                    manifest = nested / "cad" / crate / "Cargo.toml"
                    done = subprocess.run(["cargo", "metadata", "--no-deps", "--format-version", "1",
                                           "--manifest-path", str(manifest)], capture_output=True, text=True)
                    self.assertEqual(done.returncode, 0, done.stderr)
                    metadata = json.loads(done.stdout)
                    self.assertEqual(metadata["workspace_root"], str(nested / "cad"))
                    self.assertEqual({package["name"] for package in metadata["packages"]}, expected_packages)


class BrowserSmokeParserTests(unittest.TestCase):
    def test_result_is_read_from_the_rendered_page_and_unescaped(self):
        dom = '<html><pre id="result">{"ok":true,"error":null,"checks":[{"name":"a &amp; b","ok":true,"detail":"1 &lt; 2"}]}</pre></html>'
        result = browser.parse_result(dom)
        self.assertTrue(result["ok"])
        self.assertEqual(result["checks"][0]["name"], "a & b")
        self.assertEqual(result["checks"][0]["detail"], "1 < 2")

    def test_missing_or_unfinished_page_is_an_error_not_a_pass(self):
        with self.assertRaisesRegex(RuntimeError, "did not render"):
            browser.parse_result("<html></html>")
        with self.assertRaisesRegex(RuntimeError, "never finished"):
            browser.parse_result('<pre id="result">pending</pre>')

    def test_a_failed_check_makes_the_run_fail(self):
        failing = '{"ok":false,"error":null,"checks":[{"name":"x","ok":false,"detail":"d"}]}'
        with patch.object(browser, "serve"), patch.object(browser, "run_page", return_value=f'<pre id="result">{failing}</pre>'), patch.object(browser.Path, "exists", return_value=True):
            with contextlib.redirect_stdout(io.StringIO()), self.assertRaises(SystemExit) as raised:
                browser.main()
        self.assertEqual(raised.exception.code, 1)


class BrowserSessionCleanupTests(unittest.TestCase):
    def run_with_delete_delay(self, delay):
        class DriverHandler(http.server.BaseHTTPRequestHandler):
            def reply(self, value):
                body = json.dumps({"value": value}).encode()
                try:
                    self.send_response(200)
                    self.end_headers()
                    self.wfile.write(body)
                except (BrokenPipeError, ConnectionResetError):
                    pass

            def do_GET(self):
                self.reply({})

            def do_POST(self):
                body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
                if self.path == "/session":
                    self.reply({"sessionId": "test"})
                elif self.path.endswith("/execute/sync"):
                    self.reply("<html>done</html>" if "outerHTML" in body["script"] else "done")
                else:
                    self.reply(None)

            def do_DELETE(self):
                time.sleep(delay)
                self.reply(None)

            def log_message(self, *args):
                pass

        server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), DriverHandler)
        thread = threading.Thread(target=server.serve_forever, kwargs={"poll_interval": 0.05}, daemon=True)
        thread.start()
        self.addCleanup(server.server_close)
        self.addCleanup(server.shutdown)
        open_url = urllib.request.urlopen
        popen = subprocess.Popen
        drivers = []

        def start_driver(*args, **kwargs):
            driver = popen([sys.executable, "-c", "import time; time.sleep(30)"], **kwargs)
            drivers.append(driver)
            return driver

        def redirect(request, **kwargs):
            request.full_url = f"http://127.0.0.1:{server.server_port}" + request.selector
            return open_url(request, **kwargs)

        try:
            with patch.object(browser, "chromedriver", return_value="fake-driver"), \
                    patch.object(browser.subprocess, "Popen", side_effect=start_driver), \
                    patch.object(browser.urllib.request, "urlopen", side_effect=redirect):
                return browser.run_page("http://example.invalid", browser="fake-browser", timeout=10)
        finally:
            self.assertEqual(len(drivers), 1)
            self.assertIsNotNone(drivers[0].poll(), "owned driver must exit even when deletion times out")

    def test_session_delete_can_finish_after_750ms(self):
        self.assertEqual(self.run_with_delete_delay(1.0), "<html>done</html>")

    def test_session_delete_timeout_is_reported(self):
        # A slow graceful shutdown under parallel load warns instead of failing the
        # page: the sweep below still reaps everything, and any real remainder fails.
        started = time.monotonic()
        with contextlib.redirect_stderr(io.StringIO()) as warning:
            self.assertEqual(self.run_with_delete_delay(6.0), "<html>done</html>")
        self.assertIn("session deletion timed out", warning.getvalue())
        self.assertGreater(time.monotonic() - started, 4.5)
        self.assertLess(time.monotonic() - started, 10)


if __name__ == "__main__":
    unittest.main()
