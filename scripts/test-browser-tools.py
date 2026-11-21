#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
from pathlib import Path
import unittest
from unittest import mock

SPEC = importlib.util.spec_from_file_location("browser_tools", Path(__file__).with_name("browser_tools.py"))
browser_tools = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(browser_tools)


class BrowserToolsTests(unittest.TestCase):
    def test_shallow_browser_tmpdir_uses_short_home_and_falls_back_for_deep_home(self):
        with mock.patch.object(browser_tools.Path, "home", return_value=Path("/tmp/bs-home")), \
                mock.patch.object(browser_tools.Path, "mkdir") as mkdir:
            self.assertEqual(browser_tools.shallow_browser_tmpdir(), Path("/tmp/bs-home/.cache/tmp"))
            mkdir.assert_called_once_with(parents=True, exist_ok=True)
        with mock.patch.object(browser_tools.Path, "home", return_value=Path("/tmp") / ("x" * 80)), \
                mock.patch.object(browser_tools.Path, "mkdir") as mkdir:
            self.assertEqual(browser_tools.shallow_browser_tmpdir(), Path("/tmp"))
            mkdir.assert_called_once_with(parents=True, exist_ok=True)

    def test_shallow_tmpdir_checks_encoded_byte_boundary(self):
        # '/tmp/' + 29 ASCII bytes + '/.cache/tmp' = 45 bytes.
        for home, expected in ((Path('/tmp') / ('x' * 29), True),
                               (Path('/tmp') / ('x' * 30), False),
                               (Path('/tmp') / ('é' * 15), False)):
            with self.subTest(home=home), mock.patch.object(browser_tools.Path, 'home', return_value=home), \
                    mock.patch.object(browser_tools.Path, 'mkdir'):
                self.assertEqual(browser_tools.shallow_browser_tmpdir(), home / '.cache/tmp' if expected else Path('/tmp'))

    def test_browser_env_overrides_tmpdir_and_preserves_base(self):
        base = {"TMPDIR": "/deep", "KEEP": "yes", "CHROME_BIN": "old"}
        with mock.patch.object(browser_tools, "shallow_browser_tmpdir", return_value=Path("/tmp")), \
                mock.patch.object(browser_tools, "locate_chrome", return_value="/chrome"):
            env = browser_tools.browser_env(base, EXTRA=42)
        self.assertEqual(env, {"TMPDIR": "/tmp", "KEEP": "yes", "CHROME_BIN": "/chrome", "EXTRA": "42"})
        self.assertEqual(base["TMPDIR"], "/deep")
        with mock.patch.object(browser_tools, "shallow_browser_tmpdir", return_value=Path("/tmp")):
            env = browser_tools.browser_env(base, chrome="/chosen", TMPDIR="/tmp/nested")
            self.assertEqual(env["TMPDIR"], "/tmp/nested")
            self.assertEqual(env["CHROME_BIN"], "/chosen")

    def test_cancelled_launcher_keeps_build_lock_until_child_exits(self):
        import os
        import select
        import subprocess
        import sys
        import tempfile

        root = Path(__file__).resolve().parents[1]
        with tempfile.TemporaryDirectory() as temp:
            home = Path(temp)
            lock = home / ".cache/tmp/boardstudio-build.lock"
            lock.parent.mkdir(parents=True)
            ready, release = home / "ready", home / "release"
            os.mkfifo(ready)
            os.mkfifo(release)
            cargo = home / "cargo"
            cargo.write_text("#!/usr/bin/python3\nimport os\n"
                             "if os.environ.pop('GRANDCHILD', ''):\n"
                             "    pid = os.fork()\n"
                             "    if pid: os.waitpid(pid, 0); os._exit(0)\n"
                             "    os.execv(__file__, [__file__])\n"
                             "with open(os.environ['READY'], 'w') as f: f.write(str(os.getppid()) + '\\n')\n"
                             "with open(os.environ['RELEASE']) as f: f.read(1)\n")
            cargo.chmod(0o755)
            (home / "wasm-pack").symlink_to(cargo)
            env = dict(os.environ, HOME=temp, READY=str(ready), RELEASE=str(release),
                       PATH=f"{temp}:{os.environ['PATH']}", PYTHONPATH=str(root / "scripts"))
            env.pop("BOARDSTUDIO_BUILD_LOCK_FD", None)
            env.pop("BOARDSTUDIO_WASM_TEST_COMMAND", None)
            env["CHROMEDRIVER"] = "/bin/true"
            ready_fd = os.open(ready, os.O_RDWR | os.O_NONBLOCK)
            release_fd = os.open(release, os.O_RDWR | os.O_NONBLOCK)
            try:
                for mode in ("serial", "shared-wasm-build", "parallel", "web", "cad", "wasm", "grandchild"):
                    with self.subTest(mode=mode):
                        setup = ("import fcntl, os, sys\n"
                                 "f = open(os.path.expanduser('~/.cache/tmp/boardstudio-build.lock'), 'w')\n"
                                 "fcntl.flock(f, fcntl.LOCK_EX)\n"
                                 "os.environ['BOARDSTUDIO_BUILD_LOCK_FD'] = str(f.fileno())\n")
                        calls = {
                            "serial": "import check; check.run_serial(['cargo'], dict(os.environ), captured=[])",
                            "shared-wasm-build": "import check; check.run_serial(check.WASM_CRATE_BUILD, dict(os.environ), outputs=[])",
                            "parallel": "import check; check.run_parallel([['cargo']], dict(os.environ))",
                            "grandchild": "import check; check.run_serial(['cargo'], dict(os.environ))",
                            "web": "import importlib; m=importlib.import_module('build-web'); m.run(['cargo'])",
                            "cad": "sys.path.insert(0, os.environ['CAD']); import cad_build; cad_build.run(['cargo'])",
                            "wasm": "import importlib; m=importlib.import_module('run-wasm-tests'); m.build_page_test(m.ROOT, dict(os.environ))",
                        }
                        source = setup + calls[mode]
                        error_log = tempfile.TemporaryFile(mode="w+")
                        process = subprocess.Popen([sys.executable, "-c", source],
                                                   env=dict(env, CAD=str(root / "cad/scripts"),
                                                            GRANDCHILD="1" if mode == "grandchild" else ""),
                                                   stdout=subprocess.DEVNULL, stderr=error_log)
                        child_ready = False
                        try:
                            child_ready = bool(select.select([ready_fd], [], [], 5)[0])
                            error_log.seek(0)
                            self.assertTrue(child_ready, f"build child did not start: {error_log.read()}")
                            build_parent = int(os.read(ready_fd, 32).strip())
                            process.kill()
                            process.wait(timeout=5)
                            if mode == "grandchild":
                                os.kill(build_parent, 9)
                            contender = ["flock", "-n", str(lock), "true"]
                            self.assertNotEqual(subprocess.run(contender).returncode, 0,
                                                "lock released while child is running")
                        finally:
                            if child_ready:
                                os.write(release_fd, b"X")
                            if process.poll() is None:
                                process.kill()
                                process.wait(timeout=5)
                            subprocess.run(["flock", str(lock), "true"], check=True, timeout=5)
                            error_log.close()
                        self.assertEqual(subprocess.run(contender).returncode, 0)
            finally:
                os.close(ready_fd)
                os.close(release_fd)

    def test_lookup_prefers_matching_pin_and_explicit_override(self):
        import tempfile

        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary)
            for kind in ("chrome", "chromedriver"):
                pinned = base / f"{kind}-linux64" / kind
                pinned.parent.mkdir()
                pinned.touch()
                pinned.chmod(0o700)
                locate = getattr(browser_tools, f"locate_{kind}")
                variable = "CHROME_BIN" if kind == "chrome" else "CHROMEDRIVER"
                env = {"BOARDSTUDIO_CFT_DIR": str(base), "PATH": ""}
                self.assertEqual(locate(env), str(pinned))
                self.assertEqual(locate(dict(env, **{variable: "/bin/true"})), str(Path("/bin/true").resolve()))
                with self.assertRaisesRegex(browser_tools.MissingBrowserTool, variable):
                    locate(dict(env, **{variable: "/missing"}))

    def test_async_and_action_commands_use_the_session_endpoints(self):
        session = browser_tools.WebDriverSession(chrome="/chrome", driver="/chromedriver")
        session.session_id = "control"
        with mock.patch.object(session, "_request") as request:
            session.execute_async("return", "arg", timeout=2)
            session.actions({"actions": [{"id": "mouse"}]})
        self.assertEqual(request.call_args_list, [
            mock.call("POST", "/session/control/timeouts", {"script": 2000}),
            mock.call("POST", "/session/control/execute/async", {"script": "return", "args": ["arg"]}),
            mock.call("POST", "/session/control/actions", {"actions": [{"id": "mouse"}]}),
        ])


if __name__ == "__main__":
    unittest.main()
