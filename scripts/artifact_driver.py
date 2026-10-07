"""Run Core artifact requests through the native `artifact_request` example."""

from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def driver_path() -> str:
    return os.environ.get("BOARDSTUDIO_ARTIFACT_DRIVER") or str(ROOT / "target/debug/examples/artifact_request")


def request(payload: dict, failure: str) -> dict:
    """Send one request and return the parsed reply (which may be an error reply)."""
    result = subprocess.run([driver_path()], input=json.dumps(payload) + "\n", capture_output=True, text=True, encoding="utf-8")
    if result.returncode != 0:
        raise RuntimeError(result.stderr or failure)
    return json.loads(result.stdout.strip())
