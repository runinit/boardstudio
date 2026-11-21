"""Shared stdlib ChromeDriver client and loopback server for browser checks."""

from __future__ import annotations

import json
import os
from pathlib import Path
import shutil
import signal
import socket
import subprocess
import sys
import tempfile
import threading
import time
from contextlib import contextmanager
from http.server import ThreadingHTTPServer
from urllib.error import HTTPError, URLError
from urllib.request import ProxyHandler, Request, build_opener


MAX_BROWSER_TMPDIR_BYTES = 45


def shallow_browser_tmpdir() -> Path:
    """Choose a root short enough for nested Chrome Unix sockets."""
    candidate = Path.home() / ".cache/tmp"
    if len(os.fsencode(candidate)) > MAX_BROWSER_TMPDIR_BYTES:
        candidate = Path("/tmp")
    candidate.mkdir(parents=True, exist_ok=True)
    return candidate


def browser_env(base_env=None, *, chrome=None, **extra) -> dict[str, str]:
    environment = dict(os.environ if base_env is None else base_env)
    environment["TMPDIR"] = str(shallow_browser_tmpdir())
    environment["CHROME_BIN"] = str(chrome or locate_chrome(environment))
    environment.update({key: str(value) for key, value in extra.items()})
    return environment


class MissingBrowserTool(RuntimeError):
    """A prerequisite of the real-page smoke is absent; the message says how to supply it."""


CFT_PIN_JSON = Path(__file__).with_name("chrome-for-testing.json")
CHROME_BINARIES = ("google-chrome-stable", "google-chrome", "chromium", "chromium-browser", "chrome")


def pinned_cft_dir(env=None):
    env = os.environ if env is None else env
    if override := env.get("BOARDSTUDIO_CFT_DIR"):
        return Path(override)
    try:
        version = json.loads(CFT_PIN_JSON.read_text())["version"]
    except (OSError, ValueError, KeyError, TypeError):
        return None
    return Path.home() / ".cache/boardstudio/chrome-for-testing" / version if version else None


def _locate(kind, override, names, env):
    base = pinned_cft_dir(env)
    pinned = (str(base / f"{kind}-linux64" / kind),) if base else ()
    candidates = (override,) if override else (*pinned, *names)
    for name in candidates:
        if binary := shutil.which(name, path=env.get("PATH")):
            return str(Path(binary).resolve())
    variable = "CHROME_BIN" if kind == "chrome" else "CHROMEDRIVER"
    raise MissingBrowserTool(f"{kind} executable unavailable; set {variable} to a matching Chrome/ChromeDriver pair")


def locate_chrome(env=None):
    env = os.environ if env is None else env
    override = env.get("CHROME_BIN") or env.get("BOARDSTUDIO_DRAG_CHROME_BIN") or env.get("CHROME")
    return _locate("chrome", override, CHROME_BINARIES, env)


def locate_chromedriver(env=None):
    env = os.environ if env is None else env
    return _locate("chromedriver", env.get("CHROMEDRIVER"), ("chromedriver",), env)


def _inherited_build_lock() -> int | None:
    """Validate and hold the explicitly inherited shared build lock, if supplied."""
    value = os.environ.get("BOARDSTUDIO_BUILD_LOCK_FD")
    if value is None:
        return None
    import fcntl

    lock = Path.home() / ".cache/tmp/boardstudio-build.lock"
    try:
        fd = int(value)
        stat = os.fstat(fd)
        path_stat = lock.stat()
        if fd < 3 or (stat.st_dev, stat.st_ino) != (path_stat.st_dev, path_stat.st_ino):
            raise ValueError("descriptor does not refer to the shared lock")
        fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except (ValueError, OSError) as error:
        raise SystemExit(f"invalid BOARDSTUDIO_BUILD_LOCK_FD: {error}") from error
    return fd


@contextmanager
def serve(handler):
    """Serve on an ephemeral loopback port until the browser check finishes."""
    with ThreadingHTTPServer(("127.0.0.1", 0), handler) as server:
        thread = threading.Thread(target=server.serve_forever, kwargs={"poll_interval": 0.05})
        thread.start()
        try:
            yield f"http://127.0.0.1:{server.server_port}"
        finally:
            server.shutdown()
            thread.join()


def _exit_on_sigterm(signum, _frame):
    # The driver runs in a separate group, so unwind the owning runner on cancellation.
    raise SystemExit(128 + signum)


