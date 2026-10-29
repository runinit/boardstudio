#!/usr/bin/env python3
"""Hermetic checks for the opt-in packaged WEB provider runner."""
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest import mock

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("face_web", HERE / "face-completeness-web.py")
face_web = importlib.util.module_from_spec(spec)
spec.loader.exec_module(face_web)


class FaceWebTests(unittest.TestCase):
    def test_source_mapping_uses_original_assertions_and_release(self):
        source = (HERE.parent / "wasm/tests/step-face-completeness.cjs").read_text()
        mapped = face_web.regression_body(source)
        self.assertEqual(mapped, source[source.index('// Display mesh'):source.index("console.log('PASS")]
                         + 'vs.analysisHandle.free();\n')
        self.assertIn("fallback meshed faces count", mapped)
        self.assertIn("Math.fround", mapped)
        self.assertIn("triangle tag must be valid", mapped)

    def test_source_mapping_rejects_missing_or_duplicate_boundaries(self):
        source = (HERE.parent / "wasm/tests/step-face-completeness.cjs").read_text()
        for bad in (source.replace('// Display mesh', '// missing'), source + '\n// Display mesh'):
            with self.subTest(bad=bad[-30:]), self.assertRaisesRegex(ValueError, "source boundary"):
                face_web.regression_body(bad)

    def test_package_preflight_requires_all_files_and_current_stamp(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            package = root / "wasm/pkg/boardstudio_cad_wasm.js"
            wasm = package.with_name("boardstudio_cad_wasm_bg.wasm")
            stamp = root / ".cache/cad/wasm-build.sha256"
            package.parent.mkdir(parents=True)
            stamp.parent.mkdir(parents=True)
            with mock.patch.object(face_web.cad_build, "wasm_inputs_digest", side_effect=lambda no_opt=False: "fresh" if no_opt else "release"):
                with self.assertRaisesRegex(RuntimeError, "missing"):
                    face_web.preflight(root)
                package.write_text("package")
                wasm.write_bytes(b"\0asm\x01\0\0\0")
                stamp.write_text("stale")
                with self.assertRaisesRegex(RuntimeError, "stale"):
                    face_web.preflight(root)
                stamp.write_text("fresh\n")
                self.assertEqual(face_web.preflight(root), (package, wasm))
                package.unlink()
                with self.assertRaisesRegex(RuntimeError, "missing"):
                    face_web.preflight(root)


if __name__ == "__main__":
    unittest.main()
