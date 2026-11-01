#!/usr/bin/env python3
"""Regression for Rust's compact receipt serialization."""
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('drag', Path(__file__).with_name('run-drag-browser.py'))
drag = importlib.util.module_from_spec(spec)
spec.loader.exec_module(drag)

class ReceiptBytesTests(unittest.TestCase):
    def test_rust_float_bytes_and_tampering(self):
        names = ['Part drag', 'Column body drag', 'Stagger cancel', 'Stagger handle', 'Ctrl Offset handle', 'Default collision PCB pair', 'Case viewer with overflowing collision findings', 'Qualified collision product fixture', 'Unavailable physical PCB stays uncertain']
        checks = ','.join(json.dumps({'name': n}, separators=(',', ':')) for n in names)
        payload = ('{"passed":true,"run_id":"r","source_head":"h","source_sha256":"s","build_sha256":"b","float":1e-7,"checks":[' + checks + ']}').encode()
        raw = payload[:-1] + b',"receipt_sha256":"' + hashlib.sha256(payload).hexdigest().encode() + b'"}\n'
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'receipt.json'
            path.write_bytes(raw)
            drag.verify_receipt(path, 'r', 'h', 's', 'b')
            path.write_bytes(raw.replace(b'1e-7', b'2e-7'))
            with self.assertRaises(ValueError):
                drag.verify_receipt(path, 'r', 'h', 's', 'b')

if __name__ == '__main__':
    unittest.main()
