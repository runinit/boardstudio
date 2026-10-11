#!/usr/bin/env python3
"""Tests for the shared catalogue tooling: JSON output, KiCad forms and layout extraction."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import sys
import unittest

HERE = Path(__file__).parent
sys.path.insert(0, str(HERE))

import catalogue_json  # noqa: E402
from kicad_forms import child, parse_forms, value  # noqa: E402


def load(name: str, file: str):
    spec = importlib.util.spec_from_file_location(name, HERE / file)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module




class CatalogueJsonTests(unittest.TestCase):
    def test_numbers_match_javascript(self):
        # Expected strings were produced by Node's Number-to-string conversion.
        for number, expected in [
            (1.0, "1"), (0.5, "0.5"), (1e21, "1e+21"), (1e-7, "1e-7"), (0.00001, "0.00001"),
            (123456789012345680000.0, "123456789012345680000"), (-0.0, "0"), (1.5e-10, "1.5e-10"),
            (100.0, "100"), (1e300, "1e+300"), (0.000001, "0.000001"), (1.23e-6, "0.00000123"), (12345.678, "12345.678"),
        ]:
            self.assertEqual(catalogue_json.number(number), expected)

    def test_integer_like_keys_come_first_in_ascending_order(self):
        text = catalogue_json.dumps({"b": 1, "10": 2, "2": 3, "a": 4, "01": 5})
        self.assertEqual(text, '{\n  "2": 3,\n  "10": 2,\n  "b": 1,\n  "a": 4,\n  "01": 5\n}')

    def test_layout_matches_json_stringify(self):
        self.assertEqual(catalogue_json.dumps({"a": [1, 2.0, {"b": None}], "c": {}, "d": [], "é": True}),
                         '{\n  "a": [\n    1,\n    2,\n    {\n      "b": null\n    }\n  ],\n  "c": {},\n  "d": [],\n  "é": true\n}')


class KicadFormsTests(unittest.TestCase):
    def test_parses_nested_forms_and_distinguishes_quoted_strings(self):
        forms = parse_forms('(a (b "x\\"y" 1.5) c) (d)')
        self.assertEqual(len(forms), 2)
        self.assertEqual(value(child(forms[0], "b")[1]), 'x"y')
        self.assertEqual(child(forms[0], "b")[2], "1.5")
        self.assertNotEqual(child(forms[0], "b")[1], 'x"y')

    def test_rejects_unbalanced_and_bare_atoms(self):
        for source in ("(a", ")", "atom"):
            with self.assertRaises(ValueError):
                parse_forms(source)



if __name__ == "__main__":
    unittest.main()
