#!/usr/bin/env python3
"""Verify real drag-consumer failures cannot produce a success receipt.

Build the drag-smoke binary first and supply the pinned browser via CHROME_BIN.
"""

import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / "tools/browser-harness/target/debug/drag-smoke"
BUTTONS = "".join(
    f"<button>{name}</button>"
    for name in ("Create new keyboard", "Split keyboard", "Continue to layout")
)
FIXTURE = '''<div role="slider" tabindex="0" aria-label="Rows" aria-valuenow="10"></div>
<script>
const slider = document.querySelector('[role=slider]');
const get = slider.getAttribute.bind(slider); let reads = 0;
slider.getAttribute = name => name === 'aria-valuenow' ? (++reads === 1 ? '10' : '9') : get(name);
</script>'''

with tempfile.TemporaryDirectory(prefix="ticket005-controls-", dir="/tmp") as directory:
    base = Path(directory)
    controls = (
        ("assertion", "", "slider Rows did not reach 10"),
        ("rejection", "<script>Promise.reject(new Error('consumer rejection control'))</script>",
         "consumer rejection control"),
    )
    for name, prefix, expected in controls:
        page = base / f"{name}.html"
        page.write_text("<!doctype html>" + prefix + BUTTONS + FIXTURE)
        receipt = base / name / "receipt.json"
        env = os.environ.copy()
        env.update(
            BOARDSTUDIO_PREVIEW_URL=page.as_uri(),
            BOARDSTUDIO_DRAG_RECEIPT_PATH=str(receipt),
            BOARDSTUDIO_DRAG_SOURCE_HEAD="control",
            BOARDSTUDIO_DRAG_SOURCE_SHA256="control",
            BOARDSTUDIO_DRAG_BUILD_SHA256="control",
            BOARDSTUDIO_DRAG_REPO_ROOT=str(ROOT),
        )
        result = subprocess.run([str(BINARY)], env=env, text=True, capture_output=True, timeout=90)
        output = result.stdout + result.stderr
        print(f"{name}: exit {result.returncode}, receipt exists {receipt.exists()}")
        print(output, flush=True)
        if result.returncode == 0 or receipt.exists() or expected not in output:
            raise SystemExit(f"{name} control failed")
