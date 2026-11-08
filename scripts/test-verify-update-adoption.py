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

Proof purpose (slice C): every generated output (stamped A/B builds, worker copies,
broken variants, leg fixtures and exports) lives in a fresh directory this invocation
created, which is refused before any write when it already exists, is a symlink or
overlaps an input, and is removed on completion and on caught failures only once the
browser is confirmed closed. Unconfirmed or failed cleanup retains and prints the owned
paths and fails; `--keep-artifacts` retains outputs for diagnosis and never exits 0.
SIGKILL recovery is deliberately not covered.

Not covered here: any real-browser lifecycle (one integrated smoke after slice C).
"""

import importlib.util
import io
import json
import os
from pathlib import Path
import re
import socket
import subprocess
import tempfile
import unittest
import zipfile
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
        self.eval_error = None
        self.download_document = None
        self.session_id = "test-session\n"
        self.live_profiles = []

    def run(self, args, **kwargs):
        if args[0] == "curl":  # the owned server's release probe (serve_version)
            return subprocess.CompletedProcess(args, 0, 'start_offline_worker("boardstudio-0123456789abcdef-1"', "")
        assert args[0] == "agent-browser", f"unexpected process: {args}"
        self.calls.append(tuple(args))
        out = ""
        if args[1] == "snapshot" and self.download_document is not None:
            out = '\n'.join(f'button "{name}" [ref=e{i}]' for i, name in enumerate(
                ("Export", "Download .boardstudio project", "Back to Layout"), 1))
        elif args[1] == "download" and self.download_document is not None:
            with zipfile.ZipFile(args[3], "w") as bundle:
                bundle.writestr("project.json", json.dumps(self.download_document))
        elif args[1:3] == ["session", "id"]:
            out = self.session_id
        elif args[1] == "eval" and self.eval_error:
            raise self.eval_error
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

    time = monotonic  # the owned-server wait polls wall time

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


class GeometryExportOwnershipTests(unittest.TestCase):
    setUp = SessionOwnershipTests.setUp
    assert_bystander_kept = SessionOwnershipTests.assert_bystander_kept

    def test_geometry_export_survives_until_confirmed_close_and_diagnostic_retention(self):
        self.cli.download_document = dict(id="export-project", revision=17,
                                         scripts=[dict(name=VERIFY.GEO_BASE_NAME, source="user-source")])
        for keep in (False, True):
            with self.subTest(keep=keep):
                self.driver.keep_artifacts = keep
                VERIFY.start_session(self.driver, "export", URL)
                directory = self.driver.session_dir
                VERIFY.check_reopened_geometry_save(
                    self.driver, "owned-export", dict(document_id="export-project", revision=17),
                    "user-source")
                archive = directory / "owned-export.boardstudio"
                self.assertTrue(archive.is_file(), "export must remain owned until browser close")
                VERIFY.end_session(self.driver)
                self.assertEqual(archive.exists(), keep)
                self.assertEqual(self.driver.failures, [])
                self.assert_bystander_kept()


class FakeServerProcess:
    def terminate(self):
        pass

    def wait(self, timeout=None):
        return 0

    def kill(self):
        pass


class FakeBuildWeb:
    """build-web.py's embed step: each call re-embeds a distinct release version."""

    def __init__(self):
        self.embedded = 0

    def embed_offline_worker(self, site, worker):
        self.embedded += 1
        path = Path(site) / "service-worker.js"
        path.write_text(re.sub(r"boardstudio-[0-9a-f]{16}-[0-9]+",
                               f"boardstudio-{self.embedded:016x}-{self.embedded}", path.read_text()))


