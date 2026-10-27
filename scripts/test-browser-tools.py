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
