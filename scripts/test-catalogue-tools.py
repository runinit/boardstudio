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


keyboards = load("extract_keyboard_layouts", "extract-keyboard-layouts.py")
sofle = load("extract_sofle_layouts", "extract-sofle-layouts.py")


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


class LayoutExtractionTests(unittest.TestCase):
    def test_measures_switches_and_skips_stabilizers_and_unmatched_references(self):
        board = parse_forms("""(kicad_pcb
          (module MX_Alps_Hybrid:MX-1.5U (at 10 20 90) (fp_text reference SW2 (at 0 0)))
          (module Keyswitch:MX-1U (at 0 5 -45) (fp_text reference SW1 (at 0 0)))
          (module Keyswitch:MX_Stabilizer (at 1 1) (fp_text reference SW3 (at 0 0)))
          (module Diode:D (at 2 2) (fp_text reference D1 (at 0 0)))
          (module Keyswitch:MX-1U (at 3 3) (fp_text reference SW99 (at 0 0))))""")[0]
        keys = keyboards.measure(board, "test", keyboards.numbered("SW", 2))
        self.assertEqual([(key["reference"], key["x"], key["y"], key["rotation"], key["width"]) for key in keys],
                         [("SW1", 0.0, 5.0, -45.0, 1.0), ("SW2", 10.0, 20.0, -90.0, 1.5)])

    def test_rotation_is_folded_into_a_half_turn(self):
        self.assertEqual([keyboards.snap_rotation(angle) for angle in (0, 90, -90, 180, 45)], [0, -90, -90, 0, 45])

    def test_chains_edge_segments_into_contours_and_orders_them_by_area(self):
        def line(a, b):
            return f"(gr_line (start {a[0]} {a[1]}) (end {b[0]} {b[1]}) (layer Edge.Cuts))"
        square = lambda x, size: [(x, 0), (x + size, 0), (x + size, size), (x, size)]
        edges = []
        for points in (square(0, 2), square(20, 10)):
            edges += [line(points[i], points[(i + 1) % 4]) for i in (0, 1, 2, 3)]
        # Reverse one segment: chaining must follow endpoints in either direction.
        edges[1] = line((12, 0), (12, 2)).replace("(start 12 0) (end 12 2)", "(start 2 2) (end 2 0)")
        board = parse_forms(f"(kicad_pcb {' '.join(edges)})")[0]
        contours = sorted(sofle.contours_of(board, "test"), key=sofle.area, reverse=True)
        self.assertEqual([round(sofle.area(contour)) for contour in contours], [200, 8])

    def test_disconnected_outline_is_rejected(self):
        board = parse_forms("(kicad_pcb (gr_line (start 0 0) (end 1 0) (layer Edge.Cuts)))")[0]
        with self.assertRaisesRegex(ValueError, "disconnected outline"):
            sofle.contours_of(board, "test")

    def test_reference_order_is_numeric(self):
        refs = ["SW10", "SW2", "U1", "RSW1", "SW1", "D31"]
        self.assertEqual(sorted(refs, key=sofle.natural), ["D31", "RSW1", "SW1", "SW2", "SW10", "U1"])


if __name__ == "__main__":
    unittest.main()
