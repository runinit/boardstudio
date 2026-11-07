#!/usr/bin/env python3
"""Focused tests for the update-adoption verifier's browser-session ownership.

Proof purpose (update-draft-safety reliability slice B): a verifier-owned browser
session must run on a known private profile, never attach to or reuse a user's
browser or profile because of inherited settings, and delete only a directory this
invocation owns, only after the browser is confirmed closed and no Chrome still
uses that profile. Unknown close or process state retains the directory and fails.

Seams (agreed in the A/B/C plan): the real session and cleanup helpers and the
`main` command outcome run against real disposable directories. Only the external
`agent-browser` and `ps` process boundary is stubbed, and the tests assert the
observable close-invocation counts and which directories actually remain.

Not covered here: the generated-output workspace and `--keep-artifacts` cleanup
(slice C) and any real-browser lifecycle (one integrated smoke after slice C).
"""

import importlib.util
import io
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location(
    "verify_update_adoption", Path(__file__).with_name("verify-update-adoption.py"))
VERIFY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(VERIFY)

URL = "http://127.0.0.1:4733/"


class FakeBrowserBoundary:
    """Stands in for the `agent-browser` CLI and `ps`; records what reached them."""

    def __init__(self):
        self.calls = []
        self.open_profiles = []
        self.close_output = "Browser closed\n"
        self.close_returncode = 0
        self.open_error = None
        self.live_profiles = []

    def run(self, args, **kwargs):
        assert args[0] == "agent-browser", f"unexpected process: {args}"
        self.calls.append(tuple(args))
        out = ""
        if args[1:3] == ["session", "id"]:
            out = "test-session\n"
        elif args[1] == "open":
            if self.open_error:
                raise self.open_error
            profile = (kwargs.get("env") or {}).get("AGENT_BROWSER_PROFILE")
            self.open_profiles.append(profile)
            if profile:  # what a real Chrome leaves behind in its profile
                (Path(profile) / "Default" / "IndexedDB").mkdir(parents=True)
        elif args[1] == "close":
            out = self.close_output
        code = self.close_returncode if args[1] == "close" else 0
        return subprocess.CompletedProcess(args, code, out, "")

    def ps(self, args, **kwargs):
        assert args == ["ps", "-eo", "pid,args"], f"unexpected process: {args}"
        return "PID COMMAND\n" + "".join(
            f"{100 + n} chrome --user-data-dir={path} --headless\n"
            for n, path in enumerate(self.live_profiles))

    def closes(self):
        return [call for call in self.calls if call[1] == "close"]


class JumpingClock:
    """Module-local clock so the 15 s live-process wait does not sleep."""

    def __init__(self):
        self.now = 0.0

    def monotonic(self):
        self.now += 20
        return self.now

    def sleep(self, seconds):
        pass


