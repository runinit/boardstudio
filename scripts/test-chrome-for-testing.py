#!/usr/bin/env python3
"""Slice 1: decoupled Chrome-for-Testing pin.

Runs WITHOUT network:
(a) scripts/chrome-for-testing.json parses, has version + chrome_sha256
    + chromedriver_sha256 (64-hex);
(b) scripts/install-chrome-for-testing.sh contains no hardcoded version
    literal (it reads the JSON);
(c) hash verification rejects a tampered fake archive (temp file exercised
    through the same `sha256sum -c` verify path the installer uses).
"""
from __future__ import annotations

import hashlib
import json
import re
import subprocess
import tempfile
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
PIN_JSON = REPO / "scripts" / "chrome-for-testing.json"
INSTALLER = REPO / "scripts" / "install-chrome-for-testing.sh"
HEX64 = re.compile(r"^[0-9a-f]{64}$")


class ChromeForTestingPinTests(unittest.TestCase):
    def test_pin_json_parses_with_version_and_hashes(self):
        pin = json.loads(PIN_JSON.read_text())
        self.assertIn("version", pin)
        self.assertRegex(pin["version"], r"^\d+\.\d+\.\d+\.\d+$")
        for key in ("chrome_sha256", "chromedriver_sha256"):
            self.assertIn(key, pin)
            self.assertRegex(pin[key], HEX64, f"{key} must be 64-hex sha256")

    def test_installer_reads_version_from_json(self):
        text = INSTALLER.read_text()
        self.assertIn("chrome-for-testing.json", text,
                      "installer must read the version/hash JSON pin")
        pin = json.loads(PIN_JSON.read_text())
        self.assertNotIn(pin["version"], text,
                         "installer must not hardcode the pinned version")

    def test_verify_rejects_tampered_archive(self):
        text = INSTALLER.read_text()
        self.assertIn("sha256sum -c", text,
                      "installer must verify downloads with sha256sum -c")
        pin = json.loads(PIN_JSON.read_text())
        with tempfile.TemporaryDirectory() as tmp:
            fake = Path(tmp) / "fake.zip"
            fake.write_bytes(b"fake-chrome-archive-bytes")
            real_digest = hashlib.sha256(fake.read_bytes()).hexdigest()
            # Positive control: correct hash verifies.
            ok = subprocess.run(
                ["sha256sum", "-c", "-"], input=f"{real_digest}  {fake}\n",
                capture_output=True, text=True)
            self.assertEqual(ok.returncode, 0, ok.stderr)
            # Tampered archive: recorded pin hash must NOT verify a fake file.
            bad = subprocess.run(
                ["sha256sum", "-c", "-"],
                input=f"{pin['chrome_sha256']}  {fake}\n",
                capture_output=True, text=True)
            self.assertNotEqual(bad.returncode, 0,
                                "tampered archive must fail verification")


if __name__ == "__main__":
    unittest.main()
