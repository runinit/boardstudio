#!/usr/bin/env python3
"""Run the real-page drag smoke against the checked site build."""

from __future__ import annotations

from datetime import datetime, timezone
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import threading
import uuid
from http.server import ThreadingHTTPServer
from urllib.request import urlopen

sys.path.insert(0, str(Path(__file__).resolve().parent))
import browser_tools  # noqa: E402  (needs the scripts directory on sys.path)

ROOT = Path(__file__).resolve().parents[1]
SITE = ROOT / "web/target/site"
RESULTS = ROOT / "web/target/test-results/drag-browser"


def module_at(path: Path, name: str):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


def digest_files(paths: list[Path], base: Path) -> str:
    digest = hashlib.sha256()
    for path in sorted(paths):
        digest.update(path.relative_to(base).as_posix().encode())
        with path.open("rb") as source:
            for block in iter(lambda: source.read(1024 * 1024), b""):
                digest.update(block)
    return digest.hexdigest()


def source_identity() -> tuple[str, str]:
    head = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT, check=True,
                          capture_output=True, text=True).stdout.strip()
    digest = hashlib.sha256()
    diff = subprocess.run(["git", "diff", "--binary", "HEAD"], cwd=ROOT, check=True,
                          capture_output=True).stdout
    digest.update(diff)
    untracked = subprocess.run(["git", "ls-files", "--others", "--exclude-standard"], cwd=ROOT,
                               check=True, capture_output=True, text=True).stdout.splitlines()
    for relative in sorted(untracked):
        path = ROOT / relative
        if not path.is_file() or "target/" in relative or relative.startswith(".evidence/"):
            continue
        digest.update(relative.encode())
        with path.open("rb") as source:
            for block in iter(lambda: source.read(1024 * 1024), b""):
                digest.update(block)
    return head, digest.hexdigest()


def verify_receipt(path: Path, run_id: str, head: str, source_hash: str, build_hash: str) -> None:
    receipt = json.loads(path.read_text())
    checksum = receipt.pop("receipt_sha256")
    payload = json.dumps(receipt, separators=(",", ":"), ensure_ascii=False).encode()
    if hashlib.sha256(payload).hexdigest() != checksum:
        raise ValueError("drag browser receipt checksum does not match")
    if (receipt.get("passed") is not True or receipt.get("run_id") != run_id
            or receipt.get("source_head") != head or receipt.get("source_sha256") != source_hash
            or receipt.get("build_sha256") != build_hash):
        raise ValueError("drag browser receipt does not match this successful run")
    names = {check.get("name") for check in receipt.get("checks", [])}
    expected = {"Part drag", "Column body drag", "Stagger cancel", "Stagger handle", "Ctrl Offset handle",
                "Matrix move",
                "Default collision PCB pair", "Case viewer with overflowing collision findings",
                "Qualified collision product fixture", "Unavailable physical PCB stays uncertain"}
    if not expected.issubset(names):
        raise ValueError(f"drag browser receipt is missing checks: {sorted(expected - names)}")
    for check in receipt["checks"]:
        if "pointerDownToUpMs" in check and (
                not isinstance(check["pointerDownToUpMs"], int)
                or not isinstance(check.get("releaseToDurableMs"), int)
                or check["pointerDownToUpMs"] < 0 or check["releaseToDurableMs"] < 0):
            raise ValueError(f"drag browser receipt has invalid timings: {check.get('name')}")


HARNESS_MANIFEST = ROOT / "tools/browser-harness/Cargo.toml"
HARNESS_BIN = ROOT / "tools/browser-harness/target/debug/drag-smoke"


def drag_driver() -> str:
    """Which runner drives the smoke: the legacy Node script or the Rust harness.

    The Rust path is opt-in until its parity gate passes; the old runner stays
    the default so other consumers and same-build comparisons keep working.
    """
    return os.environ.get("BOARDSTUDIO_DRAG_DRIVER", "node")


def build_harness() -> None:
    """Build the Rust drag smoke with the committed lockfile, under the build lock."""
    lock = Path.home() / ".cache/tmp/boardstudio-build.lock"
    command = ["flock", str(lock), "cargo", "build", "--locked",
               "--manifest-path", str(HARNESS_MANIFEST), "--bin", "drag-smoke"]
    built = subprocess.run(command, cwd=ROOT)
    if built.returncode:
        raise SystemExit(f"drag browser smoke cannot build the Rust harness ({' '.join(command)})")


