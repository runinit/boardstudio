#!/usr/bin/env python3
"""Run the Parts inspector scroll check against a running site."""
import os

if not os.environ.get("BOARDSTUDIO_PREVIEW_URL"):
    raise SystemExit("set BOARDSTUDIO_PREVIEW_URL to the running BoardStudio site")

from browser_tools import MissingBrowserTool, _build_harness, browser_env  # noqa: E402

try:
    environment = browser_env()
    binary = _build_harness("parts-inspector-scroll")
except MissingBrowserTool as error:
    raise SystemExit(f"Parts inspector: {error}") from error

os.execve(str(binary), [str(binary)], environment)
