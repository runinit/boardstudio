"""Focused tests for the migration browser wrapper; never starts a browser."""

import importlib.util
import io
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import Mock, patch
from contextlib import redirect_stderr


SCRIPT = Path(__file__).with_name("migration-browser.py")
SPEC = importlib.util.spec_from_file_location("migration_browser", SCRIPT)
browser = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(browser)


class MigrationBrowserTests(unittest.TestCase):
    def invoke(self, *args):
        return browser.main(list(args))

    def test_relative_upload_is_passed_as_absolute_path(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            file = root / "upload.step"
            file.write_text("step")
            with patch.object(browser, "ROOT", root), patch.object(browser.subprocess, "run") as run:
                run.return_value = subprocess.CompletedProcess([], 0, "uploaded", "")
                with patch("builtins.print"):
                    self.assertEqual(self.invoke("upload", "--build-id", "candidate-1",
                                                 "--purpose", "parts", "--selector", "#file",
                                                 str(file)), 0)
                command = run.call_args.args[0]
                self.assertEqual(command[-1], str(file.resolve()))
                self.assertTrue(Path(command[-1]).is_absolute())

    def test_missing_upload_fails_before_browser(self):
        with patch.object(browser.subprocess, "run") as run, patch("sys.stderr"):
            self.assertEqual(self.invoke("upload", "--build-id", "candidate-1",
                                         "--purpose", "parts", "--selector", "#file",
                                         "/definitely/missing/file"), 2)
            run.assert_not_called()

    def test_candidate_wrong_hash_fails(self):
        served = {"build_id": "candidate-1", "source_commit": "abc",
                  "provenance_sha256": "expected", "package_proof": "proof.json",
                  "root_url": "http://localhost/", "subpath_url": "http://localhost/sub/"}
        run = {"current_progress": {"served_candidate": served}}
        fake_progress = Mock()
        fake_progress.repository_file.return_value = Path("proof.json")
        fake_progress.validate_provenance.return_value = (
            {"build_id": "candidate-1", "source_commit": "abc",
             "provenance_sha256": "wrong"}, {})
        with patch.object(browser, "RUN") as path, patch.object(browser, "_progress",
                                                                   return_value=fake_progress):
            path.read_text.return_value = json.dumps(run)
            with self.assertRaisesRegex(browser.BrowserError, "proof verification failed"):
                browser.candidate("candidate-1")
        fake_progress.verify_candidate_routes.assert_not_called()

    def test_wait_passes_caller_predicate_and_timeout_not_sleep(self):
        with patch.object(browser, "run_browser", return_value="ready") as run, patch("builtins.print"):
            self.assertEqual(self.invoke("wait", "--build-id", "candidate-1", "--purpose", "pcb",
                                         "--fn", "window.ready === true", "--timeout-ms", "4321"), 0)
        self.assertIn("--fn", run.call_args.args[0])
        self.assertIn("window.ready === true", run.call_args.args[0])
        self.assertNotIn("2000", run.call_args.args[0])
        self.assertEqual(run.call_args.kwargs["timeout"], 9.321)
        self.assertEqual(run.call_args.kwargs["env"]["AGENT_BROWSER_DEFAULT_TIMEOUT"], "4321")

    def test_inspect_requests_scoped_full_snapshot(self):
        with patch.object(browser, "run_browser", return_value="tree") as run, patch("builtins.print"):
            self.assertEqual(self.invoke("inspect", "--build-id", "candidate-1", "--purpose", "layout",
                                         "--selector", "#inspector"), 0)
        command = run.call_args.args[0]
        self.assertIn("snapshot", command)
        self.assertIn("--selector", command)
        self.assertIn("#inspector", command)
        self.assertNotIn("-i", command)
        self.assertNotIn("--interactive", command)

    def test_subprocess_errors_are_reported(self):
        with patch.object(browser.subprocess, "run", side_effect=OSError("missing executable")), \
             redirect_stderr(io.StringIO()) as error:
            self.assertEqual(self.invoke("close", "--build-id", "candidate-1", "--purpose", "layout"), 2)
        self.assertIn("agent-browser could not complete", error.getvalue())

    def test_close_targets_only_stable_owned_session(self):
        with patch.object(browser, "run_browser", return_value="closed") as run, patch("builtins.print"):
            self.assertEqual(self.invoke("close", "--build-id", "candidate-1", "--purpose", "layout"), 0)
        self.assertEqual(run.call_args.args[0],
                         ["--session", browser.owned_session("candidate-1", "layout"), "close"])
        self.assertNotEqual(browser.owned_session("candidate-1", "layout"),
                            browser.owned_session("candidate-1", "pcb"))

    def test_session_name_includes_checkout_identity(self):
        first = browser.owned_session("candidate-1", "journey")
        with patch.object(browser, "ROOT", Path("/separate/checkout")):
            second = browser.owned_session("candidate-1", "journey")
        self.assertNotEqual(first, second)

    def test_bounded_output_marks_truncation(self):
        self.assertEqual(browser.bounded("abcdefghij", 7), browser.TRUNCATION_MARKER[:7])
        shortened = browser.bounded("x" * 40, 24)
        self.assertTrue(shortened.endswith("...[output truncated]"))
        self.assertLessEqual(len(shortened), 24)


if __name__ == "__main__":
    unittest.main()
