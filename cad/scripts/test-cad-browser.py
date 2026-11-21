#!/usr/bin/env python3
"""Run the real CAD WASM bindings in headless Chrome (requires cad/wasm/pkg)."""
from functools import partial
from http.server import SimpleHTTPRequestHandler
import json
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "scripts"))
import browser_tools  # noqa: E402

CAD_ROOT = Path(__file__).resolve().parents[1]


class Handler(SimpleHTTPRequestHandler):
    extensions_map = {**SimpleHTTPRequestHandler.extensions_map,
                      ".wasm": "application/wasm", ".js": "text/javascript"}

    def log_message(self, *_args):
        pass


def run_page(url, browser=None, timeout=180):
    with browser_tools.WebDriverSession(chrome=browser, timeout=timeout) as session:
        session.get(url)
        result = session.execute_async("""
            const done = arguments[0];
            const result = document.getElementById('result');
            if (!result || result.textContent !== 'pending') {
                done(result?.textContent);
                return;
            }
            const observer = new MutationObserver(() => {
                if (result.textContent === 'pending') return;
                observer.disconnect();
                done(result.textContent);
            });
            observer.observe(result, {childList: true, characterData: true, subtree: true});
            """, timeout=timeout - 1)
        session.check_page_errors()
        if result is None:
            raise RuntimeError("smoke page did not render a result")
        return json.loads(result)


def main():
    if not (CAD_ROOT / "wasm/pkg/boardstudio_cad_wasm_bg.wasm").is_file():
        raise RuntimeError("cad/wasm/pkg is missing; run python3 cad/scripts/build-cad-wasm.py first")
    if not (CAD_ROOT.parent / "web/target/providers/cad-worker/m1_cad_worker.js").is_file():
        raise RuntimeError("CAD worker package is missing from web/target/providers/cad-worker")
    page = "occurrence-budget.html" if "--occurrence-budget" in sys.argv else "smoke.html"
    with browser_tools.serve(partial(Handler, directory=str(CAD_ROOT.parent))) as url:
        result = run_page(f"{url}/cad/scripts/browser-smoke/{page}")
    for item in result["checks"]:
        detail = f" ({item['detail']})" if not item["ok"] and item["detail"] else ""
        print(("ok   " if item["ok"] else "FAIL ") + item["name"] + detail)
    if result["error"]:
        print("page error:", result["error"])
    return int(bool(result["error"]) or not result["checks"] or not all(item["ok"] for item in result["checks"]))


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, RuntimeError, ValueError) as error:
        sys.exit(str(error))
