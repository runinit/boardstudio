#!/usr/bin/env python3
"""Verify download integrity, cache reuse and build failure propagation."""
import contextlib
import hashlib
import importlib.util
import io
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import cadrum_build as build

_spec = importlib.util.spec_from_file_location("cadrum_browser", Path(__file__).with_name("test-cadrum-browser.py"))
browser = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(browser)


class CadrumBuildTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.payload = b"archive fixture"
        self.archive_name = "occt-fixture.tar.gz"
        self.cache = self.root / ".cache/cadrum/native"
        self.extracted = self.cache / "occt-fixture"
        self.patch = patch.multiple(build, CAD_ROOT=self.root, BUILDS={
            "native": (self.archive_name, hashlib.sha256(self.payload).hexdigest())})
        self.patch.start()
        self.addCleanup(self.patch.stop)

    def extract(self, command):
        self.assertEqual(command[0:2], ["tar", "-xzf"])
        (self.extracted / "include/opencascade").mkdir(parents=True)
        (self.extracted / "lib").mkdir()

    def test_download_is_verified_and_complete_cache_reused(self):
        with patch.object(build.urllib.request, "urlopen", return_value=io.BytesIO(self.payload)) as download, patch.object(build, "run", side_effect=self.extract) as extract:
            self.assertEqual(build.prepare("native"), self.extracted)
            self.assertEqual(build.prepare("native"), self.extracted)
        download.assert_called_once()
        extract.assert_called_once()
        self.assertFalse((self.cache / (self.archive_name + ".download")).exists())

    def test_corrupted_cached_archive_rejected_even_when_extracted(self):
        self.extract(["tar", "-xzf"])
        (self.cache / self.archive_name).write_bytes(b"corrupt")
        with patch.object(build, "run") as run, self.assertRaisesRegex(RuntimeError, "checksum mismatch"):
            build.prepare("native")
        run.assert_not_called()

    def test_interrupted_download_is_not_published(self):
        class BrokenDownload(io.BytesIO):
            def read(self, *args):
                raise OSError("interrupted")
        with patch.object(build.urllib.request, "urlopen", return_value=BrokenDownload()), self.assertRaises(OSError):
            build.prepare("native")
        self.assertFalse((self.cache / self.archive_name).exists())
        self.assertFalse((self.cache / (self.archive_name + ".download")).exists())

    def test_missing_library_directory_rejected(self):
        (self.extracted / "include/opencascade").mkdir(parents=True)
        (self.cache / self.archive_name).write_bytes(self.payload)
        with self.assertRaisesRegex(RuntimeError, "unexpected layout"):
            build.prepare("native")

    def wasm_sources(self):
        (self.root / "wasm/src").mkdir(parents=True)
        (self.root / "wasm/src/lib.rs").write_text("// provider")
        (self.root / "wasm/Containerfile").write_text("FROM scratch")

    def test_container_override_preserves_mounts_and_build_failure_stops_run(self):
        self.wasm_sources()
        pkg = self.root / "wasm/pkg"
        pkg.mkdir(parents=True)
        (pkg / "package.json").write_text("generated")
        (pkg / "provider.wasm").write_bytes(b"keep")
        with patch.dict(build.os.environ, {"CADRUM_CONTAINER_RUNTIME": "podman", "CADRUM_BUILD_IMAGE": "1"}), patch.object(build, "prepare", return_value=self.root / ".cache/cadrum/wasm/occt"), patch.object(build, "run") as run:
            build.build_wasm()
        commands = [call.args[0] for call in run.call_args_list]
        self.assertEqual(commands[0][:2], ["podman", "build"])
        self.assertIn(f"{self.root}:/workspace/cad:Z", commands[1])
        self.assertIn("OCCT_ROOT=/workspace/cad/.cache/cadrum/wasm/occt", commands[1])
        self.assertIn(f"{self.root.parent}/contracts/rust:/workspace/contracts/rust:ro", commands[1])
        self.assertIn(f"{self.root}/.cache/cargo-registry:/root/.cargo/registry:Z", commands[1])
        self.assertFalse((pkg / "package.json").exists())
        self.assertEqual((pkg / "provider.wasm").read_bytes(), b"keep")
        with patch.dict(build.os.environ, {"CADRUM_CONTAINER_RUNTIME": "docker", "CADRUM_BUILD_IMAGE": "1"}), patch.object(build, "prepare", return_value=self.root / ".cache/cadrum/wasm/occt"), patch.object(build, "run", side_effect=subprocess.CalledProcessError(1, "docker")) as run:
            with self.assertRaises(subprocess.CalledProcessError):
                build.build_wasm()
        run.assert_called_once()

    def test_published_image_is_pulled_by_containerfile_digest_with_local_fallback(self):
        self.wasm_sources()
        environment = {"CADRUM_CONTAINER_RUNTIME": "docker", "CADRUM_BUILD_IMAGE": ""}
        tag = build.registry_image()
        self.assertRegex(tag, r"^ghcr\.io/runinit/boardstudio-cadrum-wasm:containerfile-[0-9a-f]{16}$")
        with patch.dict(build.os.environ, environment), patch.object(build, "prepare", return_value=self.root / "occt"), \
                patch.object(build, "succeeds", return_value=True) as pull, patch.object(build, "run") as run:
            build.build_wasm()
        pull.assert_called_once_with(["docker", "pull", tag])
        commands = [call.args[0] for call in run.call_args_list]
        self.assertEqual(commands[0], ["docker", "tag", tag, build.IMAGE])
        self.assertNotIn("build", [command[1] for command in commands])
        (self.root / ".cache/cadrum/wasm-build.sha256").unlink()
        with patch.dict(build.os.environ, environment), patch.object(build, "prepare", return_value=self.root / "occt"), \
                patch.object(build, "succeeds", return_value=False), patch.object(build, "run") as run, \
                contextlib.redirect_stdout(io.StringIO()):
            build.build_wasm()
        self.assertEqual(run.call_args_list[0].args[0][:2], ["docker", "build"])

    def test_unchanged_provider_inputs_skip_the_container(self):
        self.wasm_sources()
        environment = {"CADRUM_CONTAINER_RUNTIME": "docker", "CADRUM_BUILD_IMAGE": "1"}
        output = self.root / "wasm/pkg/boardstudio_cadrum_wasm_bg.wasm"
        with patch.dict(build.os.environ, environment), patch.object(build, "prepare", return_value=self.root / "occt"), \
                patch.object(build, "run", side_effect=lambda *a, **k: output.parent.mkdir(parents=True, exist_ok=True) or output.write_bytes(b"wasm")):
            build.build_wasm()
        with patch.dict(build.os.environ, environment), patch.object(build, "run") as run, \
                contextlib.redirect_stdout(io.StringIO()):
            build.build_wasm()
        run.assert_not_called()
        (self.root / "wasm/src/lib.rs").write_text("// changed provider")
        with patch.dict(build.os.environ, environment), patch.object(build, "prepare", return_value=self.root / "occt"), \
                patch.object(build, "run") as run:
            build.build_wasm()
        self.assertEqual(run.call_args_list[0].args[0][:2], ["docker", "build"])

    def test_prebuilt_image_skips_only_the_image_build(self):
        self.wasm_sources()
        environment = {"CADRUM_CONTAINER_RUNTIME": "docker", "CADRUM_IMAGE_READY": "1"}
        with patch.dict(build.os.environ, environment), patch.object(build, "prepare", return_value=self.root / ".cache/cadrum/wasm/occt"), \
                patch.object(build, "succeeds") as pull, patch.object(build, "run") as run:
            build.build_wasm()
        pull.assert_not_called()
        commands = [call.args[0] for call in run.call_args_list]
        self.assertEqual([command[:2] for command in commands], [["docker", "run"]])
        self.assertIn(build.IMAGE, commands[0])

    def test_native_failure_is_raised(self):
        with patch.object(build, "prepare", return_value=self.extracted), patch.object(build, "run", side_effect=[None, None, subprocess.CalledProcessError(1, "cargo")]), patch.object(build, "build_wasm") as wasm:
            with self.assertRaises(subprocess.CalledProcessError):
                build.test_cadrum()
        wasm.assert_not_called()

    def test_oracle_is_built_with_the_verified_archive_before_native_tests(self):
        with patch.object(build, "prepare", return_value=self.extracted), patch.object(build, "run") as run, patch.object(build, "build_wasm") as wasm:
            build.test_cadrum()
        commands = [(call.args[0], call.kwargs.get("env")) for call in run.call_args_list]
        self.assertEqual([command[3] for command, _ in commands], ["../core/Cargo.toml", "step-oracle/Cargo.toml", "wasm/Cargo.toml"])
        oracle_command, oracle_env = commands[1]
        self.assertEqual(oracle_command[:2], ["cargo", "build"])
        self.assertIn("--locked", oracle_command)
        self.assertEqual(oracle_env["OCCT_ROOT"], str(self.extracted))
        self.assertEqual(oracle_env["CARGO_TARGET_DIR"], str(self.root / "step-oracle/target"))
        # The `build` step already produced the WASM provider; tests must not rebuild it.
        wasm.assert_not_called()

    def test_oracle_build_failure_stops_native_tests(self):
        with patch.object(build, "prepare", return_value=self.extracted), patch.object(build, "run", side_effect=[None, subprocess.CalledProcessError(1, "cargo")]) as run, patch.object(build, "build_wasm") as wasm:
            with self.assertRaises(subprocess.CalledProcessError):
                build.test_cadrum()
        self.assertEqual(run.call_count, 2)
        wasm.assert_not_called()

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
        with patch.object(browser, "serve"), patch.object(browser, "run_page", return_value=subprocess.CompletedProcess([], 0, f'<pre id="result">{failing}</pre>', "")), patch.object(browser.Path, "exists", return_value=True):
            with contextlib.redirect_stdout(io.StringIO()), self.assertRaises(SystemExit) as raised:
                browser.main()
        self.assertEqual(raised.exception.code, 1)


if __name__ == "__main__":
    unittest.main()
