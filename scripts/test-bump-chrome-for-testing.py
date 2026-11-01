#!/usr/bin/env python3
"""Slices 4+5: CfT bump script + CI provisioned from the pin.

Hermetic (no network):
(a) scripts/bump-chrome-for-testing.py exists and its pin-update function
    rewrites version+hashes in a temp copy of the pin JSON;
(b) .github/workflows/check.yaml provisions Chrome from
    install-chrome-for-testing.sh / the pin file and no longer relies on bare
    `google-chrome --version` from the runner image.
"""
from __future__ import annotations

import importlib.util
import json
import shutil
import tempfile
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
BUMP = REPO / "scripts" / "bump-chrome-for-testing.py"
PIN_JSON = REPO / "scripts" / "chrome-for-testing.json"
WORKFLOW = REPO / ".github" / "workflows" / "check.yaml"


def load_bump():
    spec = importlib.util.spec_from_file_location("bump_cft", BUMP)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class BumpChromeForTestingTests(unittest.TestCase):
    def test_pin_rewrite_updates_temp_copy(self):
        self.assertTrue(BUMP.exists(), "scripts/bump-chrome-for-testing.py must exist")
        module = load_bump()
        rewrite = getattr(module, "rewrite_pin", None)
        self.assertTrue(callable(rewrite), "bump script must expose rewrite_pin()")
        with tempfile.TemporaryDirectory() as tmp:
            copy = Path(tmp) / "chrome-for-testing.json"
            shutil.copy(PIN_JSON, copy)
            rewrite(copy, "999.0.0.0", "a" * 64, "b" * 64)
            pin = json.loads(copy.read_text())
            self.assertEqual(pin["version"], "999.0.0.0")
            self.assertEqual(pin["chrome_sha256"], "a" * 64)
            self.assertEqual(pin["chromedriver_sha256"], "b" * 64)
        # Real pin file untouched by the hermetic rewrite.
        live = json.loads(PIN_JSON.read_text())
        self.assertNotEqual(live["version"], "999.0.0.0")

    def test_ci_provisions_chrome_from_pin(self):
        text = WORKFLOW.read_text()
        self.assertIn("install-chrome-for-testing.sh", text)
        self.assertIn("chrome-for-testing", text)
        self.assertNotIn("google-chrome --version", text,
                         "CI must not rely on bare image google-chrome --version")


if __name__ == "__main__":
    unittest.main()