def write_packaged_source(source):
    """Smallest `build-web.py --routes root,subpath` shape the stamping reads."""
    for scope in ("site-root", "site-subpath/boardstudio"):
        site = source / scope
        (site / "wasm").mkdir(parents=True)
        (site / "index.html").write_text("<html><head></head><body></body></html>")
        (site / "wasm" / "boardstudio-web_bg.wasm").write_bytes(b"\0asm\1\0\0\0")
        (site / "boardstudio_offline_worker.js").write_text("// worker glue")
        (site / "service-worker.js").write_text(
            'start_offline_worker("boardstudio-0000000000000000-0", []);\natob("QUJD");\n')


def tree(root):
    """Every entry under root, symlinks not followed: the independent no-write oracle."""
    entries = {}
    for base, names, files in os.walk(root):
        for name in names + files:
            path = Path(base) / name
            relative = path.relative_to(root).as_posix()
            if path.is_symlink():
                entries[relative] = ("link", os.readlink(path))
            else:
                entries[relative] = ("file", path.read_bytes()) if path.is_file() else ("dir",)
    return entries


class PageWasmResolutionTests(unittest.TestCase):
    def test_page_wasm_resolves_both_packaging_layouts_and_fails_closed(self):
        # write_packaged_source only builds the fast two-route shape, so the layout
        # variants this resolver must distinguish are built inline as single sites.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            fast = root / "fast-site"
            (fast / "wasm").mkdir(parents=True)
            (fast / "wasm" / "boardstudio-web_bg.wasm").write_bytes(b"\0asm\1\0\0\0")
            optimized = root / "optimized-site"
            (optimized / "assets").mkdir(parents=True)
            hashed = optimized / "assets" / "boardstudio-web_bg-dxhTEST.wasm"
            hashed.write_bytes(b"\0asm\1\0\0\0")
            (optimized / "assets" / "boardstudio-web-dxhTEST.js").write_text(
                "// hashed JS references the hashed wasm by URL")
            both = root / "both-site"
            (both / "wasm").mkdir(parents=True)
            (both / "wasm" / "boardstudio-web_bg.wasm").write_bytes(b"\0asm\1\0\0\0")
            (both / "assets").mkdir()
            (both / "assets" / "boardstudio-web_bg-dxhTEST.wasm").write_bytes(b"\0asm\1\0\0\0")
            empty = root / "empty-site"
            empty.mkdir()
            multiple = root / "multiple-site"
            (multiple / "assets").mkdir(parents=True)
            (multiple / "assets" / "boardstudio-web_bg-dxhAAA.wasm").write_bytes(b"\0asm")
            (multiple / "assets" / "boardstudio-web_bg-dxhBBB.wasm").write_bytes(b"\0asm")
            with self.subTest("fast layout keeps the historical path"):
                self.assertEqual(VERIFY.page_wasm(fast),
                                 fast / "wasm" / "boardstudio-web_bg.wasm")
            with self.subTest("optimized layout resolves the hashed binary and stamps in place"):
                self.assertEqual(VERIFY.page_wasm(optimized), hashed)
                VERIFY.stamp_wasm(hashed, "B")
                self.assertTrue(hashed.read_bytes().endswith(b"boardstudio-buildB"))
                self.assertEqual(hashed.name, "boardstudio-web_bg-dxhTEST.wasm")
            with self.subTest("fast layout wins when both exist"):
                self.assertEqual(VERIFY.page_wasm(both),
                                 both / "wasm" / "boardstudio-web_bg.wasm")
            with self.subTest("zero matches fails closed naming the site"):
                with self.assertRaises(VERIFY.Failure) as ctx:
                    VERIFY.page_wasm(empty)
                self.assertIn("empty-site", str(ctx.exception))
            with self.subTest("multiple matches fail closed naming the candidates"):
                with self.assertRaises(VERIFY.Failure) as ctx:
                    VERIFY.page_wasm(multiple)
                message = str(ctx.exception)
                self.assertIn("multiple-site", message)
                self.assertIn("dxhAAA", message)
                self.assertIn("dxhBBB", message)


