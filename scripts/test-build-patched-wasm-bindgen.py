#!/usr/bin/env python3
import importlib.util
import io
from pathlib import Path
import subprocess
import tarfile
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("builder", Path(__file__).with_name("build-patched-wasm-bindgen.py"))
builder = importlib.util.module_from_spec(spec)
spec.loader.exec_module(builder)

COMPATIBLE_LOCK = '[[package]]\nversion = "0.2.129" # Cargo package\nname = "wasm-bindgen"\n'
INVALID_LOCKS = {
    "zero packages": '[[package]]\nname = "unrelated"\nversion = "1.0.0"\n',
    "duplicate pinned packages": COMPATIBLE_LOCK * 2,
    "incompatible version": COMPATIBLE_LOCK.replace("0.2.129", "0.2.128"),
}


class BuildTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.cache = self.root / "cache"
        self.cache.mkdir()
        self.output = self.root / "pair"
        self.lock = self.root / "Cargo.lock"
        self.lock.write_text(COMPATIBLE_LOCK)
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
        original_run = subprocess.run
        def dispatch(cmd, **kwargs):
            if cmd[0] == "cargo":
                return cargo(cmd, **kwargs)
            return original_run(cmd, **kwargs)
        with patch.object(builder.subprocess, "run", side_effect=dispatch):
            self.run_build()
        self.assertEqual(len(commands), 1)
        self.assertEqual(commands[0][3:7], ["build", "--release", "--locked", "--offline"])
        self.assertEqual([commands[0][i + 1] for i, v in enumerate(commands[0]) if v == "--bin"], list(builder.BINS))
        self.assertTrue(any(value.startswith('patch.crates-io.tiny_http.path=') for value in commands[0]))
        self.assertTrue(all((self.output / n).is_file() for n in builder.BINS))
        self.assertEqual(sorted(p.name for p in self.root.iterdir()), ["Cargo.lock", "cache", "pair"])

    def test_rejects_incompatible_workspace_before_build_or_cache_key(self):
        for case, lock in INVALID_LOCKS.items():
            with self.subTest(case=case), patch.object(builder, "pinned_env") as pinned_env:
                self.lock.write_text(lock)
                with self.assertRaisesRegex(ValueError, "exactly one wasm-bindgen"):
                    self.run_build()
                with self.assertRaisesRegex(ValueError, "exactly one wasm-bindgen"):
                    builder.cache_key(self.lock)
                pinned_env.assert_not_called()
                self.assertFalse(self.output.exists())

    def test_rejects_existing_output(self):
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


class CacheKeyTest(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)
        self.lock = self.root / "Cargo.lock"
        self.lock.write_text(COMPATIBLE_LOCK + INVALID_LOCKS["zero packages"])
        self.script = self.root / "builder.py"
        self.script.write_bytes(Path(builder.__file__).read_bytes())
        self.toolchain = self.root / "rust-toolchain.toml"
        self.toolchain.write_text('[toolchain]\nchannel = "1.98.0"\n')
        self.config = self.root / ".cargo/config.toml"
        self.config.parent.mkdir()
        self.config.write_text("[build]\njobs = 12\n")
        self.enterContext(patch.multiple(builder, ROOT=self.root, __file__=str(self.script)))
        self.enterContext(patch.dict(builder.os.environ, {"RUSTC": "fixture-rustc", "RUSTUP_TOOLCHAIN": "ambient"}))
        self.compiler = self.enterContext(patch.object(builder.subprocess, "check_output", return_value="fixture compiler"))
        self.system = self.enterContext(patch.object(builder.platform, "system", return_value="Linux"))
        self.machine = self.enterContext(patch.object(builder.platform, "machine", return_value="x86_64"))

    def test_unrelated_workspace_version_keeps_key(self):
        original = builder.cache_key(self.lock)
        self.assertRegex(original, r"^patched-wasm-bindgen-v1-[0-9a-f]{64}$")
        self.lock.write_text(self.lock.read_text().replace("1.0.0", "2.0.0"))
        self.assertEqual(builder.cache_key(self.lock), original)
        args, kwargs = self.compiler.call_args
        self.assertEqual(args, (["fixture-rustc", "-vV"],))
        self.assertEqual(kwargs["env"]["RUSTUP_TOOLCHAIN"], "1.98.0")

    def test_each_build_input_changes_key(self):
        original = builder.cache_key(self.lock)
        cli_checksum, tiny_checksum = builder.CRATES.values()
        for label, path, before, after in (
            ("tiny_http patch", self.script, builder.NEW, builder.NEW.replace("<=", "<")),
            ("CLI archive including its upstream lock", self.script, cli_checksum, "0" * 64),
            ("tiny_http archive", self.script, tiny_checksum, "0" * 64),
            ("pinned CLI version", self.script, 'VERSION = "0.2.129"', 'VERSION = "0.2.130"'),
            ("upstream lock adjustment", self.script, "text.replace(TINY_LOCK", "text.replace(OTHER_LOCK"),
            ("build recipe", self.script, '"build", "--release"', '"build", "--all-targets"'),
            ("toolchain file", self.toolchain, "1.98.0", "1.99.0"),
            ("Cargo configuration", self.config, "jobs = 12", "jobs = 8"),
        ):
            with self.subTest(input=label):
                source = path.read_text()
                self.assertGreater(source.count(before), 0)
                path.write_text(source.replace(before, after, 1))
                try:
                    self.assertNotEqual(builder.cache_key(self.lock), original)
                finally:
                    path.write_text(source)
        for label, probe, changed in (
            ("effective compiler", self.compiler, "different compiler"),
            ("target OS", self.system, "Darwin"),
            ("target architecture", self.machine, "aarch64"),
        ):
            with self.subTest(input=label), patch.object(probe, "return_value", changed):
                self.assertNotEqual(builder.cache_key(self.lock), original)


if __name__ == "__main__":
    unittest.main()
