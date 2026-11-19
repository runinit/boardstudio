#!/usr/bin/env python3
"""Regression for Rust's compact receipt serialization and per-suite verification."""
import hashlib
import importlib.util
import json
from pathlib import Path
import os
import tempfile
import unittest
from unittest import mock

spec = importlib.util.spec_from_file_location('drag', Path(__file__).with_name('run-drag-browser.py'))
drag = importlib.util.module_from_spec(spec)
spec.loader.exec_module(drag)

ALL = ['Part drag', 'Part drag undo', 'Column body drag', 'Column body drag undo', 'Stagger cancel',
       'Stagger handle', 'Stagger handle undo', 'Ctrl Offset handle', 'Ctrl Offset handle undo',
       'Matrix move', 'Matrix move undo', 'Default collision PCB pair',
       'Case viewer with overflowing collision findings', 'Qualified collision product fixture',
       'Unavailable physical PCB stays uncertain']
DRAG = ALL[:11]
COLLISION = ALL[11:]


def write_receipt(directory, names, *, run_id='r', head='h', source='s', build='b', extra=b''):
    checks = ','.join(json.dumps({'name': n}, separators=(',', ':')) for n in names)
    payload = ('{"passed":true,"run_id":"%s","source_head":"%s","source_sha256":"%s","build_sha256":"%s"%s,"checks":[%s]}'
               % (run_id, head, source, build, extra.decode(), checks)).encode()
    raw = payload[:-1] + b',"receipt_sha256":"' + hashlib.sha256(payload).hexdigest().encode() + b'"}\n'
    path = Path(directory) / 'receipt.json'
    path.write_bytes(raw)
    return path, raw


class ReceiptBytesTests(unittest.TestCase):
    def test_rust_float_bytes_and_tampering(self):
        payload_extra = b',"float":1e-7'
        with tempfile.TemporaryDirectory() as directory:
            path, raw = write_receipt(directory, ALL, extra=payload_extra)
            drag.verify_receipt(path, 'r', 'h', 's', 'b')
            path.write_bytes(raw.replace(b'1e-7', b'2e-7'))
            with self.assertRaises(ValueError):
                drag.verify_receipt(path, 'r', 'h', 's', 'b')

    def test_suites_partition_all_fifteen_checks(self):
        self.assertEqual(len(ALL), 15)
        self.assertEqual((len(DRAG), len(COLLISION)), (11, 4))
        self.assertEqual(set(DRAG) | set(COLLISION), set(ALL))
        self.assertEqual(set(DRAG) & set(COLLISION), set())
        self.assertEqual(drag.SUITE_CHECKS['all'], set(ALL))
        self.assertEqual(drag.SUITE_CHECKS['drag'], set(DRAG))
        self.assertEqual(drag.SUITE_CHECKS['collision'], set(COLLISION))

    def test_verify_receipt_enforces_exact_suite_checks(self):
        with tempfile.TemporaryDirectory() as directory:
            drag_path, _ = write_receipt(directory, DRAG)
            drag.verify_receipt(drag_path, 'r', 'h', 's', 'b', 'drag')
            with self.assertRaisesRegex(ValueError, 'missing checks'):
                drag.verify_receipt(drag_path, 'r', 'h', 's', 'b', 'all')
            (Path(directory) / 'receipt.json').unlink()
            collision_path, _ = write_receipt(directory, COLLISION)
            drag.verify_receipt(collision_path, 'r', 'h', 's', 'b', 'collision')
            with self.assertRaisesRegex(ValueError, 'missing checks'):
                drag.verify_receipt(collision_path, 'r', 'h', 's', 'b', 'all')
            with self.assertRaisesRegex(ValueError, 'missing checks'):
                drag.verify_receipt(collision_path, 'r', 'h', 's', 'b', 'drag')

    def test_verify_receipt_rejects_extra_and_unknown_suite(self):
        with tempfile.TemporaryDirectory() as directory:
            path, _ = write_receipt(directory, DRAG + ['Stray check'])
            with self.assertRaisesRegex(ValueError, 'unexpected checks'):
                drag.verify_receipt(path, 'r', 'h', 's', 'b', 'drag')
            path, _ = write_receipt(directory, DRAG)
            with self.assertRaisesRegex(ValueError, 'unknown drag browser suite'):
                drag.verify_receipt(path, 'r', 'h', 's', 'b', 'bogus')

    def test_parse_args_defaults_and_env(self):
        with mock.patch.dict(os.environ, {}, clear=True):
            self.assertEqual(drag.parse_args([]).suite, 'all')
        with mock.patch.dict(os.environ, {'BOARDSTUDIO_DRAG_SUITE': 'drag'}):
            self.assertEqual(drag.parse_args([]).suite, 'drag')
        self.assertEqual(drag.parse_args(['--suite', 'collision']).suite, 'collision')
        with self.assertRaises(SystemExit):
            drag.parse_args(['--suite', 'bogus'])

if __name__ == '__main__':
    unittest.main()