class LoadedBinaryPayloadTests(unittest.TestCase):
    def test_check_loaded_binary_fetches_each_layout_page_wasm(self):
        # The loaded-binary payload must fetch the site's actual page wasm: the
        # historical fast path, or the hashed optimized binary. The browser
        # transport (eval_file) is captured, never executed; the production
        # derive-and-render path (page_wasm + loaded_binary_script) runs for real.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            fast = root / "fast-site"
            (fast / "wasm").mkdir(parents=True)
            (fast / "wasm" / "boardstudio-web_bg.wasm").write_bytes(b"\0asm\1\0\0\0")
            optimized = root / "optimized-site"
            (optimized / "assets").mkdir(parents=True)
            (optimized / "assets" / "boardstudio-web_bg-dxhTEST.wasm").write_bytes(
                b"\0asm\1\0\0\0")
            for layout, site, wasm_rel in (
                    ("fast", fast, "wasm/boardstudio-web_bg.wasm"),
                    ("optimized", optimized, "assets/boardstudio-web_bg-dxhTEST.wasm")):
                with self.subTest(layout):
                    rendered = {}
                    driver = VERIFY.Driver(4733, io.StringIO())

                    def capture(payload, text=None):
                        rendered["payload"] = payload
                        rendered["text"] = text
                        return {"wasmBuild": "B", "requestedByThisDocument": True,
                                "url": "http://127.0.0.1:4733/" + wasm_rel,
                                "status": 200, "size": 8}

                    driver.eval_file = capture
                    VERIFY.check_loaded_binary(driver, layout, "B", site)
                    self.assertEqual(rendered["payload"], "loaded-binary.js")
                    self.assertIn(f"reg.scope + '{wasm_rel}'", rendered["text"])
                    self.assertNotIn(VERIFY.PAGE_WASM_PLACEHOLDER, rendered["text"])
                    self.assertEqual(driver.failures, [])
            with self.subTest("unresolvable site fails closed before any browser fetch"):
                empty = root / "empty-site"
                empty.mkdir()
                calls = []
                driver = VERIFY.Driver(4733, io.StringIO())
                driver.eval_file = lambda *args, **kwargs: calls.append((args, kwargs))
                with self.assertRaises(VERIFY.Failure):
                    VERIFY.check_loaded_binary(driver, "empty", "B", empty)
                self.assertEqual(calls, [])


class PackagedDiffTests(unittest.TestCase):
    def test_packaged_diff_accepts_optimized_layout_sort_order(self):
        # write_packaged_source only builds the fast two-route shape, so the
        # optimized A/B pair this order check must accept is built inline.
        # Only index.html, service-worker.js and the hashed page wasm differ;
        # the hashed assets/ path sorts first, unlike the fast wasm/ path.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            builds = {}
            for label, name, version in (
                    ("A", "a", "boardstudio-000000000000000a-1"),
                    ("B", "b", "boardstudio-000000000000000b-2")):
                build = root / name
                builds[label] = build
                for scope in ("site-root", "site-subpath/boardstudio"):
                    site = build / scope
                    (site / "assets").mkdir(parents=True)
                    (site / "index.html").write_text(
                        f"<html><head></head><body>{label}</body></html>")
                    (site / "service-worker.js").write_text(
                        f'start_offline_worker("{version}", []);\n')
                    (site / "assets" / "boardstudio-web_bg-dxhTEST.wasm").write_bytes(
                        b"\0asm\1\0\0\0" + label.encode())
                    (site / "boardstudio_offline_worker.js").write_text("// worker glue")
                    (site / "assets" / "boardstudio-web-dxhTEST.js").write_text(
                        "// hashed JS references the hashed wasm by URL")
            driver = VERIFY.Driver(4733, io.StringIO())
            VERIFY.packaged_diff(builds["A"], builds["B"], io.StringIO(), driver)
            self.assertEqual(driver.failures, [])


