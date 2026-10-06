#!/usr/bin/env python3
"""Verify download integrity, cache reuse and build failure propagation."""
import hashlib
import io
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import cadrum_build as build


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

    def test_container_override_preserves_mounts_and_build_failure_stops_run(self):
        pkg = self.root / "wasm/pkg"
        pkg.mkdir(parents=True)
        (pkg / "package.json").write_text("generated")
        (pkg / "provider.wasm").write_bytes(b"keep")
        with patch.dict(build.os.environ, {"CADRUM_CONTAINER_RUNTIME": "podman"}), patch.object(build, "prepare", return_value=self.root / ".cache/cadrum/wasm/occt"), patch.object(build, "run") as run:
            build.build_wasm()
        commands = [call.args[0] for call in run.call_args_list]
        self.assertEqual(commands[0][:2], ["podman", "build"])
        self.assertIn(f"{self.root}:/workspace/cad:Z", commands[1])
        self.assertIn("OCCT_ROOT=/workspace/cad/.cache/cadrum/wasm/occt", commands[1])
        self.assertIn(f"{self.root.parent}/contracts/rust:/workspace/contracts/rust:ro", commands[1])
        self.assertFalse((pkg / "package.json").exists())
        self.assertEqual((pkg / "provider.wasm").read_bytes(), b"keep")
        with patch.dict(build.os.environ, {"CADRUM_CONTAINER_RUNTIME": "docker"}), patch.object(build, "prepare", return_value=self.root / ".cache/cadrum/wasm/occt"), patch.object(build, "run", side_effect=subprocess.CalledProcessError(1, "docker")) as run:
            with self.assertRaises(subprocess.CalledProcessError):
                build.build_wasm()
        run.assert_called_once()

    def test_native_failure_prevents_wasm_build(self):
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
        wasm.assert_called_once()

    def test_oracle_build_failure_stops_native_tests(self):
        with patch.object(build, "prepare", return_value=self.extracted), patch.object(build, "run", side_effect=[None, subprocess.CalledProcessError(1, "cargo")]) as run, patch.object(build, "build_wasm") as wasm:
            with self.assertRaises(subprocess.CalledProcessError):
                build.test_cadrum()
        self.assertEqual(run.call_count, 2)
        wasm.assert_not_called()

if __name__ == "__main__":
    unittest.main()
