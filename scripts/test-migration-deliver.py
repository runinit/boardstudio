#!/usr/bin/env python3
"""Focused regression tests for migration-deliver's local commit guard."""

import os
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


SCRIPTS = Path(__file__).resolve().parent
GUARD = SCRIPTS / "migration-deliver.py"


def run_git(root, *args):
    return subprocess.run(["git", *args], cwd=root, text=True, capture_output=True, check=True)


def run_guard(root, *args, env=None, check=False):
    return subprocess.run(
        [sys.executable, str(GUARD), *args], cwd=root, text=True,
        capture_output=True, env=env, check=check,
    )


class MigrationDeliverTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="migration-deliver-test-")
        self.root = Path(self.temp.name) / "coordinator"
        self.root.mkdir()
        run_git(self.root, "init", "-b", "integration")
        run_git(self.root, "config", "user.name", "Guard Test")
        run_git(self.root, "config", "user.email", "guard-test@example.invalid")
        (self.root / "tracked.txt").write_text("base\n")
        run_git(self.root, "add", "tracked.txt")
        run_git(self.root, "commit", "-m", "initial")
        self.register(self.root, "coordinator", "integration")

    def tearDown(self):
        self.temp.cleanup()

    def register(self, root, role, branch):
        result = run_guard(root, "register", "--role", role, "--branch", branch)
        self.assertEqual(result.returncode, 0, result.stderr)

    def stage_change(self, root, content="changed\n"):
        (root / "tracked.txt").write_text(content)
        run_git(root, "add", "tracked.txt")

    def run_fake_focused_test(self, output, exit_code=0):
        bin_dir = Path(self.temp.name) / "fake-bin"
        bin_dir.mkdir(exist_ok=True)
        cargo = bin_dir / "cargo"
        cargo.write_text(
            "#!/bin/sh\n"
            f"printf '%s\\n' '{output}'\n"
            f"exit {exit_code}\n"
        )
        cargo.chmod(0o755)
        env = os.environ.copy()
        env["PATH"] = f"{bin_dir}{os.pathsep}{env['PATH']}"
        return run_guard(self.root, "focused-test", "--", "cargo", "test", "some_filter", env=env)

    def test_focused_test_rejects_zero_executed_tests(self):
        result = self.run_fake_focused_test(
            "test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 42 filtered out"
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("zero executed tests", result.stderr)

    def test_focused_test_rejects_all_ignored_tests(self):
        result = self.run_fake_focused_test(
            "test result: ok. 0 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out"
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("at least one passed test", result.stderr)

    def test_focused_test_accepts_positive_executed_test_count(self):
        result = self.run_fake_focused_test(
            "test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 40 filtered out"
        )
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_focused_test_preserves_command_failure(self):
        result = self.run_fake_focused_test(
            "test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 41 filtered out",
            exit_code=7,
        )
        self.assertEqual(result.returncode, 7)
        self.assertIn("test result: FAILED", result.stdout)

    def test_registered_wrong_branch_fails_and_unregistered_wrapper_rejects(self):
        self.assertEqual(run_guard(self.root, "check").returncode, 0)
        run_git(self.root, "checkout", "-b", "unexpected")
        mismatch = run_guard(self.root, "check")
        self.assertNotEqual(mismatch.returncode, 0)
        self.assertIn("expected 'integration'", mismatch.stderr)
        unregistered = Path(self.temp.name) / "unregistered"
        unregistered.mkdir()
        run_git(unregistered, "init", "-b", "other")
        result = run_guard(unregistered, "commit", "--intent", "delivery", "-m", "no")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("not registered", result.stderr)

    def test_registration_requires_current_branch_and_explicit_conflict_replacement(self):
        wrong = run_guard(self.root, "register", "--role", "coordinator", "--branch", "other")
        self.assertNotEqual(wrong.returncode, 0)
        self.assertIn("does not match current branch", wrong.stderr)
        conflict = run_guard(self.root, "register", "--role", "author", "--branch", "integration")
        self.assertNotEqual(conflict.returncode, 0)
        self.assertIn("--replace-registration", conflict.stderr)

    def test_author_cannot_make_integration_commit_and_isolated_author_can_deliver(self):
        author = Path(self.temp.name) / "author"
        run_git(self.root, "worktree", "add", "-b", "author-branch", str(author), "HEAD")
        run_git(author, "config", "user.name", "Guard Test")
        run_git(author, "config", "user.email", "guard-test@example.invalid")
        self.register(author, "author", "author-branch")
        denied = run_guard(author, "commit", "--intent", "integration", "-m", "bad")
        self.assertNotEqual(denied.returncode, 0)
        self.assertIn("author role", denied.stderr)
        self.stage_change(author)
        allowed = run_guard(author, "commit", "--intent", "delivery", "-m", "author delivery")
        self.assertEqual(allowed.returncode, 0, allowed.stderr)

    def test_direct_coordinator_commit_is_blocked_but_delivery_wrapper_allows_it(self):
        run_guard(self.root, "install", check=True)
        self.stage_change(self.root)
        direct = subprocess.run(
            ["git", "commit", "-m", "direct integration"], cwd=self.root,
            text=True, capture_output=True,
        )
        self.assertNotEqual(direct.returncode, 0)
        self.assertIn("must use migration-deliver.py commit", direct.stderr)

        self.stage_change(self.root, "wrapper commit\n")
        wrapped = run_guard(self.root, "commit", "--intent", "integration", "-m", "wrapped integration")
        self.assertEqual(wrapped.returncode, 0, wrapped.stderr)
        self.assertIn("wrapped integration", run_git(self.root, "log", "-1", "--format=%s").stdout)

    def test_build_freeze_blocks_coordinator_and_releases_after_exception(self):
        sys.path.insert(0, str(SCRIPTS))
        try:
            import importlib.util
            spec = importlib.util.spec_from_file_location("migration_deliver_test_module", GUARD)
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
        finally:
            sys.path.pop(0)

        self.stage_change(self.root)
        with self.assertRaisesRegex(RuntimeError, "build failed"):
            with module.build_freeze(self.root):
                author = Path(self.temp.name) / "freeze-author"
                run_git(self.root, "worktree", "add", "-b", "freeze-author-branch", str(author), "HEAD")
                run_git(author, "config", "user.name", "Guard Test")
                run_git(author, "config", "user.email", "guard-test@example.invalid")
                self.register(author, "author", "freeze-author-branch")
                (author / "author-only.txt").write_text("isolated delivery\n")
                run_git(author, "add", "author-only.txt")
                author_delivery = run_guard(author, "commit", "--intent", "delivery", "-m", "during coordinator build")
                self.assertEqual(author_delivery.returncode, 0, author_delivery.stderr)
                direct_env = os.environ.copy()
                direct_env["MIGRATION_DELIVERY_INTENT"] = "integration"
                direct = run_guard(self.root, "hook", env=direct_env)
                self.assertNotEqual(direct.returncode, 0)
                self.assertIn("build-frozen", direct.stderr)
                wrapped = run_guard(self.root, "commit", "--intent", "integration", "-m", "during build")
                self.assertNotEqual(wrapped.returncode, 0)
                self.assertIn("build-frozen", wrapped.stderr)
                raise RuntimeError("build failed")
        self.assertEqual(run_guard(self.root, "check").returncode, 0)
        wrapped = run_guard(self.root, "commit", "--intent", "integration", "-m", "after build")
        self.assertEqual(wrapped.returncode, 0, wrapped.stderr)

    def test_wrapper_reaps_stale_marker_before_taking_its_shared_commit_lock(self):
        import importlib.util
        spec = importlib.util.spec_from_file_location("migration_deliver_stale_test", GUARD)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)

        marker = module._freeze_path(self.root)
        marker.parent.mkdir(parents=True, exist_ok=True)
        marker.write_text(json.dumps({
            "root": str(self.root.resolve()),
            "pid": 999999999,
            "started_unix": 1,
            "token": "left-by-interrupted-build",
        }) + "\n")
        self.stage_change(self.root, "stale marker recovery\n")
        result = run_guard(self.root, "commit", "--intent", "integration", "-m", "recover stale freeze")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse(marker.exists(), "wrapper should remove marker after the kernel freeze lease is gone")

    def test_build_cannot_start_while_guarded_commit_holds_checkout_lock(self):
        import importlib.util
        spec = importlib.util.spec_from_file_location("migration_deliver_lock_test", GUARD)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)

        common = Path(run_git(self.root, "rev-parse", "--path-format=absolute", "--git-common-dir").stdout.strip())
        hooks = common / "hooks"
        hooks.mkdir(parents=True, exist_ok=True)
        entered = Path(self.temp.name) / "slow-hook-entered"
        released = Path(self.temp.name) / "slow-hook-release"
        old_hook = hooks / "pre-commit"
        old_hook.write_text(
            "#!/bin/sh\n"
            f"touch '{entered}'\n"
            f"while [ ! -e '{released}' ]; do sleep 0.02; done\n"
        )
        old_hook.chmod(0o755)
        installed = run_guard(self.root, "install")
        self.assertEqual(installed.returncode, 0, installed.stderr)
        self.stage_change(self.root, "commit holds checkout lock\n")
        process = subprocess.Popen(
            [sys.executable, str(GUARD), "commit", "--intent", "integration", "-m", "lock holder"],
            cwd=self.root, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        )
        try:
            for _ in range(250):
                if entered.exists():
                    break
                if process.poll() is not None:
                    break
                __import__("time").sleep(0.02)
            self.assertTrue(entered.exists(), "pre-commit chain did not begin")
            with self.assertRaisesRegex(module.GuardError, "already active"):
                with module.build_freeze(self.root):
                    pass
        finally:
            released.touch()
            stdout, stderr = process.communicate(timeout=5)
        self.assertEqual(process.returncode, 0, stderr or stdout)

    def test_unregistered_hook_noops_and_existing_hook_is_chained(self):
        unrelated = Path(self.temp.name) / "unrelated"
        unrelated.mkdir()
        run_git(unrelated, "init", "-b", "other")
        no_op = run_guard(unrelated, "hook")
        self.assertEqual(no_op.returncode, 0, no_op.stderr)

        common = Path(run_git(self.root, "rev-parse", "--path-format=absolute", "--git-common-dir").stdout.strip())
        hooks = common / "hooks"
        hooks.mkdir(parents=True, exist_ok=True)
        prior = hooks / "pre-commit"
        marker = Path(self.temp.name) / "prior-hook-ran"
        prior.write_text(f"#!/bin/sh\nprintf yes > {marker}\n")
        prior.chmod(0o755)
        installed = run_guard(self.root, "install")
        self.assertEqual(installed.returncode, 0, installed.stderr)
        chain = hooks / "pre-commit.migration-deliver-chain"
        self.assertTrue(chain.exists(), f"install={installed.stdout!r} {installed.stderr!r}; hooks={list(hooks.iterdir())!r}")
        chained = subprocess.run([str(hooks / "pre-commit")], cwd=self.root, text=True, capture_output=True)
        self.assertEqual(chained.returncode, 1)  # Coordinator intent is absent.
        self.assertFalse(marker.exists(), "guard rejection must happen before the preserved hook runs")
        self.stage_change(self.root, "allowed wrapper commit\n")
        allowed = run_guard(self.root, "commit", "--intent", "integration", "-m", "hook chain allowed")
        self.assertEqual(allowed.returncode, 0, allowed.stderr)
        self.assertTrue(marker.exists(), "the preserved hook should run after a passing guard")

    def test_install_refuses_a_configured_hooks_path(self):
        run_git(self.root, "config", "core.hooksPath", ".custom-hooks")
        result = run_guard(self.root, "install")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("core.hooksPath is configured", result.stderr)


if __name__ == "__main__":
    unittest.main(verbosity=2)