def main() -> int:
    try:
        browser_tools.validate_drag_driver()
    except browser_tools.MissingBrowserTool as error:
        raise SystemExit(str(error))
    index = SITE / "site-subpath/boardstudio/index.html"
    if not index.is_file():
        raise SystemExit(f"drag browser smoke needs a built site; run `python3 scripts/check.py build` first ({index} missing)")
    try:
<<<<<<< HEAD
        if drag_driver() == "rust":
            chrome = browser_tools.locate_chrome()
            node = None
            build_harness()
        else:
            node, chrome = browser_tools.locate()
=======
        chrome = browser_tools.locate_chrome()
        binary = browser_tools.build_drag_harness()
>>>>>>> c1dbd3019 (tooling: default the drag smoke to the Rust harness, retire the Node path)
    except browser_tools.MissingBrowserTool as error:
        raise SystemExit(str(error))

    run_id = f"{datetime.now(timezone.utc):%Y%m%dT%H%M%SZ}-{uuid.uuid4().hex[:12]}"
    run_dir = RESULTS / run_id
    run_dir.mkdir(parents=True, exist_ok=False)
    browser_tmp = Path.home() / ".cache/tmp"
    browser_tmp.mkdir(parents=True, exist_ok=True)
    site_root = SITE / "site-subpath/boardstudio"
    build_files = [path for path in site_root.rglob("*") if path.is_file()]
    build_hash = digest_files(build_files, site_root)
    head, source_hash = source_identity()
    serve = module_at(ROOT / "scripts/serve-web.py", "boardstudio_serve_web")
    server = ThreadingHTTPServer(("127.0.0.1", 0), serve.handler_for(SITE))
    server.daemon_threads = True
    thread = threading.Thread(target=server.serve_forever, name="boardstudio-drag-smoke", daemon=True)
    thread.start()
    url = f"http://127.0.0.1:{server.server_address[1]}/boardstudio/"
    environment = os.environ.copy()
    environment.update({
        "CHROME_BIN": chrome,
        # Keep Chromium's short-lived profile on disk at a shallow path; deep paths exceed Unix socket limits.
        "TMPDIR": str(browser_tmp),
        "BOARDSTUDIO_PREVIEW_URL": url,
        "BOARDSTUDIO_DRAG_RECEIPT_PATH": str(run_dir / "receipt.json"),
        "BOARDSTUDIO_DRAG_SOURCE_HEAD": head,
        "BOARDSTUDIO_DRAG_SOURCE_SHA256": source_hash,
        "BOARDSTUDIO_DRAG_BUILD_SHA256": build_hash,
        "BOARDSTUDIO_DRAG_REPO_ROOT": str(ROOT),
<<<<<<< HEAD
        "BOARDSTUDIO_DRAG_DRIVER": drag_driver(),
=======
>>>>>>> c1dbd3019 (tooling: default the drag smoke to the Rust harness, retire the Node path)
    })
    try:
        with urlopen(url, timeout=5) as response:
            if response.status != 200:
                raise RuntimeError(f"preview returned HTTP {response.status}")
<<<<<<< HEAD
        if drag_driver() == "rust":
            command = [str(HARNESS_BIN)]
        else:
            assert node is not None
            command = [node, str(SCRIPT)]
=======
        command = [str(binary)]
>>>>>>> c1dbd3019 (tooling: default the drag smoke to the Rust harness, retire the Node path)
        run_log = run_dir / "run.log"
        with run_log.open("w") as output:
            process = subprocess.Popen(command, cwd=ROOT, env=environment, stdout=subprocess.PIPE,
                                       stderr=subprocess.STDOUT, text=True, bufsize=1)
            assert process.stdout is not None
            for line in process.stdout:
                output.write(line)
                output.flush()
                print(line, end="", flush=True)
            return_code = process.wait()
        if return_code == 0:
            try:
                verify_receipt(run_dir / "receipt.json", run_id, head, source_hash, build_hash)
            except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as error:
                message = f"Invalid drag browser receipt: {error}\n"
                with run_log.open("a") as output:
                    output.write(message)
                print(message, end="", file=sys.stderr)
                return_code = 1
        if return_code:
            print(f"drag browser smoke failed; log: {run_log}", file=sys.stderr)
        return return_code
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=3)


if __name__ == "__main__":
    raise SystemExit(main())
