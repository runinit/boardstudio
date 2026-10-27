#!/usr/bin/env python3
"""Graphite comparison preflight must reject invalid bindings before browser launch."""
import os
import importlib.util
import io
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]


class GraphitePreflightTests(unittest.TestCase):
    def test_served_files_must_match_package(self):
        spec = importlib.util.spec_from_file_location("graphite_identity", ROOT / "scripts/graphite-identity.py")
        identity = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(identity)
        with tempfile.TemporaryDirectory() as directory:
            site = Path(directory)
            (site / "index.html").write_bytes(b"built")
            class Response(io.BytesIO):
                def geturl(self):
                    return "http://localhost/boardstudio/index.html"
            with patch.object(identity, "urlopen", side_effect=lambda *a, **k: Response(b"built")):
                identity.verify_served(site, "http://localhost/boardstudio/")
            with patch.object(identity, "urlopen", side_effect=lambda *a, **k: Response(b"stale")):
                with self.assertRaisesRegex(ValueError, "served file differs"):
                    identity.verify_served(site, "http://localhost/boardstudio/")

    def test_missing_receipt_rejected_before_harness_build(self):
        with tempfile.TemporaryDirectory() as directory:
            env = dict(os.environ, BOARDSTUDIO_GRAPHITE_BUILD_RUN=str(Path(directory) / "missing"))
            result = subprocess.run([sys.executable, "-B", "scripts/run-graphite-feedback.py"],
                                    cwd=ROOT, env=env, capture_output=True, text=True, timeout=20)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("build receipt", result.stderr)
            self.assertNotIn("cannot build the Rust harness", result.stderr)


if __name__ == "__main__":
    unittest.main()
