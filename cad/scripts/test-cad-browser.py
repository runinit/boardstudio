#!/usr/bin/env python3
"""Run the real Monstertruck CAD WASM bindings in headless Chromium (needs `cad/wasm/pkg`, built by build-cad-wasm.py)."""
import functools
import http.server
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import sys
import tempfile
import threading
import time
import urllib.error
import urllib.request

CAD_ROOT = Path(__file__).resolve().parents[1]
PAGE = ("cad/scripts/browser-smoke/occurrence-budget.html" if "--occurrence-budget" in sys.argv
        else "cad/scripts/browser-smoke/smoke.html")


def chromium():
    for name in (os.environ.get("CHROME_BIN"), os.environ.get("CHROME"), "chromium", "chromium-browser", "google-chrome", "chrome"):
        if name and shutil.which(name):
            return shutil.which(name)
    raise RuntimeError("Chrome or Chromium is required (set CHROME_BIN to override)")


def chromedriver():
    for name in (os.environ.get("CHROMEDRIVER"), "chromedriver"):
        if name and shutil.which(name):
            return shutil.which(name)
    raise RuntimeError("ChromeDriver is required (set CHROMEDRIVER to override)")


def serve():
    class Quiet(http.server.SimpleHTTPRequestHandler):
        extensions_map = {**http.server.SimpleHTTPRequestHandler.extensions_map,
                          ".wasm": "application/wasm", ".js": "text/javascript"}

        def log_message(self, *args):
            pass

    server = http.server.ThreadingHTTPServer(
        ("127.0.0.1", 0), functools.partial(Quiet, directory=str(CAD_ROOT.parent)))
    threading.Thread(target=server.serve_forever, daemon=True).start()
    return server


