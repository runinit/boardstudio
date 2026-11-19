"""The check scheduler's job budget, its overlap marker and the standalone Tailwind CLI lookup.

A leaf module, so `check.py`, `build-web.py` and `run-wasm-tests.py` can share these without
importing one another.
"""

from __future__ import annotations

import os
from pathlib import Path
import shutil
import subprocess

TAILWIND_ENV = "BOARDSTUDIO_TAILWINDCSS"
DX_TOOLS = Path.home() / ".local/share/.dx/tools"
# Set to "1" for the `build`, `lint` and `test` steps `check.py` runs together. It generated their
# shared stylesheet first, and owns their process group.
IN_OVERLAP_ENV = "BOARDSTUDIO_IN_OVERLAP"


def check_jobs() -> int:
    default = min(6, os.cpu_count() or 4)
    try:
        return max(1, int(os.environ.get("BOARDSTUDIO_CHECK_JOBS", str(default))))
    except ValueError:
        return default


def tailwind_cli() -> Path | None:
    """A standalone Tailwind v4 CLI: $BOARDSTUDIO_TAILWINDCSS, PATH, then the newest dx-installed one."""
    def is_v4(path: Path) -> bool:
        try:
            result = subprocess.run([str(path), "--help"], capture_output=True, text=True)
        except OSError:
            return False
        return "tailwindcss v4." in result.stdout + result.stderr

    if explicit := os.environ.get(TAILWIND_ENV):
        if not is_v4(Path(explicit)):
            raise ValueError(f"{TAILWIND_ENV}={explicit} is not a standalone Tailwind v4 CLI")
        return Path(explicit)
    candidates = [Path(found)] if (found := shutil.which("tailwindcss")) else []
    installed = [path / "tailwindcss" if path.is_dir() else path for path in DX_TOOLS.glob("tailwindcss-*")]
    candidates += sorted((path for path in installed if path.is_file() and os.access(path, os.X_OK)),
                         key=lambda path: path.stat().st_mtime, reverse=True)
    return next((path for path in candidates if is_v4(path)), None)