class OutputWorkspaceTests(unittest.TestCase):
    """`main post` against a stub browser, build and server boundary on real directories."""

    def setUp(self):
        SessionOwnershipTests.setUp(self)  # same boundary, bystander profile and temp root
        self.created = []  # every mkdtemp this run asked for, wherever it landed
        real_mkdtemp = tempfile.mkdtemp

        def mkdtemp(prefix=None, dir=None, **kwargs):
            path = real_mkdtemp(prefix=prefix, dir=self.root if dir in (None, "/tmp") else dir)
            self.created.append(Path(path))
            return path

        self.served = []
        self.build_web = FakeBuildWeb()
        self.repo = self.root / "repo"
        (self.repo / "web" / "target").mkdir(parents=True)
        (self.repo / ".evidence").mkdir()
        self.source = self.root / "source"
        write_packaged_source(self.source)
        self.legacy = self.root / "legacy-b"
        self.legacy.mkdir()
        (self.legacy / "package").write_text("immutable")
        self.serve_log = self.root / "serve.log"
        self.serve_log.write_text("earlier log")
        self.work = self.root / "work"
        with socket.socket() as sock:
            sock.bind(("127.0.0.1", 0))
            self.port = sock.getsockname()[1]
        for target in (
                patch.object(tempfile, "mkdtemp", mkdtemp),
                patch.object(VERIFY, "ROOT", self.repo),
                patch.object(VERIFY, "load_build_web", lambda: self.build_web),
                patch.object(VERIFY.subprocess, "Popen", self.popen),
                patch.object(VERIFY.shutil, "which", lambda name: "/usr/bin/" + name)):
            target.start()
            self.addCleanup(target.stop)

    assert_bystander_kept = SessionOwnershipTests.assert_bystander_kept

    def popen(self, args, **kwargs):
        build = Path(args[2])  # serve-web.py <build dir> <port>
        self.served.append((build, build.is_dir()))
        return FakeServerProcess()

    def surviving(self):
        """Directories this run created that still exist."""
        return [path for path in self.created if path.exists()]

    def run_main(self, work_dir="work", *extra, legs=""):
        argv = ["post", "--port", str(self.port), "--src-dir", str(self.source),
                "--legacy-b-dir", str(self.legacy), "--serve-log", str(self.serve_log),
                "--legs", legs, *extra]
        if work_dir is not None:
            argv += ["--work-dir", str(self.root / work_dir)]
        out = io.StringIO()
        with patch.object(VERIFY.sys, "stdout", out):
            code = VERIFY.main(argv)
        return code, out.getvalue()

    def assert_workspace_refused(self, work_dir, reason):
        before = tree(self.root)
        code, output = self.run_main(work_dir)
        self.assertEqual(code, 2, output)
        self.assertIn(reason, output)
        self.assertEqual(tree(self.root), before, "refusal wrote or deleted something")
        self.assertEqual((self.created, self.served, self.cli.calls), ([], [], []))

    def test_completed_run_removes_every_generated_output_before_reporting_acceptance(self):
        protected = {path: tree(path) for path in (self.source, self.legacy, self.bystander)}
        for name in ("work", None):  # explicit new path, and the default fresh location
            with self.subTest(name):
                self.served.clear()
                code, output = self.run_main(name)

                self.assertEqual(code, 0, output)
                self.assertEqual([built for _, built in self.served], [True],
                                 "the server must have served a generated build")
                self.assertFalse(self.work.exists())
                self.assertEqual(self.surviving(), [])
                self.assertEqual(list((self.repo / "web" / "target").iterdir()), [])
                self.assertEqual({path: tree(path) for path in protected}, protected)
                self.assertEqual(self.serve_log.exists(), True)
                self.assertLess(output.index("workspace owned fixture/profile/database removed"),
                                output.index("ACCEPTANCE PASS"))

    # A leg with no scripted page state aborts in set_marker (eval output is not JSON):
    # the real leg code, real start_session and real cleanup run against the stub CLI.
    def test_caught_leg_failure_removes_workspace_and_leg_directory_after_confirmed_close(self):
        protected = {path: tree(path) for path in (self.source, self.legacy, self.bystander)}

        code, output = self.run_main("work", legs="takeover-draft")

        self.assertEqual({path: tree(path) for path in protected}, protected)
        self.assertEqual(code, 1, output)
        self.assertIn("FAILED: leg takeover-draft aborted", output)
        self.assertNotIn("ACCEPTANCE PASS", output)
        self.assertEqual(len(self.cli.closes()), 1)
        self.assertFalse(self.work.exists())
        self.assertEqual(self.surviving(), [])
        self.assert_bystander_kept()

    def test_uncaught_exception_still_removes_workspace_and_leg_directory(self):
        self.cli.eval_error = subprocess.TimeoutExpired("agent-browser eval", 180)

        with self.assertRaises(subprocess.TimeoutExpired):
            self.run_main("work", legs="takeover-draft")

        self.assertEqual(len(self.cli.closes()), 1)
        self.assertFalse(self.work.exists())
        self.assertEqual(self.surviving(), [])

    def test_leg_directory_is_removed_when_its_session_never_started(self):
        self.cli.session_id = ""  # new_session refuses before any browser is opened
        for leg in ("layout-constraint", "takeover-draft"):
            with self.subTest(leg):
                code, output = self.run_main("work", legs=leg)

                self.assertEqual(code, 1, output)
                self.assertIn(f"FAILED: leg {leg} aborted", output)
                self.assertEqual(self.cli.closes(), [])
                self.assertFalse(self.work.exists())
                self.assertEqual(self.surviving(), [])
                self.assertNotIn("not owned", output)

    def test_leg_directory_with_fixtures_is_removed_on_success_and_never_blocks_the_next_session(self):
        directory = VERIFY.make_session_dir(self.driver, "boardstudio-010-layout-")
        (directory / "layout.boardstudio").write_text("fixture")
        VERIFY.start_session(self.driver, "pfx", URL, directory / "profile")
        (directory / "layout-A.boardstudio").write_text("export")

        VERIFY.cleanup_owned_profile(self.driver, directory, "layout")

        self.assertFalse(directory.exists())
        self.assertEqual(len(self.cli.closes()), 1)
        self.assertEqual(self.driver.failures, [])
        self.assert_bystander_kept()
        VERIFY.start_session(self.driver, "next", URL)  # nothing left unresolved
        VERIFY.end_session(self.driver)
        self.assertEqual(self.surviving(), [])

    def test_leg_directory_is_not_made_while_an_earlier_session_is_unresolved(self):
        self.cli.close_output = "daemon gone\n"
        VERIFY.start_session(self.driver, "first", URL)
        VERIFY.end_session(self.driver)
        retained = self.created[0]
        made = list(self.created)

        with self.assertRaises(VERIFY.Failure):
            VERIFY.make_session_dir(self.driver, "boardstudio-010-layout-")

        self.assertEqual(self.created, made)
        self.assertEqual(self.driver.session_dir, retained)
        self.assertTrue((retained / "profile" / "Default" / "IndexedDB").is_dir())

    def test_unconfirmed_close_retains_and_prints_workspace_and_leg_directory(self):
        self.cli.close_output = "daemon gone\n"

        code, output = self.run_main("work", legs="takeover-draft")

        leg_directory = self.created[0]
        self.assertEqual(code, 1, output)
        self.assertEqual(len(self.cli.closes()), 1)
        self.assertEqual(self.surviving(), [leg_directory])
        self.assertTrue((self.work / "a").is_dir())
        self.assertNotIn("ACCEPTANCE PASS", output)
        self.assertIn(f"RETAINED (owned output, remove after inspection): {leg_directory}", output)
        self.assertIn(f"RETAINED (owned output, remove after inspection): {self.work}", output)

    def test_workspace_removal_failure_retains_it_and_fails(self):
        real_rmtree = VERIFY.shutil.rmtree

        def rmtree(path, *args, **kwargs):
            if Path(path) == self.work:
                raise OSError("removal denied")
            real_rmtree(path, *args, **kwargs)

        with patch.object(VERIFY.shutil, "rmtree", rmtree):
            code, output = self.run_main("work")

        self.assertEqual(code, 1, output)
        self.assertTrue((self.work / "a").is_dir())
        self.assertIn("FAILED: workspace cleanup: removal failed; retain owned directory", output)
        self.assertNotIn("ACCEPTANCE PASS", output)
        self.assertIn(f"RETAINED (owned output, remove after inspection): {self.work}", output)

    def test_keep_artifacts_retains_outputs_but_is_never_an_acceptance_pass(self):
        code, output = self.run_main("work", "--keep-artifacts")

        self.assertEqual(code, 3, output)  # distinct from 0 (accepted) and 1 (failed)
        self.assertNotIn("ACCEPTANCE PASS", output)
        self.assertNotIn("FAILED:", output)
        self.assertNotIn("RETAINED", output)  # kept on purpose, not left behind by a failure
        self.assertTrue((self.work / "a").is_dir() and (self.work / "b").is_dir())
        self.assertIn(f"DIAGNOSTIC ONLY (--keep-artifacts, not an acceptance run): {self.work}", output)
        self.assertIn("NOT ACCEPTANCE", output)

    def test_keep_artifacts_keeps_leg_directory_only_after_confirmed_close(self):
        for confirmed in (True, False):
            with self.subTest(confirmed=confirmed):
                self.created.clear()
                self.cli.calls.clear()
                self.cli.close_output = "Browser closed\n" if confirmed else "daemon gone\n"

                code, output = self.run_main(f"work-{confirmed}", "--keep-artifacts",
                                             legs="takeover-draft")

                leg_directory = self.created[0]
                self.assertEqual(code, 1, output)
                self.assertEqual(len(self.cli.closes()), 1)
                self.assertTrue(leg_directory.is_dir())
                self.assertNotIn("ACCEPTANCE PASS", output)
                kept = f"DIAGNOSTIC ONLY (--keep-artifacts, not an acceptance run): {leg_directory}"
                retained = f"RETAINED (owned output, remove after inspection): {leg_directory}"
                self.assertEqual((kept in output, retained in output), (confirmed, not confirmed))

    def test_existing_output_workspace_is_refused_without_touching_it(self):
        (self.root / "empty").mkdir()
        (self.root / "stale").mkdir()
        (self.root / "stale" / "a").mkdir()
        (self.root / "stale" / "a" / "old-build").write_text("must survive")
        (self.root / "plain-file").write_text("must survive")
        for name in ("empty", "stale", "plain-file"):
            with self.subTest(name):
                self.assert_workspace_refused(name, "already exists")

    def test_symlinked_output_workspace_is_refused_without_following_it(self):
        (self.root / "alias").symlink_to(self.bystander, target_is_directory=True)
        (self.root / "dangling").symlink_to(self.root / "nowhere")
        for name in ("alias", "dangling"):
            with self.subTest(name):
                self.assert_workspace_refused(name, "symbolic link")

    def test_output_workspace_inside_an_input_or_its_alias_is_refused(self):
        (self.root / "alias").symlink_to(self.source, target_is_directory=True)
        for name in ("source/work", "legacy-b/work", "alias/work", "repo/.evidence/work"):
            with self.subTest(name):
                self.assert_workspace_refused(name, "overlap")

    def test_unusable_output_workspace_path_is_refused(self):
        (self.root / "plain-file").write_text("not a directory")
        for name, reason in (("missing/work", "parent"), ("plain-file/work", "parent"),
                             ("bystander/../work", "'..'"), (None, "parent")):
            with self.subTest(name):
                if name is None:  # default location under a repo with no web/target
                    (self.repo / "web" / "target").rmdir()
                self.assert_workspace_refused(name, reason)


if __name__ == "__main__":
    unittest.main()
