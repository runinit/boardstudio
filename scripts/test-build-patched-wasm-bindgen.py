#!/usr/bin/env python3
import importlib.util
import io
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("builder", Path(__file__).with_name("build-patched-wasm-bindgen.py"))
builder = importlib.util.module_from_spec(spec)
spec.loader.exec_module(builder)


class BuildTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.cache = self.root / "cache"
        self.cache.mkdir()
        self.output = self.root / "pair"
        self.lock = self.root / "Cargo.lock"
        self.lock.write_text('name = "wasm-bindgen"\nversion = "0.2.129"\n')
        self.archive("wasm-bindgen-cli-0.2.129", {"Cargo.lock": builder.TINY_LOCK + '\n'})
        self.archive("tiny_http-0.12.0", {"src/util/task_pool.rs": builder.OLD + '\n'})

    def archive(self, name, files):
        with tarfile.open(self.cache / (name + ".crate"), "w:gz") as tar:
            for path, content in files.items():
                data = content.encode()
                info = tarfile.TarInfo(name + "/" + path)
                info.size = len(data)
                tar.addfile(info, io.BytesIO(data))

    def run_build(self, **kwargs):
        hashes = {name: builder.digest(self.cache / (name + ".crate")) for name in builder.CRATES}
        with patch.object(builder, "CRATES", hashes):
            builder.build(self.output, self.cache, True, self.lock, **kwargs)

    def test_build_patches_and_builds_both_locked_offline_then_installs(self):
        commands = []
        def cargo(cmd, **kwargs):
            commands.append(cmd)
            self.assertEqual(kwargs["env"]["RUSTUP_TOOLCHAIN"], "1.98.0")
            self.assertEqual(cmd[1:3], ["--config", str(builder.ROOT / ".cargo/config.toml")])
            source = Path(kwargs["cwd"])
            self.assertIn(builder.NEW, (source.parent / "tiny_http-0.12.0/src/util/task_pool.rs").read_text())
            self.assertNotIn('source = ', (source / "Cargo.lock").read_text())
            target = Path(kwargs["env"]["CARGO_TARGET_DIR"]) / "release"
            target.mkdir(parents=True)
            for name in builder.BINS:
                tool = target / name
                tool.write_text(f'#!/bin/sh\necho {name} 0.2.129\n')
                tool.chmod(0o755)
        import subprocess
        original_run = subprocess.run
        def dispatch(cmd, **kwargs):
            if cmd[0] == "cargo":
                return cargo(cmd, **kwargs)
            return original_run(cmd, **kwargs)
        with patch.object(builder.subprocess, "run", side_effect=dispatch):
            self.run_build()
        self.assertEqual(commands[0][3:7], ["build", "--release", "--locked", "--offline"])
        self.assertEqual([commands[0][i + 1] for i, v in enumerate(commands[0]) if v == "--bin"], list(builder.BINS))
        self.assertTrue(any(value.startswith('patch.crates-io.tiny_http.path=') for value in commands[0]))
        self.assertTrue(all((self.output / n).is_file() for n in builder.BINS))
        self.assertEqual(sorted(p.name for p in self.root.iterdir()), ["Cargo.lock", "cache", "pair"])

    def test_rejects_wrong_version_or_existing_output(self):
        self.lock.write_text('name = "wasm-bindgen"\nversion = "0.2.128"\n')
        with self.assertRaisesRegex(ValueError, "version"):
            self.run_build()
        self.lock.write_text('name = "wasm-bindgen"\nversion = "0.2.129"\n')
        self.output.mkdir()
        with self.assertRaisesRegex(ValueError, "exists"):
            self.run_build()

    def test_rejects_changed_patch_and_cleans_failure(self):
        self.archive("tiny_http-0.12.0", {"src/util/task_pool.rs": "different"})
        with self.assertRaisesRegex(ValueError, "patch"):
            self.run_build()
        self.assertFalse(self.output.exists())
        self.assertEqual(sorted(p.name for p in self.root.iterdir()), ["Cargo.lock", "cache"])

    def test_checksum_and_lock_mismatch_fail_closed(self):
        with self.assertRaisesRegex(ValueError, "checksum"):
            builder.build(self.output, self.cache, True, self.lock)
        self.archive("wasm-bindgen-cli-0.2.129", {"Cargo.lock": "unexpected"})
        with self.assertRaisesRegex(ValueError, "lockfile"):
            self.run_build()
        self.assertFalse(self.output.exists())
        self.assertEqual(sorted(p.name for p in self.root.iterdir()), ["Cargo.lock", "cache"])

    def test_cargo_failure_cleans_staging(self):
        with patch.object(builder.subprocess, "run", side_effect=RuntimeError("build failed")):
            with self.assertRaisesRegex(RuntimeError, "build failed"):
                self.run_build()
        self.assertFalse(self.output.exists())
        self.assertEqual(sorted(p.name for p in self.root.iterdir()), ["Cargo.lock", "cache"])


if __name__ == "__main__":
    unittest.main()