def run_page(url, browser=None, timeout=180):
    deadline = time.monotonic() + timeout

    def request(port, method, path, payload=None, until=None):
        request_deadline = deadline if until is None else until
        remaining = request_deadline - time.monotonic()
        if remaining <= 0:
            raise TimeoutError("ChromeDriver smoke test timed out")
        data = None if payload is None else json.dumps(payload).encode()
        req = urllib.request.Request(
            f"http://127.0.0.1:{port}{path}", data=data, method=method,
            headers={"Content-Type": "application/json"})
        try:
            with urllib.request.urlopen(req, timeout=remaining) as response:
                body = response.read().decode()
        except urllib.error.HTTPError as error:
            body = error.read().decode(errors="replace")
            raise RuntimeError(f"ChromeDriver {method} {path} failed: {body}") from error
        result = json.loads(body) if body else {}
        value = result.get("value")
        if isinstance(value, dict) and value.get("error"):
            raise RuntimeError(f"ChromeDriver {value['error']}: {value.get('message', '')}")
        return result

    def profile_processes(profile):
        proc_root = Path("/proc")
        if not proc_root.exists():
            return None
        marker = f"--user-data-dir={profile}".encode()

        def is_owned(pid):
            try:
                return marker in Path(f"/proc/{pid}/cmdline").read_bytes().split(b"\0")
            except OSError:
                return False

        return [int(entry.name) for entry in proc_root.iterdir()
                if entry.name.isdigit() and is_owned(entry.name)]

    def profile_process_is_owned(pid, profile):
        marker = f"--user-data-dir={profile}".encode()
        try:
            return marker in Path(f"/proc/{pid}/cmdline").read_bytes().split(b"\0")
        except OSError:
            return False

    with tempfile.TemporaryDirectory(prefix="cad-browser-") as owned_dir:
        profile = str(Path(owned_dir) / "chrome-profile")
        Path(profile).mkdir()
        log_path = Path(owned_dir) / "chromedriver.log"
        with socket.socket() as listener:
            listener.bind(("127.0.0.1", 0))
            port = listener.getsockname()[1]
        driver = subprocess.Popen(
            [chromedriver(), f"--port={port}", f"--log-path={log_path}"],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        session_id = None
        primary_error = None
        try:
            while True:
                if driver.poll() is not None:
                    raise RuntimeError(f"ChromeDriver exited with status {driver.returncode}")
                try:
                    request(port, "GET", "/status")
                    break
                except (OSError, urllib.error.URLError, TimeoutError):
                    if time.monotonic() >= deadline:
                        raise TimeoutError("ChromeDriver did not become ready")
                    time.sleep(0.05)

            result = request(port, "POST", "/session", {
                "capabilities": {"alwaysMatch": {
                    "browserName": "chrome",
                    "goog:chromeOptions": {
                        "binary": browser or chromium(),
                        "args": ["--headless=new", "--no-sandbox", "--disable-gpu",
                                 f"--user-data-dir={profile}"]}}}})
            session_id = result["value"]["sessionId"]
            base = f"/session/{session_id}"
            page_timeout = max(1, int((deadline - time.monotonic()) * 1000))
            request(port, "POST", f"{base}/timeouts", {
                "pageLoad": page_timeout, "script": page_timeout, "implicit": 0})
            request(port, "POST", f"{base}/url", {"url": url})

            pending = False
            while True:
                try:
                    value = request(port, "POST", f"{base}/execute/sync", {
                        "script": "const result = document.getElementById('result'); "
                                  "return result ? result.textContent : null;", "args": []})["value"]
                except TimeoutError as error:
                    if pending:
                        raise RuntimeError("smoke page never finished") from error
                    raise
                if value is None:
                    raise RuntimeError("smoke page did not render a result")
                if value != "pending":
                    return request(port, "POST", f"{base}/execute/sync", {
                        "script": "return document.documentElement.outerHTML;", "args": []})["value"]
                pending = True
                if time.monotonic() >= deadline:
                    raise TimeoutError("smoke page never finished")
                time.sleep(min(0.1, max(0, deadline - time.monotonic())))
        except Exception as error:
            primary_error = error
            if log_path.exists():
                details = log_path.read_text(errors="replace").splitlines()[-30:]
                if details:
                    error.add_note("ChromeDriver log:\n" + "\n".join(details))
            raise
        finally:
            active_error = primary_error or sys.exc_info()[1]
            cleanup_errors = []

            def driver_running():
                try:
                    return driver.poll() is None
                except Exception as error:
                    if not any(item.startswith("could not check ChromeDriver")
                               for item in cleanup_errors):
                        cleanup_errors.append(f"could not check ChromeDriver process: {error}")
                    return True

            if session_id is not None:
                try:
                    request(port, "DELETE", f"/session/{session_id}",
                            until=time.monotonic() + 5)
                except Exception as error:
                    cleanup_errors.append(f"session deletion failed: {error}")

            cleanup_deadline = time.monotonic() + 4

            def refresh_owned(pids):
                try:
                    discovered = profile_processes(profile)
                except Exception as error:
                    cleanup_errors.append(f"could not inspect owned Chrome processes: {error}")
                    return pids, set()
                if discovered is None:
                    return pids, set()
                discovered = set(discovered)
                return pids | discovered, discovered - pids

            def signal_owned(pids, signal_number, verb):
                for pid in pids:
                    if not profile_process_is_owned(pid, profile):
                        continue
                    try:
                        os.kill(pid, signal_number)
                    except ProcessLookupError:
                        pass
                    except OSError as error:
                        cleanup_errors.append(f"could not {verb} owned Chrome process {pid}: {error}")

            owned, _ = refresh_owned(set())
            signal_owned(owned, 15, "stop")
            if driver_running():
                try:
                    driver.terminate()
                except ProcessLookupError:
                    pass
                except OSError as error:
                    cleanup_errors.append(f"could not stop ChromeDriver: {error}")

            term_deadline = min(cleanup_deadline, time.monotonic() + 1)
            while time.monotonic() < term_deadline:
                owned, discovered = refresh_owned(owned)
                signal_owned(discovered, 15, "stop")
                owned = {pid for pid in owned if profile_process_is_owned(pid, profile)}
                if not driver_running() and not owned:
                    break
                time.sleep(min(0.05, max(0, term_deadline - time.monotonic())))

            owned, discovered = refresh_owned(owned)
            signal_owned(discovered, 9, "kill")
            if driver_running():
                try:
                    driver.kill()
                except ProcessLookupError:
                    pass
                except OSError as error:
                    cleanup_errors.append(f"could not kill ChromeDriver: {error}")
            signal_owned(owned, 9, "kill")

            while time.monotonic() < cleanup_deadline:
                owned, discovered = refresh_owned(owned)
                signal_owned(discovered, 9, "kill")
                owned = {pid for pid in owned if profile_process_is_owned(pid, profile)}
                if not driver_running() and not owned:
                    break
                time.sleep(min(0.05, max(0, cleanup_deadline - time.monotonic())))
            owned, _ = refresh_owned(owned)
            owned = {pid for pid in owned if profile_process_is_owned(pid, profile)}
            if driver_running():
                cleanup_errors.append("ChromeDriver did not exit before cleanup deadline")
            if owned:
                cleanup_errors.append(
                    f"owned Chrome processes remained after cleanup deadline: {owned}")
            if cleanup_errors:
                message = "cleanup incomplete: " + "; ".join(cleanup_errors)
                if active_error is None:
                    raise RuntimeError(message)
                active_error.add_note(message)


def parse_result(dom):
    marker = '<pre id="result">'
    start = dom.find(marker)
    if start < 0:
        raise RuntimeError("smoke page did not render a result")
    text = dom[start + len(marker):dom.index("</pre>", start)]
    text = text.replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&")
    if text == "pending":
        raise RuntimeError("smoke page never finished")
    return json.loads(text)


def main():
    if not (CAD_ROOT / "wasm/pkg/boardstudio_cad_wasm_bg.wasm").exists():
        sys.exit("cad/wasm/pkg is missing; run python3 cad/scripts/build-cad-wasm.py first")
    server = serve()
    try:
        done = run_page(f"http://127.0.0.1:{server.server_address[1]}/{PAGE}")
    finally:
        server.shutdown()
    result = parse_result(done)
    for item in result["checks"]:
        print(("ok   " if item["ok"] else "FAIL ") + item["name"] + (f" ({item['detail']})" if not item["ok"] and item["detail"] else ""))
    if result["error"]:
        print("page error:", result["error"])
    if not result["ok"]:
        sys.exit(1)
    print(f"{len(result['checks'])} browser checks passed")


if __name__ == "__main__":
    main()
