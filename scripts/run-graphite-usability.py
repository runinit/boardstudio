#!/usr/bin/env python3
"""Run the manual Rust Graphite usability check against BOARDSTUDIO_PREVIEW_URL."""
import os
import subprocess
import sys
from pathlib import Path

if os.environ.get("BOARDSTUDIO_GRAPHITE_IDENTITY") or os.environ.get("BOARDSTUDIO_GRAPHITE_BUILD_RUN"):
    subprocess.run([sys.executable, str(Path(__file__).with_name("graphite-identity.py"))], check=True)
from browser_tools import _build_harness

# Usability and feedback share the same acceptance workflow.
binary = _build_harness("graphite-feedback")
os.execv(str(binary), [str(binary)])
