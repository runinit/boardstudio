#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import json
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest import mock

SPEC = importlib.util.spec_from_file_location("refresh", Path(__file__).with_name("gitnexus-refresh.py"))
refresh = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(refresh)


def make_repo(root: Path) -> None:
    for command in (["init", "-q"], ["config", "user.email", "t@t"], ["config", "user.name", "t"]):
        subprocess.run(["git", *command], cwd=root, check=True)
    (root / "a.txt").write_text("a")
    subprocess.run(["git", "add", "a.txt"], cwd=root, check=True)
    subprocess.run(["git", "commit", "-qm", "init"], cwd=root, check=True)


class RefreshTests(unittest.TestCase):
    def test_stale_reasons(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            make_repo(root)
            self.assertIn("no index", refresh.stale_reason(root))
            head = refresh.git("rev-parse", "HEAD", cwd=root)
            (root / ".gitnexus").mkdir()
            meta = root / ".gitnexus" / "meta.json"
            meta.write_text(json.dumps({"lastCommit": "0" * 40}))
            self.assertIn("HEAD is", refresh.stale_reason(root))
            meta.write_text(json.dumps({"lastCommit": head}))
            self.assertIsNone(refresh.stale_reason(root))
            (root / "b.txt").write_text("b")
            self.assertIn("uncommitted", refresh.stale_reason(root))

    def test_the_index_directory_itself_never_counts_as_a_change(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            make_repo(root)
            (root / ".gitnexus").mkdir()
            (root / ".gitnexus" / "meta.json").write_text(json.dumps({"lastCommit": refresh.git("rev-parse", "HEAD", cwd=root)}))
            (root / ".gitnexus" / "lbug").write_text("db")
            self.assertEqual(refresh.dirty_sources(root), [])

    def test_alias_is_unique_for_worktrees(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "tce-x"
            root.mkdir()
            (root / ".git").write_text("gitdir: elsewhere")
            self.assertEqual(refresh.alias(root), "boardstudio-tce-x")
            (root / ".git").unlink()
            (root / ".git").mkdir()
            self.assertEqual(refresh.alias(root), "boardstudio")

    def test_command_never_rewrites_agent_instruction_files(self):
        command = refresh.analyze_command()
        self.assertIn("--index-only", command)
        self.assertIn("--name", command)

    def test_a_second_caller_does_not_start_a_second_analyze(self):
        import fcntl
        with tempfile.TemporaryDirectory() as tmp:
            lock_path = Path(tmp) / "gitnexus-refresh.lock"
            with open(lock_path, "w") as held:
                fcntl.flock(held, fcntl.LOCK_EX | fcntl.LOCK_NB)
                with mock.patch.object(refresh, "stale_reason", return_value="stale"), \
                        mock.patch.object(refresh, "git", return_value=str(Path(tmp))), \
                        mock.patch.object(refresh.subprocess, "run") as run:
                    self.assertEqual(refresh.main([]), 0)
                    run.assert_not_called()


if __name__ == "__main__":
    unittest.main()