class SessionOwnershipTests(unittest.TestCase):
    def setUp(self):
        scratch = tempfile.TemporaryDirectory()
        self.addCleanup(scratch.cleanup)
        self.root = Path(scratch.name)
        # A user's own browser profile sitting next to anything the verifier makes.
        self.bystander = self.root / "bystander"
        (self.bystander / "profile" / "Default" / "IndexedDB").mkdir(parents=True)
        (self.bystander / "profile" / "Default" / "IndexedDB" / "user-data").write_text("keep")
        self.home = self.root / "home"
        self.home.mkdir()
        self.cli = FakeBrowserBoundary()
        self.log = io.StringIO()
        self.driver = VERIFY.Driver(4733, self.log)
        for target in (
                patch.object(VERIFY.subprocess, "run", self.cli.run),
                patch.object(VERIFY.subprocess, "check_output", self.cli.ps),
                patch.object(VERIFY, "time", JumpingClock()),
                patch.object(tempfile, "tempdir", str(self.root)),
                patch.dict(os.environ, {"HOME": str(self.home)}, clear=True)):
            target.start()
            self.addCleanup(target.stop)

    def made_directories(self):
        return sorted(path.name for path in self.root.iterdir()
                      if path.name.startswith(("boardstudio-verify-session-", "explicit")))

    def private_profile(self):
        profile = self.cli.open_profiles[0]
        self.assertIsNotNone(profile, "browser opened without a known profile")
        return Path(profile)

    def assert_bystander_kept(self):
        self.assertEqual(
            (self.bystander / "profile" / "Default" / "IndexedDB" / "user-data").read_text(), "keep")

    def test_profile_less_session_runs_on_a_private_profile_removed_after_confirmed_close(self):
        self.cli.live_profiles = [self.bystander / "profile"]  # user's Chrome, other profile
        VERIFY.start_session(self.driver, "pfx", URL)
        profile = self.private_profile()
        owned = profile.parent
        self.assertEqual(profile.name, "profile")
        self.assertEqual(owned.parent, self.root)
        self.assertTrue(owned.is_dir())

        VERIFY.end_session(self.driver)
        VERIFY.end_session(self.driver)

        self.assertEqual(len(self.cli.closes()), 1)
        self.assertFalse(owned.exists())
        self.assertEqual(self.driver.failures, [])
        self.assert_bystander_kept()

    def test_unconfirmed_close_retains_the_directory_and_fails(self):
        self.cli.close_output = "daemon gone\n"
        VERIFY.start_session(self.driver, "pfx", URL)
        owned = self.private_profile().parent

        VERIFY.end_session(self.driver)

        self.assertEqual(len(self.cli.closes()), 1)
        self.assertTrue((owned / "profile" / "Default" / "IndexedDB").is_dir())
        self.assertIn("session cleanup: browser close unconfirmed", self.driver.failures)
        self.assertIn("session cleanup: close unconfirmed; retain owned directory",
                      self.driver.failures)

    def test_retained_profile_cannot_be_abandoned_by_the_next_session(self):
        for reason in ("unconfirmed close", "live process"):
            with self.subTest(reason=reason):
                driver = VERIFY.Driver(4733, self.log)
                closes_before = len(self.cli.closes())
                self.cli.close_output = "daemon gone" if reason == "unconfirmed close" else "Browser closed"
                VERIFY.start_session(driver, "first", URL)
                profile = Path(self.cli.open_profiles[-1])
                self.cli.live_profiles = [profile] if reason == "live process" else []
                VERIFY.end_session(driver)
                VERIFY.end_session(driver)
                self.assertTrue(profile.exists())
                calls = list(self.cli.calls)
                with self.assertRaises(VERIFY.Failure):
                    VERIFY.start_session(driver, "next", URL)
                self.assertEqual(self.cli.calls, calls)
                self.assertEqual(driver.owned_profile, profile)
                self.assertEqual(driver.session_dir, profile.parent)
                self.assertEqual(driver.env["AGENT_BROWSER_PROFILE"], str(profile))
                self.assertEqual(len(self.cli.closes()) - closes_before, 1)

    def test_cleanup_without_an_owned_profile_never_deletes_a_directory(self):
        for phase in ("before start", "after cleanup"):
            with self.subTest(phase=phase):
                if phase == "after cleanup":
                    VERIFY.start_session(self.driver, "pfx", URL)
                    VERIFY.end_session(self.driver)
                unowned = self.root / phase
                unowned.mkdir()
                marker = unowned / "user-data"
                marker.write_text("keep")
                calls = list(self.cli.calls)
                VERIFY.cleanup_owned_profile(self.driver, unowned, "unowned")
                self.assertTrue(marker.is_file())
                self.assertEqual(self.cli.calls, calls)
                self.assertIn("unowned cleanup: directory not owned by this session; retain it",
                              self.driver.failures)

    def test_process_inspection_failure_retains_profile_and_records_failure(self):
        VERIFY.start_session(self.driver, "pfx", URL)
        profile = self.private_profile()
        with patch.object(VERIFY.subprocess, "check_output", side_effect=OSError("ps unavailable")):
            VERIFY.end_session(self.driver)
        self.assertTrue(profile.exists())
        self.assertIn("session cleanup: process state unknown; retain owned directory",
                      self.driver.failures)

    def test_deletion_failure_is_recorded_without_abandoning_the_profile(self):
        VERIFY.start_session(self.driver, "pfx", URL)
        profile = self.private_profile()
        with patch.object(VERIFY.shutil, "rmtree", side_effect=OSError("removal denied")):
            VERIFY.end_session(self.driver)
        self.assertTrue(profile.exists())
        self.assertEqual(self.driver.owned_profile, profile)
        self.assertIn("session cleanup: removal failed; retain owned directory",
                      self.driver.failures)

    def test_live_profile_with_spaces_in_its_path_is_not_deleted(self):
        scratch = self.root / "space in temp root"
        scratch.mkdir()
        with patch.object(tempfile, "tempdir", str(scratch)):
            VERIFY.start_session(self.driver, "pfx", URL)
        profile = self.private_profile()
        self.cli.live_profiles = [profile]
        VERIFY.end_session(self.driver)
        self.assertTrue(profile.exists())
        self.assertIn("session cleanup: profile still in use; retain owned directory",
                      self.driver.failures)

    def test_failed_close_exit_status_cannot_authorize_profile_deletion(self):
        self.cli.close_returncode = 1  # Even if stdout contains the success phrase.
        VERIFY.start_session(self.driver, "pfx", URL)
        profile = self.private_profile()
        VERIFY.end_session(self.driver)
        self.assertFalse(self.driver.session_closed)
        self.assertTrue(profile.exists())

    def test_chrome_still_using_the_profile_retains_the_directory_and_fails(self):
        VERIFY.start_session(self.driver, "pfx", URL)
        profile = self.private_profile()
        self.cli.live_profiles = [profile]

        VERIFY.end_session(self.driver)

        self.assertEqual(len(self.cli.closes()), 1)
        self.assertTrue((profile / "Default" / "IndexedDB").is_dir())
        self.assertIn("session cleanup: profile still in use; retain owned directory",
                      self.driver.failures)

    def test_setup_exception_closes_once_and_removes_only_this_sessions_directory(self):
        self.cli.open_error = subprocess.TimeoutExpired("agent-browser open", 120)
        with self.assertRaises(subprocess.TimeoutExpired):
            VERIFY.start_session(self.driver, "pfx", URL)
        self.assertEqual(len(self.made_directories()), 1)

        VERIFY.end_session(self.driver)

        self.assertEqual(len(self.cli.closes()), 1)
        self.assertEqual(self.made_directories(), [])
        self.assertEqual(self.driver.failures, [])
        self.assert_bystander_kept()

    def test_explicit_owned_profile_caller_keeps_its_own_directory_lifecycle(self):
        explicit = self.root / "explicit"
        explicit.mkdir()
        VERIFY.start_session(self.driver, "pfx", URL, explicit / "profile")
        self.assertEqual(self.cli.open_profiles, [str(explicit / "profile")])
        self.assertEqual(self.made_directories(), ["explicit"])

        VERIFY.cleanup_owned_profile(self.driver, explicit, "leg")

        self.assertEqual(len(self.cli.closes()), 1)
        self.assertEqual(self.made_directories(), [])
        self.assertEqual(self.driver.failures, [])

    def test_directory_this_session_never_used_is_never_deleted(self):
        VERIFY.start_session(self.driver, "pfx", URL)
        owned = self.private_profile().parent

        VERIFY.cleanup_owned_profile(self.driver, self.bystander, "leg")

        self.assertEqual(self.cli.closes(), [])
        self.assert_bystander_kept()
        self.assertTrue(owned.is_dir())
        self.assertIn("leg cleanup: directory not owned by this session; retain it",
                      self.driver.failures)

    def assert_refused_before_any_browser_command(self, raised, *named):
        message = str(raised)
        for name in named:
            self.assertIn(name, message)
        self.assertNotIn("SECRET", message)
        self.assertEqual(self.cli.calls, [])
        self.assertEqual(self.made_directories(), [])
        self.assertTrue(self.driver.session_closed)

    def test_inherited_connection_or_persistence_environment_is_refused_by_name_only(self):
        for name, value in (
                ("AGENT_BROWSER_CDP", "ws://SECRET:9222"),
                ("AGENT_BROWSER_AUTO_CONNECT", "SECRET"),
                ("AGENT_BROWSER_PROVIDER", "SECRET"),
                ("AGENT_BROWSER_RESTORE", "SECRET"),
                ("AGENT_BROWSER_SESSION_NAME", "SECRET"),
                ("AGENT_BROWSER_STATE", "/home/SECRET/auth.json"),
                ("AGENT_BROWSER_ARGS", "--no-sandbox,--user-data-dir=/home/SECRET/chrome"),
                ("AGENT_BROWSER_ARGS", "--remote-debugging-port=SECRET")):
            with self.subTest(name=name, value=value), patch.dict(os.environ, {name: value}):
                with self.assertRaises(VERIFY.Failure) as raised:
                    VERIFY.start_session(self.driver, "pfx", URL)
                self.assert_refused_before_any_browser_command(raised.exception, name)

    def test_unsafe_or_unreadable_config_files_are_refused_by_location_and_key_only(self):
        project = self.root / "project"
        project.mkdir()
        elsewhere = self.root / "elsewhere.json"
        for label, path, text, env, named in (
                ("user config", self.home / ".agent-browser" / "config.json",
                 '{"cdp": "SECRET"}', {}, ("config.json", "cdp")),
                ("project config", project / "agent-browser.json",
                 '{"autoConnect": "SECRET"}', {}, ("agent-browser.json", "autoConnect")),
                ("explicit config", elsewhere, '{"sessionName": "SECRET"}',
                 {"AGENT_BROWSER_CONFIG": str(elsewhere)}, ("elsewhere.json", "sessionName")),
                ("unreadable config", project / "agent-browser.json", "{ SECRET",
                 {}, ("agent-browser.json", "unreadable"))):
            with self.subTest(label), patch.object(VERIFY, "ROOT", project), \
                    patch.dict(os.environ, env):
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(text)
                with self.assertRaises(VERIFY.Failure) as raised:
                    VERIFY.start_session(self.driver, "pfx", URL)
                path.unlink()
                self.assert_refused_before_any_browser_command(raised.exception, *named)

    def test_explicit_config_replaces_default_files_and_inherited_profile_never_reaches_the_cli(self):
        # CLI contract (agent-browser 0.39.1): --config / AGENT_BROWSER_CONFIG replaces both
        # default config files; AGENT_BROWSER_PROFILE is replaced by the private profile.
        clean = self.root / "clean.json"
        clean.write_text('{"headed": false}')
        user_config = self.home / ".agent-browser" / "config.json"
        user_config.parent.mkdir()
        user_config.write_text('{"cdp": "9222"}')
        inherited = {"AGENT_BROWSER_CONFIG": str(clean), "AGENT_BROWSER_ARGS": "--no-sandbox",
                     "AGENT_BROWSER_PROFILE": str(self.bystander / "profile")}
        with patch.dict(os.environ, inherited):
            VERIFY.start_session(self.driver, "pfx", URL)
            profile = self.private_profile()
            VERIFY.end_session(self.driver)

        self.assertEqual(profile.parent.parent, self.root)
        self.assertNotEqual(profile, self.bystander / "profile")
        self.assertEqual(self.made_directories(), [])
        self.assert_bystander_kept()

    def test_main_refuses_unsafe_environment_with_exit_2_before_any_browser_work(self):
        out = io.StringIO()
        with patch.object(VERIFY.shutil, "which", lambda name: "/usr/bin/" + name), \
                patch.dict(os.environ, {"AGENT_BROWSER_CDP": "ws://SECRET:9222"}), \
                patch.object(VERIFY.sys, "stdout", out):
            code = VERIFY.main(["post"])

        self.assertEqual(code, 2)
        self.assertIn("AGENT_BROWSER_CDP", out.getvalue())
        self.assertNotIn("SECRET", out.getvalue())
        self.assertEqual(self.cli.calls, [])
        self.assertEqual(self.made_directories(), [])


if __name__ == "__main__":
    unittest.main()
