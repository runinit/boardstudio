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
        artifact = '{"reason":"compiler-artifact","target":{"name":"drag-smoke","kind":["bin"]},"executable":"/target/triple/debug/drag-smoke.exe"}\n'
        with mock.patch.object(browser_tools.subprocess, "run", return_value=mock.Mock(
                returncode=0, stdout=artifact, stderr="")):
            self.assertEqual(str(browser_tools.build_drag_harness()), "/target/triple/debug/drag-smoke.exe")
        with mock.patch.dict(browser_tools.os.environ, {"CARGO_REGISTRY_TOKEN": "secret-value"}), \
                mock.patch.object(browser_tools.subprocess, "run", return_value=mock.Mock(
                    returncode=101, stdout="", stderr="error: failed to compile; token secret-value")):
            with self.assertRaises(SystemExit) as error:
                browser_tools.build_drag_harness()
            self.assertIn("failed to compile", str(error.exception))
            self.assertNotIn("secret-value", str(error.exception))

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
