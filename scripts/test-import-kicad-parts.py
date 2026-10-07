#!/usr/bin/env python3
"""Importer tests; they build Core's `artifact_request` example first."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).with_name("import-kicad-parts.py")
SOURCE = '(footprint "test" (layer "F.Cu") (pad "1" smd rect (at 0 0) (size 1 1) (layers "F.Cu" "F.Paste" "F.Mask")) (pad "1" smd rect (at 3 0) (size 1 1) (layers "F.Cu" "F.Paste" "F.Mask")) (pad "2" np_thru_hole circle (at 6 0) (size 2 2) (drill 2) (layers "*.Cu" "*.Mask")))'


class ImportKicadPartsTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        root = SCRIPT.parents[1]
        subprocess.run(["cargo", "build", "--manifest-path", str(root / "core/Cargo.toml"), "--locked",
                        "--example", "artifact_request"], check=True)

    def test_explicit_roles_retain_duplicate_smd_pads_and_reject_holes_and_source_drift_atomically(self):
        with tempfile.TemporaryDirectory() as name:
            directory = Path(name)
            (directory / "part.kicad_mod").write_text(SOURCE, encoding="utf-8")
            (directory / "LICENSE").write_text("Test fixture authored for this test.", encoding="utf-8")
            entry = {
                "id": "test", "name": "Test", "kind": "custom", "file": "part.kicad_mod",
                "sha256": hashlib.sha256(SOURCE.encode()).hexdigest(), "repository": "https://example.com/test",
                "revision": "test", "sourcePath": "part.kicad_mod", "license": "test", "licenseFile": "LICENSE",
                "terminals": {"signal": ["1"]},
            }

            def run(entries):
                (directory / "manifest.json").write_text(json.dumps({"formatVersion": 1, "entries": entries}), encoding="utf-8")
                return subprocess.run([sys.executable, str(SCRIPT), str(directory / "manifest.json")], capture_output=True, text=True)

            valid = run([entry])
            self.assertEqual(valid.returncode, 0, valid.stderr)
            output = directory / "imported-parts.json"
            accepted = output.read_text(encoding="utf-8")
            definition = json.loads(accepted)["parts"][0]["definition"]
            self.assertEqual(len(definition["terminals"]["signal"]), 2)
            self.assertEqual(definition["kicadSource"]["source"], SOURCE)
            for entries, message in [
                ([{**entry, "terminals": {"signal": ["2"]}}], "Missing conductive pad"),
                ([{**entry, "sha256": "incorrect"}], "Source hash mismatch"),
                ([entry, entry], "Duplicate definition ID"),
                ([{**entry, "matrixTerminals": {"row": "signal", "column": "missing"}}], "Missing matrix terminal"),
            ]:
                rejected = run(entries)
                self.assertNotEqual(rejected.returncode, 0)
                self.assertRegex(rejected.stderr, re.escape(message))
                self.assertEqual(output.read_text(encoding="utf-8"), accepted)


if __name__ == "__main__":
    unittest.main()
