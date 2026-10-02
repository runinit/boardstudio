#!/usr/bin/env python3
"""The build source guard covers every input to the packaged Ergogen catalogue."""
from importlib.util import module_from_spec, spec_from_file_location
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest import TestCase, main
from unittest.mock import patch

SPEC = spec_from_file_location("build_m1", Path(__file__).with_name("build-m1.py"))
BUILD = module_from_spec(SPEC)
SPEC.loader.exec_module(BUILD)
INPUTS = (
    "ergogen/scripts/generate.mjs",
    "ergogen/library/switch_gateron_ks27_ks33.js",
    "ergogen/library/src/defaultModels.mjs",
    "ergogen/package.json",
    "package.json",
    "pnpm-lock.yaml",
    "pnpm-workspace.yaml",
)


class BuildSourceTests(TestCase):
    def test_catalogue_inputs_are_hashed_and_changes_trip_the_final_guard(self):
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            for name in INPUTS:
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b"initial input")
            tracked = ("\0".join(INPUTS) + "\0").encode()
            with patch.object(BUILD, "REPO", root), patch.object(
                BUILD.subprocess, "check_output", return_value=tracked
            ):
                original = BUILD.sources()
                self.assertEqual(set(original), set(INPUTS))
                for name in INPUTS:
                    path = root / name
                    path.write_bytes(b"changed input")
                    self.assertNotEqual(BUILD.sources(), original, name)
                    path.write_bytes(b"initial input")
                self.assertEqual(BUILD.sources(), original)


if __name__ == "__main__":
    main()
