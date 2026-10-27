#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
from pathlib import Path
import unittest
from unittest import mock

SPEC = importlib.util.spec_from_file_location("browser_tools", Path(__file__).with_name("browser_tools.py"))
browser_tools = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(browser_tools)


def which(*available):
    return lambda name: f"/usr/bin/{name}" if name in available else None


class BrowserToolsTests(unittest.TestCase):
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
            (home / "scripts").mkdir()
            (home / "scripts/run-drag-browser.py").write_text("import browser_tools; browser_tools.build_drag_harness()\n")
            ready_fd = os.open(ready, os.O_RDWR | os.O_NONBLOCK)
            release_fd = os.open(release, os.O_RDWR | os.O_NONBLOCK)
            try:
                for mode in ("harness", "standalone", "nested", "serial", "parallel", "web", "cad", "wasm", "grandchild"):
                    with self.subTest(mode=mode):
                        setup = ("import fcntl, os, sys\n"
                                 "f = open(os.path.expanduser('~/.cache/tmp/boardstudio-build.lock'), 'w')\n"
                                 "fcntl.flock(f, fcntl.LOCK_EX)\n"
                                 "os.environ['BOARDSTUDIO_BUILD_LOCK_FD'] = str(f.fileno())\n")
                        calls = {
                            "harness": "import browser_tools; browser_tools.build_drag_harness()",
                            "standalone": "import browser_tools; browser_tools.build_drag_harness()",
                            "nested": "import check; from pathlib import Path; check.run_serial([sys.executable, 'scripts/run-drag-browser.py'], dict(os.environ), cwd=Path(os.environ['HOME']))",
                            "serial": "import check; check.run_serial(['cargo'], dict(os.environ), captured=[])",
                            "parallel": "import check; check.run_parallel([['cargo']], dict(os.environ))",
                            "grandchild": "import check; check.run_serial(['cargo'], dict(os.environ))",
                            "web": "import importlib; m=importlib.import_module('build-web'); m.run(['cargo'])",
                            "cad": "sys.path.insert(0, os.environ['CAD']); import cad_build; cad_build.run(['cargo'])",
                            "wasm": "import importlib; m=importlib.import_module('run-wasm-tests'); m.run_filter(None, dict(os.environ))",
                        }
                        # Standalone uses flock's own process; the others inherit a validated descriptor.
                        source = ("import os, sys\n" if mode == "standalone" else setup) + calls[mode]
                        process = subprocess.Popen([sys.executable, "-c", source],
                                                   env=dict(env, CAD=str(root / "cad/scripts"),
                                                            GRANDCHILD="1" if mode == "grandchild" else ""),
                                                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                        try:
                            self.assertTrue(select.select([ready_fd], [], [], 5)[0], "build child did not start")
                            build_parent = int(os.read(ready_fd, 32).strip())
                            process.kill()
                            process.wait(timeout=5)
                            if mode in ("standalone", "nested", "grandchild"):
                                os.kill(build_parent, 9)
                            contender = ["flock", "-n", str(lock), "true"]
                            self.assertNotEqual(subprocess.run(contender).returncode, 0,
                                                "lock released while child is running")
                        finally:
                            os.write(release_fd, b"X")
                            if process.poll() is None:
                                process.kill()
                                process.wait(timeout=5)
                            subprocess.run(["flock", str(lock), "true"], check=True, timeout=5)
                        self.assertEqual(subprocess.run(contender).returncode, 0)
            finally:
                os.close(ready_fd)
                os.close(release_fd)

    def test_harness_build_uses_cargo_artifact_and_reports_redacted_error(self):
        import json
        package = (Path(__file__).resolve().parents[1] / "tools/browser-harness").as_uri()
        def artifact(package_id, executable):
            return json.dumps({"reason": "compiler-artifact", "package_id": package_id,
                               "target": {"name": "drag-smoke", "kind": ["bin"]},
                               "executable": executable}) + "\n"
        artifacts = (artifact("path+file:///elsewhere#other@1.0.0", "/wrong") +
                     artifact(f"path+{package}#boardstudio-browser-harness@0.1.0", "/target/triple/debug/drag-smoke.exe"))
        with mock.patch.object(browser_tools.subprocess, "run", return_value=mock.Mock(
                returncode=0, stdout=artifacts, stderr="")):
            self.assertEqual(str(browser_tools.build_drag_harness()), "/target/triple/debug/drag-smoke.exe")
        with mock.patch.dict(browser_tools.os.environ, {"CARGO_REGISTRY_TOKEN": "secret-value"}), \
                mock.patch.object(browser_tools.subprocess, "run", return_value=mock.Mock(
                    returncode=101, stdout="", stderr="error: failed to compile; token secret-value")):
            with self.assertRaises(SystemExit) as error:
                browser_tools.build_drag_harness()
            self.assertIn("failed to compile", str(error.exception))
            self.assertNotIn("secret-value", str(error.exception))

    def test_build_diagnostics_hide_non_environment_secrets(self):
        import json
        cases = (
            ('ssh://alice:synthetic-ssh-credential@example.invalid/path?session=synthetic-query#synthetic-fragment',
             ("alice", "synthetic-ssh-credential", "synthetic-query", "synthetic-fragment")),
            ('HTTPS://bob:synthetic-upper-credential@example.invalid/path#synthetic-anchor',
             ("bob", "synthetic-upper-credential", "synthetic-anchor")),
            ('git+ssh://carol:synthetic-git-credential@example.invalid/repo',
             ("carol", "synthetic-git-credential")),
            ('{"outer": {"token": "quoted-secret", "nested": {"password": "nested-secret"}}}',
             ("quoted-secret", "nested-secret")),
            ('Authorization: Bearer bearer-secret', ("bearer-secret",)),
            ('{"Authorization": "Bearer quoted-bearer"}', ("quoted-bearer",)),
            ('Bearer standalone-bearer', ("standalone-bearer",)),
            ('{"token": {\n  "value": "multiline-secret"\n}}', ("multiline-secret",)),
        )
        for diagnostic, secrets in cases:
            for rendered in (False, True):
                with self.subTest(rendered=rendered, case=cases.index((diagnostic, secrets))):
                    message = json.dumps({"reason": "compiler-message", "message": {
                        "rendered": "error: registry request failed\n" + diagnostic}})
                    with mock.patch.dict(browser_tools.os.environ, {}, clear=True), \
                            mock.patch.object(browser_tools.subprocess, "run", return_value=mock.Mock(
                                returncode=101, stdout=message if rendered else "",
                                stderr="" if rendered else "error: registry request failed\n" + diagnostic)):
                        with self.assertRaises(SystemExit) as error:
                            browser_tools.build_drag_harness()
                    output = str(error.exception)
                    self.assertIn("registry request failed", output)
                    for secret in secrets:
                        self.assertFalse(secret in output, "sensitive value reached build diagnostics")

    def test_build_lock_across_scheduler_and_standalone(self):
        import fcntl
        import os
        import subprocess
        import sys
        import tempfile

        root = Path(__file__).resolve().parents[1]
        with tempfile.TemporaryDirectory() as temp:
            home = Path(temp)
            lock = home / ".cache/tmp/boardstudio-build.lock"
            lock.parent.mkdir(parents=True)
            cargo = home / "cargo"
            cargo.write_text("#!/usr/bin/python3\n" +
                "import json, os, subprocess\n" +
                "assert subprocess.run(['flock', '-n', os.environ['LOCK'], 'true']).returncode != 0\n" +
                "print(json.dumps(" + repr({"reason": "compiler-artifact",
                    "package_id": f"path+{(root / 'tools/browser-harness').as_uri()}#boardstudio-browser-harness@0.1.0",
                    "target": {"name": "drag-smoke", "kind": ["bin"]}, "executable": "/fake/drag-smoke"}) + "))\n")
            cargo.chmod(0o755)
            env = dict(os.environ, HOME=temp, LOCK=str(lock), PATH=f"{temp}:{os.environ['PATH']}", PYTHONPATH=str(root / "scripts"))
            direct = "import browser_tools; print(browser_tools.build_drag_harness())"
            standalone = subprocess.run([sys.executable, "-c", direct], cwd=root / "scripts", env=env,
                                        capture_output=True, text=True, timeout=5)
            self.assertEqual(standalone.returncode, 0, standalone.stderr)
            with lock.open("w") as held:
                fcntl.flock(held, fcntl.LOCK_EX)
                env["BOARDSTUDIO_BUILD_LOCK_FD"] = str(held.fileno())
                # Use the scheduler's actual drag command path and descriptor handling.
                (home / "scripts").mkdir()
                (home / "scripts/run-drag-browser.py").write_text(direct)
                nested = "import check, os, sys; from pathlib import Path; sys.exit(check.run_serial([sys.executable, 'scripts/run-drag-browser.py'], dict(os.environ), cwd=Path(os.environ['HOME']))[1])"
                result = subprocess.run([sys.executable, "-c", nested], cwd=root / "scripts", env=env,
                                        pass_fds=(held.fileno(),), capture_output=True, text=True, timeout=5)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIn("/fake/drag-smoke", result.stdout)
                with (home / "wrong").open("w") as wrong, lock.open("w") as unowned:
                    for fd in (wrong.fileno(), unowned.fileno()):
                        invalid_env = dict(env, BOARDSTUDIO_BUILD_LOCK_FD=str(fd))
                        invalid = subprocess.run([sys.executable, "-c", direct], cwd=root / "scripts",
                                                 env=invalid_env, pass_fds=(fd,), capture_output=True, text=True,
                                                 timeout=5)
                        self.assertNotEqual(invalid.returncode, 0)
                        self.assertIn("invalid BOARDSTUDIO_BUILD_LOCK_FD", invalid.stderr)
                standalone_env = dict(env)
                standalone_env.pop("BOARDSTUDIO_BUILD_LOCK_FD")
                blocked = subprocess.Popen([sys.executable, "-c", direct], cwd=root / "scripts",
                                           env=standalone_env, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                           text=True)
                try:
                    with self.assertRaises(subprocess.TimeoutExpired):
                        blocked.communicate(timeout=0.5)
                    fcntl.flock(held, fcntl.LOCK_UN)
                    stdout, stderr = blocked.communicate(timeout=5)
                    self.assertEqual(blocked.returncode, 0, stderr)
                    self.assertIn("/fake/drag-smoke", stdout)
                finally:
                    if blocked.poll() is None:
                        blocked.kill()
                        blocked.communicate()

    def test_missing_flock_is_actionable(self):
        import subprocess
        import sys
        result = subprocess.run([sys.executable, "-c", "import browser_tools; browser_tools.build_drag_harness()"],
                                cwd=Path(__file__).parent, env={"PATH": ""}, capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("flock", result.stderr)
        self.assertNotIn("FileNotFoundError", result.stderr)
        self.assertIn("install", result.stderr.lower())

    def locate(self, *available, playwright=0, env=None):
        with mock.patch.object(browser_tools.shutil, "which", side_effect=which(*available)), \
                mock.patch.object(browser_tools.subprocess, "run",
                                  return_value=mock.Mock(returncode=playwright)) as run, \
                mock.patch.dict(browser_tools.os.environ, env or {}, clear=True):
            return browser_tools.locate(), run

    def test_all_tools_present_returns_node_and_chrome(self):
        (node, chrome), run = self.locate("node", "chromium")
        self.assertEqual((node, chrome), ("/usr/bin/node", "/usr/bin/chromium"))
        self.assertEqual(run.call_args.args[0], ["/usr/bin/node", "-e", "require('playwright-core')"])

    def test_chrome_bin_wins_over_a_discovered_browser(self):
        (_, chrome), _ = self.locate("node", "chromium", env={"CHROME_BIN": "/opt/chrome"})
        self.assertEqual(chrome, "/opt/chrome")

    def test_each_absent_tool_is_named(self):
        cases = (
            (("chromium",), 0, "Node.js"),
            (("node", "chromium"), 1, "playwright-core"),
            (("node",), 0, "Chrome or Chromium"),
        )
        for available, playwright, expected in cases:
            with self.subTest(expected=expected), self.assertRaises(browser_tools.MissingBrowserTool) as raised:
                self.locate(*available, playwright=playwright)
            self.assertIn(expected, str(raised.exception))

    def test_missing_playwright_names_the_manual_prerequisite_not_ci(self):
        with self.assertRaises(browser_tools.MissingBrowserTool) as raised:
            self.locate("node", "chromium", playwright=1)
        message = str(raised.exception)
        self.assertIn("NODE_PATH", message)
        self.assertIn("install", message.lower())
        self.assertNotIn("CI web-tools action installs it", message)

    def locate_chrome(self, *available, env=None):
        with mock.patch.object(browser_tools.shutil, "which", side_effect=which(*available)), \
                mock.patch.dict(browser_tools.os.environ, env or {}, clear=True):
            return browser_tools.locate_chrome()

    def test_chrome_only_probe_needs_no_node(self):
        self.assertEqual(self.locate_chrome("chromium"), "/usr/bin/chromium")

    def test_chrome_only_probe_honours_chrome_bin(self):
        self.assertEqual(self.locate_chrome(env={"CHROME_BIN": "/opt/chrome"}), "/opt/chrome")

    def test_drag_browser_override_and_chrome_bin_precedence(self):
        self.assertEqual(self.locate_chrome(env={"BOARDSTUDIO_DRAG_CHROME_BIN": "/drag/chrome"}), "/drag/chrome")
        self.assertEqual(self.locate_chrome(env={"CHROME_BIN": "/chosen/chrome",
                                                 "BOARDSTUDIO_DRAG_CHROME_BIN": "/drag/chrome"}), "/chosen/chrome")

    def test_chrome_only_probe_names_chrome_bin(self):
        with self.assertRaises(browser_tools.MissingBrowserTool) as raised:
            self.locate_chrome()
        self.assertIn("CHROME_BIN", str(raised.exception))


if __name__ == "__main__":
    unittest.main()
