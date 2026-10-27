#!/usr/bin/env python3
"""Run the Parts inspector scroll check against a running site."""
import os

if not os.environ.get("BOARDSTUDIO_PREVIEW_URL"):
    raise SystemExit("set BOARDSTUDIO_PREVIEW_URL to the running BoardStudio site")

from browser_tools import _build_harness  # noqa: E402

binary = _build_harness("parts-inspector-scroll")
os.execv(str(binary), [str(binary)])
