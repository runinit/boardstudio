"""Refresh the independent VIK STEP bound oracle with installed FreeCAD.

Run from the repository root:
  PYTHONPATH=/usr/lib/freecad/lib /usr/bin/python3 cad/test/measure-vik-model-bounds.py

This measures raw source geometry, not assembly alignment or hardware fit.
"""
import hashlib
import json
from pathlib import Path

import FreeCAD
import Part

root = Path(__file__).resolve().parents[2]
ledger = json.loads((root / 'app/src/modules/asset-ledger.json').read_text())
measurements = []
for model in ledger['models']:
    source = root / model['bundledFile']
    if source.suffix.lower() not in ('.step', '.stp'):
        continue
    shape = Part.read(str(source))
    bounds = shape.optimalBoundingBox()
    measurements.append({
        'assetId': model['assetId'],
        'sha256': hashlib.sha256(source.read_bytes()).hexdigest(),
        'min': [getattr(bounds, axis + 'Min') for axis in ('X', 'Y', 'Z')],
        'max': [getattr(bounds, axis + 'Max') for axis in ('X', 'Y', 'Z')],
    })

output = root / 'cad/test/fixtures/vik-model-tight-bounds.json'
output.write_text(json.dumps({
    'measurement': 'FreeCAD Part.Shape.optimalBoundingBox in source millimetres; independent of CAD mesh importer',
    'freecadVersion': FreeCAD.Version(),
    'models': measurements,
}, indent=2) + '\n')
print(f'Recorded {len(measurements)} pinned STEP measurements in {output}')