class WebDriverSession:
    """Own a ChromeDriver process group, a throwaway profile, and one W3C session."""

    def __init__(self, *, chrome=None, driver=None, timeout=180, window_size=(1513, 1043)):
        self.chrome = str(chrome or locate_chrome())
        self.driver_binary = str(driver or locate_chromedriver())
        self.timeout = timeout
        self.window_size = window_size
        self.driver = None
        self.session_id = None
        self.previous_sigterm = None
        self.opener = build_opener(ProxyHandler({}))  # Local commands must bypass environment proxies.

    def __enter__(self):
        self.deadline = time.monotonic() + self.timeout
        self.directory = tempfile.TemporaryDirectory(prefix="bs-wd-", dir=shallow_browser_tmpdir())
        try:
            if threading.current_thread() is threading.main_thread():
                self.previous_sigterm = signal.signal(signal.SIGTERM, _exit_on_sigterm)
            profile = Path(self.directory.name) / "profile"
            log = Path(self.directory.name) / "driver.log"
            with socket.socket() as listener:
                listener.bind(("127.0.0.1", 0))
                port = listener.getsockname()[1]
            self.endpoint = f"http://127.0.0.1:{port}"
            self.driver = subprocess.Popen(
                [self.driver_binary, f"--port={port}", "--allowed-ips=127.0.0.1", f"--log-path={log}"],
                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, start_new_session=True,
                env=browser_env(chrome=self.chrome),
            )
            startup = min(self.deadline, time.monotonic() + 30)
            while True:
                if self.driver.poll() is not None:
                    raise RuntimeError(f"ChromeDriver exited with status {self.driver.returncode}")
                try:
                    if self._request("GET", "/status", until=startup)["ready"]:
                        break
                except URLError:
                    pass
                if time.monotonic() >= startup:
                    raise TimeoutError("ChromeDriver did not become ready")
                time.sleep(0.05)
            width, height = self.window_size
            session = self._request("POST", "/session", {"capabilities": {"alwaysMatch": {
                "browserName": "chrome",
                "goog:loggingPrefs": {"browser": "ALL"},
                "goog:chromeOptions": {
                    "binary": self.chrome,
                    "args": ["--headless=new", "--no-sandbox", "--disable-dev-shm-usage",
                             "--use-angle=swiftshader", "--enable-unsafe-swiftshader",
                             f"--window-size={width},{height}", f"--user-data-dir={profile}"],
                },
            }}}, until=startup)
            self.session_id = session["sessionId"]
            self.command("POST", "/timeouts", {"implicit": 0, "pageLoad": 60_000, "script": 30_000})
            return self
        except BaseException as error:
            self.__exit__(type(error), error, error.__traceback__)
            raise

    def __exit__(self, _kind, error, _traceback):
        try:
            if self.session_id:
                try:
                    self._request("DELETE", f"/session/{self.session_id}", until=time.monotonic() + 5)
                except (OSError, RuntimeError) as cleanup:
                    message = f"ChromeDriver session deletion: {cleanup}"
                    if error is not None:
                        error.add_note(message)
                    else:
                        print(f"warning: {message}", file=sys.stderr)
        finally:
            try:
                if self.driver:
                    # Chrome children inherit this dedicated group, including during a failed launch.
                    for sig in (signal.SIGTERM, signal.SIGKILL):
                        try:
                            os.killpg(self.driver.pid, sig)
                        except ProcessLookupError:
                            break
                        if sig == signal.SIGTERM:
                            try:
                                self.driver.wait(timeout=1)
                            except subprocess.TimeoutExpired:
                                pass
                    self.driver.wait(timeout=5)
            finally:
                try:
                    self.directory.cleanup()
                finally:
                    if self.previous_sigterm is not None:
                        signal.signal(signal.SIGTERM, self.previous_sigterm)

    def _request(self, method, path, payload=None, *, until=None):
        remaining = (self.deadline if until is None else until) - time.monotonic()
        if remaining <= 0:
            raise TimeoutError(f"ChromeDriver {method} {path} timed out")
        data = None if payload is None else json.dumps(payload, allow_nan=False).encode()
        request = Request(self.endpoint + path, data=data, method=method,
                          headers={"Content-Type": "application/json"})
        try:
            with self.opener.open(request, timeout=remaining) as response:
                body = json.load(response)
        except HTTPError as error:
            with error:
                try:
                    value = json.load(error).get("value", {})
                except (ValueError, AttributeError):
                    value = {}
            message = value.get("message", str(error)).split("\nStacktrace:", 1)[0]
            raise RuntimeError(f"ChromeDriver {method} {path}: {message}") from error
        return body["value"]

    def command(self, method, path, payload=None):
        return self._request(method, f"/session/{self.session_id}{path}", payload)

    def get(self, url):
        self.command("POST", "/url", {"url": url})

    def execute_sync(self, js, *args):
        return self.command("POST", "/execute/sync", {"script": js, "args": list(args)})

    def execute_async(self, js, *args, timeout=30):
        self.command("POST", "/timeouts", {"script": int(timeout * 1000)})
        return self.command("POST", "/execute/async", {"script": js, "args": list(args)})

    def actions(self, actions_payload):
        self.command("POST", "/actions", actions_payload)

    def check_page_errors(self):
        entries = self.command("POST", "/log", {"type": "browser"})
        errors = [entry["message"] for entry in entries if entry["level"] == "SEVERE"
                  and not (entry["source"] == "network" and "/favicon.ico " in entry["message"])]
        if errors:
            raise RuntimeError("browser errors:\n" + "\n".join(errors))


def run_provider_page(url, suite, *, timeout=300, chrome=None):
    """Run the page's /run.js provider and return its named JSON results."""
    suites = [suite] if isinstance(suite, str) else list(suite)
    if not suites or len(suites) != len(set(suites)):
        raise ValueError("expected distinct provider suites")
    results = {}
    failures = []
    with WebDriverSession(chrome=chrome, timeout=timeout) as browser:
        for name in suites:
            try:
                browser.get(url)
                result = browser.execute_async("""
                    const [url, name, done] = arguments;
                    (async () => {
                        if (location.href !== url) throw Error('unexpected page URL: ' + location.href);
                        const {run} = await import('/run.js');
                        return await run(name);
                    })().then(value => done({value}), error => done({error: error.stack || String(error)}));
                    """, url, name, timeout=min(timeout, 300))
                if "error" in result:
                    failures.append(f"{name}: {result['error']}")
                else:
                    results[name] = result["value"]
                    for entry in result["value"] if isinstance(result["value"], list) else []:
                        if isinstance(entry, dict) and entry.get("ok") is False:
                            failures.append(f"{name}/{entry.get('case')}: {entry.get('error')}")
                browser.check_page_errors()
            except (OSError, RuntimeError) as error:
                failures.append(f"{name}: {error}")
        if failures:
            raise RuntimeError("\n".join(failures))
    return results
