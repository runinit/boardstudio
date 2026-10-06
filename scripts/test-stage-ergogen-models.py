#!/usr/bin/env python3
"""Behavior tests for safe packaged Ergogen model staging."""
from importlib.util import module_from_spec, spec_from_file_location
from pathlib import Path
import tempfile
import unittest

SCRIPT = Path(__file__).with_name("stage-ergogen-models.py")
SPEC = spec_from_file_location("stage_ergogen_models", SCRIPT)
assert SPEC and SPEC.loader
STAGER = module_from_spec(SPEC)
SPEC.loader.exec_module(STAGER)


class StageModelsTests(unittest.TestCase):
    def test_aliases_preserve_source_bytes_under_safe_static_paths(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source_root = root / "vendor"
            source = source_root / "thqwgd001/3d_models/THQWGD001-rotation.stp"
            source.parent.mkdir(parents=True)
            source.write_bytes(b"opaque source payload")
            spaced = source_root / "sample/3d_models/nested/Board Model.STEP"
            spaced.parent.mkdir(parents=True)
            spaced.write_bytes(b"")
            nested = source_root / "infused-kim/3d_models/trackpoint/TP_Cap_Green_T430.step"
            nested.parent.mkdir(parents=True)
            nested.write_bytes(b"nested model")
            destination = root / "public/assets/ergogen-models"
            manifest_path = root / "model-manifest.json"

            entries = STAGER.stage_models(source_root, destination, manifest_path)

            alias = next(entry for entry in entries if entry["sourceRelativePath"].endswith("THQWGD001-rotation.stp"))
            self.assertEqual(alias["id"], "bundled-model:thqwgd001/THQWGD001 #1.stp")
            self.assertEqual(alias["filename"], "THQWGD001 #1.stp")
            self.assertEqual((destination / Path(alias["emittedPath"]).name).read_bytes(), source.read_bytes())
            board_model = next(entry for entry in entries if entry["sourceRelativePath"].endswith("Board Model.STEP"))
            self.assertEqual(board_model["filename"], "nested/Board Model.STEP")
            self.assertEqual(board_model["id"], "bundled-model:sample/nested/Board Model.STEP")
            nested_model = next(entry for entry in entries if entry["sourceRelativePath"].endswith("TP_Cap_Green_T430.step"))
            self.assertEqual(nested_model["filename"], "trackpoint/TP_Cap_Green_T430.step")
            self.assertEqual(nested_model["mediaType"], "model/step")
            for entry in entries:
                self.assertNotRegex(entry["emittedPath"], r"[%#?\\:]")
                self.assertTrue((destination / Path(entry["emittedPath"]).name).exists())

    def test_source_tree_is_catalogued_once_and_stage_rerun_prunes_stale_entries(self):
        root = Path(__file__).resolve().parents[1]
        source_root = root / "ergogen/library/vendor"
        entries = STAGER.catalogue(source_root)
        self.assertEqual(len(entries), 88)
        self.assertEqual(len({entry["id"] for entry in entries}), len(entries))
        self.assertEqual(len({entry["emittedPath"] for entry in entries}), len(entries))

        with tempfile.TemporaryDirectory() as temporary:
            stage = Path(temporary) / "assets/ergogen-models"
            manifest = Path(temporary) / "model-catalog.json"
            STAGER.stage_models(source_root, stage, manifest)
            # No output entry depends on an encoded vendor path.
            self.assertEqual({path.name for path in stage.iterdir()}, {Path(entry["emittedPath"]).name for entry in entries})
            self.assertTrue(all((stage / Path(entry["emittedPath"]).name).read_bytes() == (source_root / entry["sourceRelativePath"]).read_bytes() for entry in entries))
            self.assertEqual(STAGER.stage_models(source_root, stage, manifest), entries)


if __name__ == "__main__":
    unittest.main()
