#!/usr/bin/env python3
from __future__ import annotations

import contextlib
import importlib.util
import io
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
from unittest import mock

SPEC = importlib.util.spec_from_file_location("stop_hook", Path(__file__).with_name("gitnexus-stop-hook.py"))
hook = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(hook)


def make_repo(root: Path) -> None:
    root.mkdir(parents=True)
    for command in (["init", "-q"], ["config", "user.email", "t@t"], ["config", "user.name", "t"]):
        subprocess.run(["git", *command], cwd=root, check=True)
    (root / "a.txt").write_text("a", encoding="utf-8")
    subprocess.run(["git", "add", "a.txt"], cwd=root, check=True)
    subprocess.run(["git", "commit", "-qm", "init"], cwd=root, check=True)
    (root / "scripts").mkdir()
    (root / "scripts" / "gitnexus-refresh.py").write_text("# mocked by tests\n", encoding="utf-8")


def hook_input(session_id: str | None = None, cwd: Path | None = None, thread_id: str | None = None) -> str:
    value = {}
    if session_id is not None:
        value["session_id"] = session_id
    if thread_id is not None:
        value["thread_id"] = thread_id
    if cwd is not None:
        value["cwd"] = str(cwd)
    return json.dumps(value)


class StopHookTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.main = self.root / "main"
        make_repo(self.main)

    def add_worktree(self, name: str) -> Path:
        worktree = self.root / name
        subprocess.run(["git", "-C", str(self.main), "worktree", "add", "-q", str(worktree)], check=True)
        (worktree / "scripts").mkdir()
        (worktree / "scripts" / "gitnexus-refresh.py").write_text("# mocked by tests\n", encoding="utf-8")
        return worktree

    def register(self, session_id: str, worktree: Path) -> Path:
        self.assertEqual(hook.main(["--register", session_id, "--worktree", str(worktree)]), 0)
        return Path(subprocess.run(
            ["git", "-C", str(worktree), "rev-parse", "--path-format=absolute", "--git-common-dir"],
            check=True, capture_output=True, text=True,
        ).stdout.strip()) / hook.REGISTRATION_NAME

    def invoke_hook(self, payload: str):
        with mock.patch("sys.stdin", io.StringIO(payload)), mock.patch.object(hook, "run_refresh") as run_refresh:
            with contextlib.redirect_stdout(io.StringIO()) as stdout, contextlib.redirect_stderr(io.StringIO()) as stderr:
                result = hook.main([])
        return result, run_refresh, stdout.getvalue(), stderr.getvalue()

    def test_owner_session_can_select_integration_worktree_from_main_cwd(self):
        integration = self.add_worktree("integration")
        self.register("orchestrator-session", integration)

        result, run_refresh, stdout, stderr = self.invoke_hook(hook_input("orchestrator-session", self.main))

        self.assertEqual(result, 0)
        run_refresh.assert_called_once_with(integration)
        self.assertEqual(stdout, "")
        self.assertEqual(stderr, "")

    def test_lane_or_child_session_with_shared_cwd_cannot_refresh(self):
        integration = self.add_worktree("integration")
        self.register("orchestrator-session", integration)

        result, run_refresh, stdout, stderr = self.invoke_hook(hook_input("child-session", self.main))

        self.assertEqual(result, 0)
        run_refresh.assert_not_called()
        self.assertEqual(stdout, "")
        self.assertEqual(stderr, "")

    def test_refresh_invocation_is_silent_and_uses_registered_worktree(self):
        with mock.patch.object(hook.subprocess, "run") as run:
            hook.run_refresh(self.main)

        run.assert_called_once_with(
            [hook.sys.executable, str(self.main / "scripts" / "gitnexus-refresh.py"), "--background"],
            cwd=self.main,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        )

    def test_thread_id_is_used_when_session_id_is_absent(self):
        self.register("thread-owner", self.main)

        result, run_refresh, _, _ = self.invoke_hook(hook_input(cwd=self.main, thread_id="thread-owner"))

        self.assertEqual(result, 0)
        run_refresh.assert_called_once_with(self.main)

    def test_registration_uses_absolute_worktree_and_common_directory(self):
        with mock.patch.object(Path, "cwd", return_value=self.main):
            result = hook.main(["--register", "owner"])

        self.assertEqual(result, 0)
        common = Path(subprocess.run(
            ["git", "-C", str(self.main), "rev-parse", "--path-format=absolute", "--git-common-dir"],
            check=True, capture_output=True, text=True,
        ).stdout.strip())
        registration = json.loads((common / hook.REGISTRATION_NAME).read_text(encoding="utf-8"))
        self.assertEqual(registration, {"session_id": "owner", "worktree": str(self.main.resolve())})

    def test_missing_or_malformed_input_and_registration_are_silent(self):
        result, run_refresh, stdout, stderr = self.invoke_hook("not json")
        self.assertEqual((result, stdout, stderr), (0, "", ""))
        run_refresh.assert_not_called()

        self.register("owner", self.main)
        common = Path(subprocess.run(
            ["git", "-C", str(self.main), "rev-parse", "--path-format=absolute", "--git-common-dir"],
            check=True, capture_output=True, text=True,
        ).stdout.strip())
        registration_path = common / hook.REGISTRATION_NAME
        registration_path.write_text("{broken", encoding="utf-8")
        result, run_refresh, stdout, stderr = self.invoke_hook(hook_input("owner", self.main))
        self.assertEqual((result, stdout, stderr), (0, "", ""))
        run_refresh.assert_not_called()

        registration_path.unlink()
        result, run_refresh, stdout, stderr = self.invoke_hook(hook_input("owner", self.main))
        self.assertEqual((result, stdout, stderr), (0, "", ""))
        run_refresh.assert_not_called()

    def test_registered_target_in_another_git_family_is_rejected(self):
        elsewhere = self.root / "elsewhere"
        make_repo(elsewhere)
        main_common = Path(subprocess.run(
            ["git", "-C", str(self.main), "rev-parse", "--path-format=absolute", "--git-common-dir"],
            check=True, capture_output=True, text=True,
        ).stdout.strip())
        (main_common / hook.REGISTRATION_NAME).write_text(
            json.dumps({"session_id": "owner", "worktree": str(elsewhere.resolve())}), encoding="utf-8"
        )
        result, run_refresh, stdout, stderr = self.invoke_hook(hook_input("owner", self.main))
        self.assertEqual((result, stdout, stderr), (0, "", ""))
        run_refresh.assert_not_called()

    def test_removed_worktree_or_refresh_script_is_rejected(self):
        integration = self.add_worktree("integration")
        self.register("owner", integration)
        shutil.rmtree(integration)
        result, run_refresh, stdout, stderr = self.invoke_hook(hook_input("owner", self.main))
        self.assertEqual((result, stdout, stderr), (0, "", ""))
        run_refresh.assert_not_called()

        integration = self.add_worktree("integration-again")
        self.register("owner", integration)
        (integration / "scripts" / "gitnexus-refresh.py").unlink()
        result, run_refresh, stdout, stderr = self.invoke_hook(hook_input("owner", self.main))
        self.assertEqual((result, stdout, stderr), (0, "", ""))
        run_refresh.assert_not_called()

    def test_registration_errors_are_nonzero_and_explain_the_problem(self):
        with contextlib.redirect_stderr(io.StringIO()) as stderr:
            result = hook.main(["--register", "", "--worktree", str(self.main)])
        self.assertNotEqual(result, 0)
        self.assertIn("session", stderr.getvalue().lower())


if __name__ == "__main__":
    unittest.main()
