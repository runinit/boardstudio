"""Regression tests for exact-input migration gate receipts."""

import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest


SCRIPTS = Path(__file__).resolve().parent
MODULE_PATH = SCRIPTS / "migration_gate_receipts.py"
spec = importlib.util.spec_from_file_location("migration_gate_receipts_tests", MODULE_PATH)
receipts = importlib.util.module_from_spec(spec)
spec.loader.exec_module(receipts)


def git(root, *args):
    return subprocess.run(["git", *args], cwd=root, check=True, text=True, capture_output=True)


LOG_DIGEST = "a" * 64


class MigrationGateReceiptTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="migration-gate-receipts-")
        self.root = Path(self.temp.name) / "repo"
        self.root.mkdir()
        git(self.root, "init", "-b", "main")
        git(self.root, "config", "user.name", "Receipt Test")
        git(self.root, "config", "user.email", "receipt-test@example.invalid")
        (self.root / "Cargo.toml").write_text("[workspace]\n")
        (self.root / "src.rs").write_text("pub fn current() {}\n")
        git(self.root, "add", "Cargo.toml", "src.rs")
        git(self.root, "commit", "-m", "fixture")

    def tearDown(self):
        self.temp.cleanup()

    def identity(self, **kwargs):
        command = kwargs.pop("command", [sys.executable, "--version"])
        return receipts.make_identity(self.root, "test-gate", command, **kwargs)

    def test_success_receipt_is_atomic_reusable_and_has_timing_and_log_hash(self):
        identity = self.identity()
        started = time.time() - 2
        receipt = receipts.record_success(
            self.root, identity, started=started, ended=time.time(),
            counts={"passed": 2, "failed": 0}, log_digest=LOG_DIGEST,
        )
        self.assertIsNotNone(receipts.lookup(self.root, identity, require_positive=True))
        self.assertGreater(receipt["duration_seconds"], 0)
        self.assertEqual(receipt["log_sha256"], "a" * 64)
        self.assertEqual(list((receipts.cache_directory(self.root) / "test-gate").glob(".*")), [])

    def test_source_or_command_change_invalidates_success(self):
        identity = self.identity()
        receipts.record_success(self.root, identity, started=1, ended=2,
                                counts={"passed": 1, "failed": 0}, log_digest=LOG_DIGEST)
        (self.root / "src.rs").write_text("pub fn changed() {}\n")
        self.assertIsNone(receipts.lookup(self.root, self.identity(), require_positive=True))
        self.assertNotEqual(identity["key"], self.identity(command=[sys.executable, "-c", "pass"])["key"])

    def test_staged_source_content_participates_even_when_worktree_matches(self):
        original = (self.root / "src.rs").read_text()
        identity = self.identity()
        (self.root / "src.rs").write_text("pub fn staged() {}\n")
        git(self.root, "add", "src.rs")
        (self.root / "src.rs").write_text(original)
        self.assertNotEqual(identity["key"], self.identity()["key"])

    def test_document_and_unrelated_index_churn_do_not_change_gate_identity(self):
        (self.root / "handoff.md").write_text("first\n")
        git(self.root, "add", "handoff.md")
        before = self.identity()
        fingerprint = receipts.source_fingerprint(self.root)
        (self.root / "handoff.md").write_text("second\n")
        git(self.root, "add", "handoff.md")
        after = self.identity()
        self.assertEqual(before["key"], after["key"])
        self.assertEqual(fingerprint, receipts.source_fingerprint(self.root))

    def test_final_fingerprint_preserves_maintained_source_index_and_config_drift(self):
        import shutil
        scripts = self.root / "scripts"
        scripts.mkdir()
        shutil.copy2(SCRIPTS / "build-m1.py", scripts / "build-m1.py")
        source = self.root / "web/src/owner.rs"
        source.parent.mkdir(parents=True)
        source.write_text("pub fn before() {}\n")
        git(self.root, "add", "scripts/build-m1.py", "web/src/owner.rs")
        before = receipts.source_fingerprint(self.root)
        for name in ("CONSTRAINTS.md", "handoff.md", "docs/migration/run.json"):
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("record update\n")
            git(self.root, "add", name)
            self.assertEqual(before, receipts.source_fingerprint(self.root))
        source.write_text("pub fn after() {}\n")
        self.assertNotEqual(before, receipts.source_fingerprint(self.root))
        git(self.root, "add", "web/src/owner.rs")
        source.write_text("pub fn before() {}\n")
        self.assertNotEqual(before, receipts.source_fingerprint(self.root))
        git(self.root, "add", "web/src/owner.rs")
        self.assertEqual(before, receipts.source_fingerprint(self.root))
        config = self.root / ".cargo/config.toml"
        config.parent.mkdir()
        config.write_text("[build]\nrustflags = ['--cfg', 'changed']\n")
        self.assertNotEqual(before, receipts.source_fingerprint(self.root))

    def test_maintained_symlink_hashes_target_bytes_and_rejects_escaping_target(self):
        target = self.root / "src-target.rs"
        target.write_text("pub fn before() {}\n")
        link = self.root / "src-link.rs"
        link.symlink_to(target.name)
        git(self.root, "add", "src-link.rs", "src-target.rs")
        before = self.identity()
        target.write_text("pub fn after() {}\n")
        self.assertNotEqual(before["key"], self.identity()["key"])
        elsewhere = Path(self.temp.name) / "outside.rs"
        elsewhere.write_text("outside\n")
        link.unlink()
        link.symlink_to(elsewhere)
        with self.assertRaisesRegex(ValueError, "escapes the repository"):
            self.identity()

    def test_wasm_gate_identity_tracks_rustup_selected_compiler_bytes(self):
        bin_dir = self.root / "bin"
        sysroot = self.root / "toolchain"
        (sysroot / "bin").mkdir(parents=True)
        bin_dir.mkdir()
        cargo = bin_dir / "cargo"
        cargo.write_text("#!/bin/sh\nexit 0\n")
        cargo.chmod(0o755)
        rustc_proxy = bin_dir / "rustc"
        rustc_proxy.write_text(
            "#!/bin/sh\n"
            f"if [ \"$1\" = --print ]; then echo '{sysroot}'; exit 0; fi\n"
            "echo 'rustc selected-toolchain'\n"
        )
        rustc_proxy.chmod(0o755)
        selected = sysroot / "bin/rustc"
        selected.write_text("selected compiler one\n")
        selected.chmod(0o755)
        env = {"PATH": str(bin_dir), "RUSTC": str(rustc_proxy)}
        first = receipts.make_identity(
            self.root, "wasm-headless-tests", [sys.executable, "runner.py"], env=env,
        )
        selected.write_text("selected compiler two\n")
        second = receipts.make_identity(
            self.root, "wasm-headless-tests", [sys.executable, "runner.py"], env=env,
        )
        self.assertNotEqual(first["key"], second["key"])
        tool = next(item for item in second["tools"] if item["command"] == "selected-rustc")
        self.assertEqual(tool["path"], str(selected))

    def test_subprocess_receipts_reuse_exact_inputs_and_invalidate_changes(self):
        repo = Path(self.temp.name) / "cache-experiment"
        repo.mkdir()
        git(repo, "init", "-b", "main")
        git(repo, "config", "user.name", "Receipt Test")
        git(repo, "config", "user.email", "receipt-test@example.invalid")
        (repo / "Cargo.toml").write_text("[workspace]\n")
        (repo / "source.rs").write_text("pub fn original() {}\n")
        (repo / "handoff.md").write_text("initial docs\n")
        git(repo, "add", "Cargo.toml", "source.rs", "handoff.md")
        git(repo, "commit", "-m", "fixture")
        counter = Path(self.temp.name) / "execution-count"
        tool = Path(self.temp.name) / "gate-tool.py"
        cargo_home = Path(self.temp.name) / "cargo-home"
        cargo_home.mkdir()
        cargo_config = cargo_home / "config.toml"
        cargo_config.write_text("[build]\njobs = 2\n")
        env = {"PATH": os.environ.get("PATH", ""), "CARGO_HOME": str(cargo_home)}
        parser = lambda _output: {"complete": True, "counts": {"passed": 1, "failed": 0}}

        def invoke():
            started = time.monotonic()
            receipt, reused, process = receipts.run_cached(
                repo, "subprocess-experiment", [sys.executable, str(tool)], env=env,
                extra_tools=(str(tool),), result_parser=parser, require_positive=True,
            )
            self.assertEqual(process.returncode, 0, process.stderr)
            return receipt, reused, time.monotonic() - started

        def write_tool(label):
            tool.write_text(
                "from pathlib import Path\n"
                f"counter = Path({str(counter)!r})\n"
                "count = int(counter.read_text()) if counter.exists() else 0\n"
                "counter.write_text(str(count + 1))\n"
                f"print({label!r})\n"
            )

        events = []
        write_tool("first tool")
        receipt, reused, elapsed = invoke()
        self.assertFalse(reused)
        events.append({"change": "initial", "executed": True, "counter": 1,
                       "receipt_duration_seconds": receipt["duration_seconds"],
                       "elapsed_seconds": elapsed})
        receipt, reused, elapsed = invoke()
        self.assertTrue(reused)
        self.assertEqual(counter.read_text(), "1")
        events.append({"change": "identical rerun", "executed": False, "counter": 1,
                       "receipt_duration_seconds": receipt["duration_seconds"],
                       "elapsed_seconds": elapsed})
        (repo / "handoff.md").write_text("documentation only\n")
        git(repo, "add", "handoff.md")
        receipt, reused, elapsed = invoke()
        self.assertTrue(reused)
        self.assertEqual(counter.read_text(), "1")
        events.append({"change": "docs and unrelated index", "executed": False, "counter": 1,
                       "receipt_duration_seconds": receipt["duration_seconds"],
                       "elapsed_seconds": elapsed})
        (repo / "source.rs").write_text("pub fn changed() {}\n")
        receipt, reused, elapsed = invoke()
        self.assertFalse(reused)
        self.assertEqual(counter.read_text(), "2")
        events.append({"change": "maintained source bytes", "executed": True, "counter": 2,
                       "receipt_duration_seconds": receipt["duration_seconds"],
                       "elapsed_seconds": elapsed})
        cargo_config.write_text("[build]\njobs = 4\n")
        receipt, reused, elapsed = invoke()
        self.assertFalse(reused)
        self.assertEqual(counter.read_text(), "3")
        events.append({"change": "external Cargo config bytes", "executed": True, "counter": 3,
                       "receipt_duration_seconds": receipt["duration_seconds"],
                       "elapsed_seconds": elapsed})
        write_tool("changed tool")
        receipt, reused, elapsed = invoke()
        self.assertFalse(reused)
        self.assertEqual(counter.read_text(), "4")
        events.append({"change": "runner tool bytes", "executed": True, "counter": 4,
                       "receipt_duration_seconds": receipt["duration_seconds"],
                       "elapsed_seconds": elapsed})
        self.subprocess_experiment = events

    def test_external_cargo_configuration_content_participates_in_identity(self):
        cargo_home = self.root / "private-cargo-home"
        cargo_home.mkdir()
        config = cargo_home / "config.toml"
        config.write_text("[build]\njobs = 2\n")
        env = {"PATH": os.environ.get("PATH", ""), "CARGO_HOME": str(cargo_home)}
        before = self.identity(env=env)
        config.write_text("[build]\njobs = 4\n")
        self.assertNotEqual(before["key"], self.identity(env=env)["key"])
        self.assertNotIn("jobs = 4", json.dumps(self.identity(env=env)))

    def test_runner_override_executable_content_participates_in_identity(self):
        runner = self.root / "fake-runner"
        runner.write_text("#!/bin/sh\necho first\n")
        runner.chmod(0o755)
        env = {"PATH": os.environ.get("PATH", ""),
               "BOARDSTUDIO_WASM_TEST_COMMAND": str(runner)}
        first = self.identity(env=env)
        runner.write_text("#!/bin/sh\necho second\n")
        self.assertNotEqual(first["key"], self.identity(env=env)["key"])

    def test_secret_environment_values_are_hashed_and_never_serialized(self):
        secret = "token-that-must-not-be-stored"
        identity = self.identity(env={"PATH": os.environ.get("PATH", ""), "CARGO_HOME": secret})
        receipts.record_success(self.root, identity, started=1, ended=2,
                                counts={"passed": 1, "failed": 0}, log_digest=LOG_DIGEST)
        saved = next((receipts.cache_directory(self.root) / "test-gate").glob("*.json")).read_text()
        self.assertNotIn(secret, saved)
        self.assertEqual(identity["environment_sha256"]["CARGO_HOME"], receipts._digest(secret.encode()))

    def test_failed_incomplete_or_zero_test_results_are_not_reusable(self):
        identity = self.identity()
        with self.assertRaises(ValueError):
            receipts.record_success(self.root, identity, started=1, ended=2,
                                    counts={"passed": 0, "failed": 0}, log_digest=LOG_DIGEST)
        self.assertIsNone(receipts.lookup(self.root, identity, require_positive=True))
        with self.assertRaises(ValueError):
            receipts.record_success(self.root, identity, started=1, ended=2,
                                    counts={"passed": 2, "failed": 1}, log_digest=LOG_DIGEST)
        with self.assertRaises(ValueError):
            receipts.record_success(self.root, identity, started=1, ended=2,
                                    counts={"passed": 2}, log_digest=LOG_DIGEST, complete=False)
        with self.assertRaises(ValueError):
            receipts.record_success(self.root, identity, started=1, ended=2,
                                    counts={"passed": 2, "incomplete": 1}, log_digest=LOG_DIGEST)
        self.assertIsNone(receipts.lookup(self.root, identity, require_positive=True))

    def test_cached_runner_rejects_source_drift_during_command(self):
        target = self.root / "src.rs"
        command = [sys.executable, "-c", "from pathlib import Path; Path('src.rs').write_text('drift')"]
        receipt, reused, result = receipts.run_cached(
            self.root, "mutating-gate", command,
            result_parser=lambda _output: {"complete": True, "counts": {"passed": 1, "failed": 0}},
            require_positive=True,
        )
        self.assertEqual(result.returncode, 0)
        self.assertFalse(reused)
        self.assertTrue(receipt.get("source_drift"))
        self.assertEqual(list((receipts.cache_directory(self.root) / "mutating-gate").glob("*.json")), [])

    def test_known_exclusions_are_not_successful_receipts(self):
        identity = self.identity()
        with self.assertRaises(ValueError):
            receipts.record_success(self.root, identity, started=1, ended=2,
                                    counts={"passed": 9, "failed": 0, "excluded_failures": 1}, log_digest=LOG_DIGEST)


if __name__ == "__main__":
    unittest.main(verbosity=2)
