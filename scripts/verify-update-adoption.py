#!/usr/bin/env python3
"""Packaged A/B adoption proof for ticket 019 (Update Now through protected Refresh).

Extends the 018 waiting-update driver (scripts/verify-waiting-update.py). A real
packaged build A is served at one origin and installed in an isolated browser
profile; a real packaged build B is then published at the SAME origin so A sees
a genuine waiting service worker. The driver clicks the production **Update Now**
control with real input events and proves the loaded application is B, not just
that a reload happened.

Packaged identities (so the A/B claim is checkable):
  * `--src-dir` is one real `scripts/build-web.py --routes root,subpath` output.
  * A and B are stamped copies of it: each copy's `index.html` receives a
    `<meta name="boardstudio-build" content="A|B">`, its page application binary
    `wasm/boardstudio-web_bg.wasm` receives a trailing wasm custom section
    `boardstudio-build<A|B>` (a valid, ignored section, so the application runs
    unchanged but the two binaries genuinely differ), and the production
    `embed_offline_worker` re-embeds the service-worker bootstrap, so each copy
    carries its own versioned manifest `boardstudio-<sha256[:16]>-<time_ns>`.
    The packaged diff between A and B is therefore exactly `index.html`,
    `service-worker.js` and the page wasm per route; the driver asserts that and
    logs each file's sha256. Nothing else is claimed to differ.
  * Proof that B is the loaded application: the loaded document's own meta
    reads `B`, the application binary served to the page carries B's custom
    section, the controlling worker's cache stores B's bootstrap text, only
    B's versioned cache survives activation, and the page came from a reload
    (the in-page load marker is gone, navigation type `reload`).
    Residual limitation (as in 018): no web API exposes a worker's own version,
    so worker attribution chains install chronology (a fresh profile admits
    exactly one install per step) with the production cache bytes.

Modes:
  pre    RED control. A real pre-019 packaged pair (`--legacy-a-dir`/`--legacy-b-dir`,
         the 018 builds) must offer the Update Now action; it does not, so the
         driver fails. Control only: no 019 code exists in those builds.
  post   GREEN. Runs every leg (`--legs` selects a subset):
           adopt             sole editor: toast, dismissal, header Update Now,
                             adoption to B, saved project reopened.
           second-editor     a second A editor defers adoption (cancel-only
                             warning, nothing forced); its uncached-asset
                             request is served offline from A's cache; adoption
                             completes once it is closed.
           subpath           root and /boardstudio/ registrations stay isolated
                             while each adopts independently.
           failed-install    broken B (script error / missing asset) never shows
                             the notice or Update Now, never reloads, and a
                             repaired B recovers.
           unavailable-target a waiting worker that cannot take over reports,
                             does not reload, keeps Update Now, and a repaired
                             B adopts.
           late-adopt        the waiting WORKER is the authority: an adoption
                             request posted straight to it while a second editor
                             is open (a probe-then-skipWaiting race) is
                             declined, nothing activates, no cache is lost and
                             the second editor stays offline-capable.
           draft-safety      a real Board name draft plus an invalid field
                             defers Update Now (cancel-only, keyboard focus
                             returns), then after the field is fixed and the
                             work saves, adoption reloads onto B and the
                             renamed board is still saved.
           keyboard-matrix   real Tab / Enter / Space (trusted key events) reach
                             and drive Update Now and the warning's Cancel
                             across Theme x Appearance, narrow viewport and
                             reduced motion, with layout and contrast probes.

Every command and every raw browser observation is echoed; stdout is the
complete evidence log (redirect it to .evidence/). The driver owns its server
processes, refuses to start when its port is occupied, and verifies the port
is free afterwards. It never touches foreign processes.

Generated outputs and exit status (post mode): the stamped A/B copies, worker copies and
broken variants live in one output workspace this run creates (`--work-dir`, a path that
does not exist yet, or a new directory under web/target); leg fixtures, exports and browser
profiles live in private directories made by `make_session_dir`/`start_session`. An existing,
symlinked, input-overlapping or unusable workspace is refused (exit 2) before anything is
written. The run deletes only the directories it created: on completion and after caught
failures once the browser close is confirmed and no Chrome uses the exact profile; otherwise
it retains them, prints `RETAINED` paths and fails. It never deletes `--src-dir`, the legacy
packages, caches or logs. Exit 0 means every assertion held AND every output was removed;
`--keep-artifacts` keeps outputs for diagnosis, prints `DIAGNOSTIC ONLY` paths and exits 3,
never 0. A SIGKILLed run is not recovered: remove its leftovers by hand.
"""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import os
import re
import shutil
import socket
import subprocess
import sys
import tempfile
import time
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PAYLOADS = Path(__file__).resolve().parent / "verify-update-adoption-payloads"
SERVE = [sys.executable, str(ROOT / "scripts" / "serve-web.py")]
VERSION_RE = re.compile(r"boardstudio-[0-9a-f]{16}-[0-9]+")
ROUTES = {"root": ("/", "site-root"), "subpath": ("/boardstudio/", "site-subpath/boardstudio")}
CACHE_PREFIX = "boardstudio-m1-offline-path-"
ALL_LEGS = ["adopt", "second-editor", "subpath", "failed-install", "unavailable-target",
            "late-adopt", "draft-safety", "keyboard-matrix"]
UPDATE_NOW_TOAST = "[data-slot=toast-provider] [aria-label='Update Now']"
UPDATE_NOW_HEADER = ".header-update [aria-label='Update Now']"
# rustc folds short constant string compares into integer compares, so the message
# literal never appears in the wasm; the adoption path is identified by the
# service-worker import it adds (skipWaiting), absent from every pre-019 worker.
ADOPT_IMPORT = b"skipWaiting"


class Failure(Exception):
    pass


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def cache_name(route: str, version: str) -> str:
    scope = ROUTES[route][0]
    return f"{CACHE_PREFIX}{scope.encode().hex()}-{version}"


def site_dir(build: Path, route: str) -> Path:
    return build / ROUTES[route][1]


def worker_version(build: Path, route: str) -> str:
    text = (site_dir(build, route) / "service-worker.js").read_text()
    match = re.search(r'start_offline_worker\("([^"]+)"', text)
    if not match:
        raise Failure(f"no worker version in {site_dir(build, route)}")
    return match.group(1)


def route_url(port: int, route: str) -> str:
    return f"http://127.0.0.1:{port}{ROUTES[route][0]}"


class Driver:
    def __init__(self, port: int, log, keep_artifacts: bool = False):
        self.port = port
        self.log = log
        # --keep-artifacts: owned outputs are kept (after the same close checks) for
        # diagnosis; the run can then never be an acceptance pass.
        self.keep_artifacts = keep_artifacts
        self.kept: list[Path] = []
        self.failures: list[str] = []
        self.env = dict(os.environ)
        self.session_closed = True
        self.close_attempted = False
        # Directory this driver created for a profile-less session, and the profile
        # the live session was started on; cleanup deletes nothing else.
        self.session_dir: Path | None = None
        self.owned_profile: Path | None = None

    def cmd(self, text: str) -> None:
        print(f"--- cmd: {text} ---", file=self.log, flush=True)

    def note(self, text: str) -> None:
        print(f"NOTE: {text}", file=self.log, flush=True)

    def run(self, args: list[str], timeout: int = 120, ok: bool = False) -> str:
        self.cmd("$ " + " ".join(args))
        done = subprocess.run(args, cwd=ROOT, env=self.env, capture_output=True, text=True,
                              timeout=timeout)
        out = (done.stdout or "") + (done.stderr or "")
        print(out, file=self.log, flush=True)
        if done.returncode != 0 and not ok:
            self.fail(f"command failed rc={done.returncode}: {' '.join(args)}", out.strip()[:500])
        return out

    def fail(self, name: str, detail: str = "") -> None:
        print(f"FAIL: {name}" + (f" | {detail}" if detail else ""), file=self.log, flush=True)
        self.failures.append(name)

    def check(self, name: str, cond: bool, detail: str = "") -> None:
        print(f"{'PASS' if cond else 'FAIL'}: {name}" + (f" | {detail}" if detail else ""),
              file=self.log, flush=True)
        if not cond:
            self.failures.append(name)

    def snapshot(self) -> str:
        self.cmd("$ agent-browser snapshot -i")
        done = subprocess.run(["agent-browser", "snapshot", "-i"], cwd=ROOT, env=self.env,
                              capture_output=True, text=True, timeout=120)
        print(done.stdout or "", file=self.log, flush=True)
        if done.returncode != 0:
            self.fail("snapshot failed", (done.stderr or "")[:300])
        return done.stdout or ""

    def click_button(self, name: str) -> None:
        """Snapshot, resolve a ref, click. Refs go stale when the page re-renders
        between snapshot and click, so retry with a fresh snapshot before failing."""
        self.cmd(f'$ snapshot + click button "{name}"')
        last = ""
        for attempt in range(1, 4):
            ref = None
            for line in self.snapshot().splitlines():
                match = re.search(r'button "' + re.escape(name) + r'"[^\n]*?ref=(e\d+)', line)
                if match:
                    ref = "@" + match.group(1)
                    break
            self.cmd(f"ref (attempt {attempt}): {ref}")
            if ref is not None:
                last = self.run(["agent-browser", "click", ref], ok=True)
                if "✗" not in last:
                    return
            time.sleep(1.5)
        self.fail(f"button could not be clicked: {name}", last.strip()[:200])
        raise Failure(f"no usable ref for {name}")

    def click_selector(self, selector: str) -> None:
        self.run(["agent-browser", "click", selector])

    def eval_file(self, payload: str):
        path = PAYLOADS / payload
        self.cmd(f"$ agent-browser eval --stdin < {path}")
        print(f"--- payload {path} ---", file=self.log, flush=True)
        print(path.read_text(), file=self.log, flush=True)
        print("--- output ---", file=self.log, flush=True)
        with open(path) as handle:
            done = subprocess.run(["agent-browser", "eval", "--stdin"], cwd=ROOT, env=self.env,
                                  stdin=handle, capture_output=True, text=True, timeout=180)
        print(done.stdout or "", file=self.log, flush=True)
        if done.stderr:
            print(done.stderr, file=self.log, flush=True)
        try:
            return json.loads(done.stdout)
        except (json.JSONDecodeError, ValueError):
            self.fail(f"eval output not JSON: {payload}", (done.stdout or "")[:300])
            raise Failure(f"bad eval output: {payload}")

    def eval_expr(self, expr: str) -> str:
        done = subprocess.run(["agent-browser", "eval", expr], cwd=ROOT, env=self.env,
                              capture_output=True, text=True, timeout=60)
        return ((done.stdout or "") + (done.stderr or "")).strip()

    def poll(self, label: str, expr: str, timeout_s: int = 60) -> bool:
        """Poll a JS boolean expression until truthy; tolerant of navigation gaps."""
        self.cmd(f"poll until {label}: {expr}")
        deadline = time.time() + timeout_s
        last = ""
        while time.time() < deadline:
            last = self.eval_expr(expr)
            if last == "true":
                print(f"poll ok: {label}", file=self.log, flush=True)
                return True
            time.sleep(0.5)
        print(f"poll timed out: {label} (last={last!r})", file=self.log, flush=True)
        return False

    def state(self) -> dict:
        return self.eval_file("state.js")

    # Tabs -------------------------------------------------------------
    def press(self, key: str) -> None:
        self.run(["agent-browser", "press", key])
        time.sleep(0.4)

    def tab_new(self, label: str, url: str) -> None:
        self.run(["agent-browser", "tab", "new", "--label", label, url], timeout=90)

    def tab_switch(self, label: str) -> None:
        self.run(["agent-browser", "tab", label])

    def tab_close(self, label: str) -> None:
        self.run(["agent-browser", "tab", "close", label])


def workspace_refusal(work_dir: Path | None, protected: list[Path]) -> str:
    """Why this run must not create its output workspace, or "" when it may.

    The workspace is created and deleted by this command, so it must not exist yet (not
    even as an empty directory or a symlink), must sit in an existing real directory and
    must not be inside an input or the evidence-log tree. Nothing is written or deleted
    to make a path acceptable.
    """
    if work_dir is None:
        parent = ROOT / "web" / "target"
    else:
        path = work_dir if work_dir.is_absolute() else Path.cwd() / work_dir
        if ".." in path.parts:
            return f"refusing output workspace {work_dir}: path contains '..'"
        if os.path.lexists(path):
            kind = "symbolic link" if path.is_symlink() else "directory" if path.is_dir() else "file"
            return (f"refusing output workspace {path}: already exists ({kind}); "
                    "it must be a new path this command creates and removes")
        parent = path.parent
    try:
        real_parent = parent.resolve(strict=True)
    except OSError:
        real_parent = None
    if real_parent is None or not real_parent.is_dir():
        return f"refusing output workspace: parent {parent} is not an existing directory"
    for guarded in protected:
        real = guarded.resolve()
        if real_parent == real or real in real_parent.parents:
            return f"refusing output workspace: it would overlap {guarded}"
    return ""


def create_workspace(work_dir: Path | None) -> Path:
    """Create the fresh workspace; the returned real path is the only tree this run deletes."""
    if work_dir is None:
        return Path(tempfile.mkdtemp(prefix="site-019-ab-", dir=ROOT / "web" / "target")).resolve()
    path = work_dir if work_dir.is_absolute() else Path.cwd() / work_dir
    workspace = path.parent.resolve(strict=True) / path.name
    workspace.mkdir()
    return workspace


def port_free(port: int) -> bool:
    with socket.socket() as sock:
        sock.settimeout(2)
        return sock.connect_ex(("127.0.0.1", port)) != 0


def serve_version(port: int, route: str = "root") -> str:
    out = subprocess.run(["curl", "-s", f"http://127.0.0.1:{port}{ROUTES[route][0]}service-worker.js"],
                         capture_output=True, text=True, timeout=30).stdout
    match = VERSION_RE.search(out)
    if not match:
        raise Failure(f"no release version served on {port} ({route})")
    return match.group(0)


class Server:
    """Owned test server: started by this driver, stopped by this driver."""

    def __init__(self, build_dir: Path, port: int, log_path: Path):
        self.build_dir = build_dir
        self.port = port
        self.proc: subprocess.Popen | None = None
        self.log_path = log_path

    def start(self) -> None:
        log = open(self.log_path, "a")
        self.proc = subprocess.Popen([*SERVE, str(self.build_dir), str(self.port),
                                      "--host", "127.0.0.1"],
                                     cwd=ROOT, stdout=log, stderr=subprocess.STDOUT)
        deadline = time.time() + 30
        while time.time() < deadline:
            try:
                serve_version(self.port)
                return
            except Failure:
                time.sleep(1)
        raise Failure(f"server did not come up: {self.build_dir}")

    def stop(self) -> None:
        if self.proc is not None:
            self.proc.terminate()
            try:
                self.proc.wait(timeout=15)
            except subprocess.TimeoutExpired:
                self.proc.kill()
            self.proc = None

    def swap(self, driver: Driver, build_dir: Path, why: str) -> None:
        driver.cmd(f"origin swap ({why}): stop server, serve {build_dir}")
        self.stop()
        self.build_dir = build_dir
        # Fresh mtimes on the two files that differ between releases. The test
        # server sends no Cache-Control, so the browser applies heuristic
        # freshness (10% of Date - Last-Modified) to them; a release served with
        # Last-Modified == now is never fresh, so every later fetch of the new
        # release revalidates and receives its own bytes (a deployment with
        # correct revalidation headers). This also stops poisoned script entries
        # from revalidating 304.
        for route in ROUTES:
            for name in ("service-worker.js", "index.html", "wasm/boardstudio-web_bg.wasm"):
                target = site_dir(build_dir, route) / name
                if target.is_file():
                    os.utime(target, None)
        self.start()


# ---------------------------------------------------------------------------
# Packaging: stamped A/B copies of one real build.
# ---------------------------------------------------------------------------

def load_build_web():
    sys.path.insert(0, str(ROOT / "scripts"))
    spec = importlib.util.spec_from_file_location("build_web", ROOT / "scripts" / "build-web.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def leb128(value: int) -> bytes:
    out = bytearray()
    while True:
        byte = value & 0x7F
        value >>= 7
        if value:
            out.append(byte | 0x80)
        else:
            out.append(byte)
            return bytes(out)


def stamp_wasm(wasm: Path, label: str) -> None:
    """Append a custom section naming the build. Custom sections are ignored by the
    engine, so the application behaves identically but the binary's bytes differ."""
    name = b"boardstudio-build"
    payload = leb128(len(name)) + name + label.encode()
    with wasm.open("ab") as handle:
        handle.write(b"\x00" + leb128(len(payload)) + payload)


def extract_worker(site: Path, into: Path) -> Path:
    """The offline worker packaged inside `site`, as a worker directory. Stamping
    must re-embed exactly the worker the source build shipped, never whatever the
    shared providers directory holds now (a later build may have replaced it)."""
    import base64
    text = (site / "service-worker.js").read_text()
    match = re.search(r'atob\("([^"]+)"\)', text)
    if not match:
        raise Failure(f"no embedded worker wasm in {site / 'service-worker.js'}")
    into.mkdir(parents=True, exist_ok=True)
    (into / "boardstudio_offline_worker_bg.wasm").write_bytes(base64.b64decode(match.group(1)))
    shutil.copy2(site / "boardstudio_offline_worker.js", into / "boardstudio_offline_worker.js")
    return into


def stamp_build(src: Path, dst: Path, label: str, log) -> None:
    """Copy a real packaged build, stamp its identity, re-embed its worker."""
    build_web = load_build_web()
    for route in ROUTES:
        site = site_dir(dst, route)
        site.parent.mkdir(parents=True, exist_ok=True)
        shutil.copytree(site_dir(src, route), site)
        index = site / "index.html"
        html = index.read_text()
        if "<head>" not in html:
            raise Failure(f"no <head> to stamp in {index}")
        stamp = f'<head><meta name="boardstudio-build" content="{label}">'
        index.write_text(html.replace("<head>", stamp, 1))
        stamp_wasm(site / "wasm" / "boardstudio-web_bg.wasm", label)
        worker = extract_worker(site_dir(src, route), dst.parent / f"{dst.name}-worker-{route}")
        build_web.embed_offline_worker(site, worker)
    print(f"stamped {label}: {dst}", file=log, flush=True)


def packaged_diff(a: Path, b: Path, log, driver: Driver) -> None:
    for route in ROUTES:
        sa, sb = site_dir(a, route), site_dir(b, route)
        files_a = {p.relative_to(sa).as_posix() for p in sa.rglob("*") if p.is_file()}
        files_b = {p.relative_to(sb).as_posix() for p in sb.rglob("*") if p.is_file()}
        differing = sorted(f for f in files_a & files_b if sha256_file(sa / f) != sha256_file(sb / f))
        for name in differing:
            print(f"packaged-diff {route} {name}: A={sha256_file(sa / name)} B={sha256_file(sb / name)}",
                  file=log, flush=True)
        va, vb = worker_version(a, route), worker_version(b, route)
        print(f"packaged-version {route}: A={va} B={vb}", file=log, flush=True)
        driver.check(f"{route}: A and B file sets are identical", files_a == files_b)
        driver.check(f"{route}: packaged diff is exactly index.html + service-worker.js + page wasm",
                     differing == ["index.html", "service-worker.js", "wasm/boardstudio-web_bg.wasm"],
                     str(differing))
        driver.check(f"{route}: A and B versions differ", va != vb, f"A={va} B={vb}")


def derive_broken(b: Path, dst: Path, kind: str, log) -> Path:
    """A copy of B whose install must fail: a script error or a missing asset."""
    if dst.exists():
        shutil.rmtree(dst)
    for route in ROUTES:
        site = site_dir(dst, route)
        site.parent.mkdir(parents=True, exist_ok=True)
        shutil.copytree(site_dir(b, route), site)
    for route in ROUTES:
        site = site_dir(dst, route)
        if kind == "script":
            sw = site / "service-worker.js"
            sw.write_text(sw.read_text() + '\nthrow new Error("019 injected install failure");\n')
        else:
            # A manifest entry whose file does not exist: its URL was never served,
            # so no HTTP-cache entry can satisfy it and cache.addAll must reject.
            sw = site / "service-worker.js"
            text = sw.read_text()
            manifest = re.search(r'(start_offline_worker\("[^"]+", )(\[.*\])(\);)', text, re.S)
            assets = json.loads(manifest.group(2))
            missing = "assets/019-missing-release-asset.bin"
            assets.append(missing)
            sw.write_text(text[:manifest.start(2)] + json.dumps(assets) + text[manifest.end(2):])
            print(f"broken({kind}) {route}: manifest lists missing asset {missing}", file=log,
                  flush=True)
    return dst


# ---------------------------------------------------------------------------
# Shared steps.
# ---------------------------------------------------------------------------

# Settings that attach the CLI to an existing browser or reuse persisted user state.
# agent-browser 0.39.1 precedence: ~/.agent-browser/config.json < ./agent-browser.json
# < AGENT_BROWSER_* environment < flags, except that --config / AGENT_BROWSER_CONFIG
# replaces both default config files. AGENT_BROWSER_PROFILE is not listed:
# start_session always overrides it with the session's private profile.
UNSAFE_ENV = ("AGENT_BROWSER_CDP", "AGENT_BROWSER_AUTO_CONNECT", "AGENT_BROWSER_PROVIDER",
              "AGENT_BROWSER_RESTORE", "AGENT_BROWSER_SESSION_NAME", "AGENT_BROWSER_STATE")
UNSAFE_CONFIG_KEYS = ("cdp", "autoConnect", "provider", "restore", "sessionName", "state")
PROFILE_ARGS = ("--user-data-dir", "--remote-debugging", "--profile-directory")


def inherited_settings_refusal() -> str:
    """Why this process must not start a browser session, naming settings, never values."""
    found = [f"environment {name}" for name in UNSAFE_ENV if os.environ.get(name)]
    args = [("environment AGENT_BROWSER_ARGS", os.environ.get("AGENT_BROWSER_ARGS", ""))]
    explicit = os.environ.get("AGENT_BROWSER_CONFIG")
    configs = [ROOT / explicit] if explicit else [
        Path.home() / ".agent-browser" / "config.json", ROOT / "agent-browser.json"]
    for path in configs:
        if not path.is_file():
            continue
        try:
            config = json.loads(path.read_text())
        except (OSError, ValueError):
            config = None
        if not isinstance(config, dict):
            found.append(f"config {path} unreadable")
            continue
        found += [f"config {path} key {key}" for key in UNSAFE_CONFIG_KEYS if config.get(key)]
        args.append((f"config {path} key args", str(config.get("args") or "")))
    found += [f"{where} (profile or debugging-port argument)" for where, text in args
              if any(marker in text for marker in PROFILE_ARGS)]
    if not found:
        return ""
    return ("refusing inherited browser settings that could attach to or reuse user browser "
            "state: " + "; ".join(found))


def new_session(prefix: str) -> str:
    done = subprocess.run(["agent-browser", "session", "id", "--scope", "worktree",
                           "--prefix", prefix], cwd=ROOT, capture_output=True, text=True,
                          timeout=60)
    session = done.stdout.strip().splitlines()[-1] if done.stdout.strip() else ""
    if not session:
        raise Failure("could not create agent-browser session")
    return session


UNRESOLVED = "previous browser/profile cleanup unresolved; retain it before starting another session"


def cleanup_unresolved(driver: Driver) -> bool:
    return not driver.session_closed or driver.owned_profile is not None


def make_session_dir(driver: Driver, prefix: str) -> Path:
    """Create the directory one session (and its caller's fixtures) lives in.

    The driver records it at once, so the cleanup that follows the session, including a
    session that never started, may delete it; nothing else is ever recorded or deleted.
    """
    if cleanup_unresolved(driver) or driver.session_dir is not None:
        raise Failure(UNRESOLVED)
    driver.session_dir = Path(tempfile.mkdtemp(prefix=prefix))
    return driver.session_dir


def start_session(driver: Driver, prefix: str, url: str, profile: Path | None = None) -> None:
    refusal = inherited_settings_refusal()
    if refusal:
        raise Failure(refusal)
    if cleanup_unresolved(driver):
        raise Failure(UNRESOLVED)
    session = new_session(prefix)
    if profile is None:
        profile = make_session_dir(driver, "boardstudio-verify-session-") / "profile"
    driver.env = dict(os.environ, AGENT_BROWSER_SESSION=session,
                      AGENT_BROWSER_PROFILE=str(profile))
    driver.owned_profile = profile
    driver.note(f"owned browser profile: {profile}")
    driver.session_closed = False
    driver.close_attempted = False
    driver.cmd(f"session: {session} (fresh isolated browser profile)")
    driver.run(["agent-browser", "open", url])
    driver.run(["agent-browser", "wait", "--text", "Your keyboards"], timeout=90)


def end_session(driver: Driver) -> None:
    # A profile-less session's private directory goes through the same confirmed
    # close, exact-profile process check and removal as an explicit owned profile.
    if driver.session_dir is None:
        close_browser(driver)
    else:
        cleanup_owned_profile(driver, driver.session_dir, "session")


def close_browser(driver: Driver) -> None:
    # A second close can start a new daemon/browser and recreate a just-deleted
    # explicit disk profile. Leg cleanup and main's finally share this guard.
    if driver.session_closed or driver.close_attempted:
        return
    driver.close_attempted = True
    failures_before_close = len(driver.failures)
    try:
        closed = driver.run(["agent-browser", "close"])
        if "Browser closed" in closed and len(driver.failures) == failures_before_close:
            driver.session_closed = True
        else:
            driver.fail("session cleanup: browser close unconfirmed")
    except (OSError, subprocess.SubprocessError) as error:
        driver.fail("session cleanup: close failed", str(error))


def cleanup_owned_profile(driver: Driver, directory: Path, label: str) -> None:
    """Remove only this fixture's directory after its own Chrome has exited."""
    if driver.owned_profile is None and driver.session_dir == directory:
        # Made for a session that never started: no browser can be using it.
        if remove_owned_tree(driver, directory, label):
            driver.session_dir = None
        return
    if driver.owned_profile != directory / "profile":
        driver.fail(f"{label} cleanup: directory not owned by this session; retain it",
                    str(directory))
        return
    close_browser(driver)
    if not driver.session_closed:
        driver.fail(f"{label} cleanup: close unconfirmed; retain owned directory", str(directory))
        return
    deadline = time.monotonic() + 15
    profile_arg = f"--user-data-dir={directory / 'profile'}"
    while True:
        try:
            processes = subprocess.check_output(
                ["ps", "-eo", "pid,args"], text=True, timeout=15)
        except (OSError, subprocess.SubprocessError) as error:
            driver.fail(f"{label} cleanup: process state unknown; retain owned directory",
                        f"{directory}: {error}")
            return
        # ps prints argument text without shell quoting, including spaces in paths.
        live = [line for line in processes.splitlines()
                if re.search(r"(?<!\S)" + re.escape(profile_arg) + r"(?=\s|$)", line)]
        if not live or time.monotonic() >= deadline:
            break
        time.sleep(0.5)
    driver.note(f"{label} profile Chrome processes after close: {live}")
    if live:
        driver.fail(f"{label} cleanup: profile still in use; retain owned directory", str(directory))
        return
    driver.note(f"{label} profile IndexedDB before deletion: {list(directory.rglob('IndexedDB'))}")
    if remove_owned_tree(driver, directory, label):
        driver.owned_profile = None
        if driver.session_dir == directory:
            driver.session_dir = None


def remove_owned_tree(driver: Driver, directory: Path, label: str) -> bool:
    """The one place an invocation-owned directory is deleted; True once it is released."""
    if driver.keep_artifacts:
        driver.kept.append(directory)
        driver.note(f"{label} kept for diagnosis (--keep-artifacts): {directory}")
        return True
    try:
        shutil.rmtree(directory)
    except OSError as error:
        driver.fail(f"{label} cleanup: removal failed; retain owned directory",
                    f"{directory}: {error}")
        return False
    driver.check(f"{label} owned fixture/profile/database removed", not directory.exists())
    return not directory.exists()


def find_ref(snapshot: str, pattern: str) -> str | None:
    for line in snapshot.splitlines():
        match = re.search(pattern + r'[^\n]*?ref=(e\d+)', line)
        if match:
            return "@" + match.group(match.lastindex)
    return None


def open_demo(driver: Driver) -> None:
    """Open the demo copy. The library list re-renders when another editor of the
    same origin saves, collapsing the demo chooser, so re-resolve from a fresh
    snapshot every attempt instead of trusting one ref."""
    driver.cmd("open demo copy (fresh snapshot each attempt)")
    for attempt in range(1, 6):
        snap = driver.snapshot()
        copy = find_ref(snap, r'button "Open demo copy"')
        if copy is not None:
            out = driver.run(["agent-browser", "click", copy], ok=True)
            if "\u2717" not in out:
                break
        else:
            chooser = find_ref(snap, r'button "Open demo"')
            driver.cmd(f"attempt {attempt}: chooser ref {chooser}")
            if chooser is not None:
                driver.run(["agent-browser", "click", chooser], ok=True)
        time.sleep(1.5)
    driver.wait_main_board()


def _wait_main_board(self: Driver) -> None:
    self.run(["agent-browser", "wait", "--text", "Main board"], timeout=90)


Driver.wait_main_board = _wait_main_board  # type: ignore[attr-defined]


def set_marker(driver: Driver) -> str:
    mark = (driver.eval_file("marker.js") or {}).get("loadMark")
    driver.check("load marker set", bool(mark), str(mark))
    if not mark:
        raise Failure("no load marker")
    return mark


def check_caches(driver: Driver, label: str, expected: dict[str, str],
                 shells: dict[str, str] | None = None) -> None:
    """Expect exactly these versioned caches, each storing that release's worker."""
    content = driver.eval_file("cachecontent.js") or {}
    cached = content.get("cachedWorkerVersions", {})
    driver.note(f"{label}: cached shell builds {content.get('cachedShellBuilds')}")
    for name, build in (shells or {}).items():
        actual = (content.get("cachedShellBuilds") or {}).get(name)
        driver.check(f"{label}: cache {name[-36:]} holds build {build}'s index.html", actual == build,
                     f"actual={actual}")
    for name, version in expected.items():
        driver.check(f"{label}: cache {name[-36:]} holds {version}", cached.get(name) == version,
                     f"actual={cached.get(name)}")
    driver.check(f"{label}: no unexpected caches", set(cached) == set(expected),
                 f"caches={sorted(cached)}")


def close_toast_with_keyboard(driver: Driver) -> None:
    snap = driver.snapshot()
    ref = None
    for line in snap.splitlines():
        match = re.search(r'button "close"[^\n]*?ref=(e\d+)', line)
        if match:
            ref = "@" + match.group(1)
            break
    driver.cmd(f"close ref: {ref}")
    if ref is None:
        driver.fail("toast close ref not found")
        raise Failure("no close ref")
    driver.run(["agent-browser", "focus", ref])
    driver.run(["agent-browser", "press", "Enter"])
    time.sleep(2)


def publish_and_wait_waiting(driver: Driver, label: str) -> dict:
    result = driver.eval_file("update.js")
    driver.check(f"{label}: update did not reject", result.get("updateThrew") is None,
                 str(result.get("updateThrew")))
    driver.check(f"{label}: waiting worker present", result.get("waiting") == "PRESENT", str(result))
    driver.check(f"{label}: notice shown", result.get("notice") is True)
    return result


def expect_waiting_state(driver: Driver, label: str, mark: str, build: str | None) -> dict:
    s = driver.state()
    driver.check(f"{label}: controlled", bool(s.get("controller")))
    driver.check(f"{label}: waiting present", s.get("waiting") == "installed", str(s.get("waiting")))
    driver.check(f"{label}: loaded document is {build}", s.get("build") == build, str(s.get("build")))
    driver.check(f"{label}: load marker unchanged (no reload)", s.get("loadMark") == mark, mark)
    driver.check(f"{label}: editor alive", s.get("editorAlive") is True)
    return s


def wait_for_adoption_reload(driver: Driver, label: str, timeout_s: int = 60) -> None:
    """After Update Now: the old document must go away (marker gone), app must render."""
    gone = driver.poll(f"{label}: old load marker gone (page reloaded)",
                       "window.__loadMark === undefined", timeout_s)
    driver.check(f"{label}: page reloaded onto the adopted build", gone)
    driver.poll(f"{label}: app rendered",
                "document.body && /Your keyboards|Main board/.test(document.body.textContent)",
                timeout_s)


def check_loaded_binary(driver: Driver, label: str, build: str) -> None:
    binary = driver.eval_file("loaded-binary.js") or {}
    driver.check(f"{label}: application binary served to this page is build {build}",
                 binary.get("wasmBuild") == build, str(binary))
    driver.check(f"{label}: this document requested that application binary",
                 binary.get("requestedByThisDocument") is True, str(binary.get("url")))


def expect_adopted(driver: Driver, label: str, build: str, expected_caches: dict[str, str]) -> dict:
    s = driver.state()
    check_loaded_binary(driver, label, build)
    driver.check(f"{label}: loaded document is {build}", s.get("build") == build, str(s.get("build")))
    driver.check(f"{label}: navigation was a reload", s.get("navType") == "reload", str(s.get("navType")))
    driver.check(f"{label}: controlled by the new worker", bool(s.get("controller")))
    driver.check(f"{label}: worker active", s.get("active") == "activated", str(s.get("active")))
    driver.check(f"{label}: nothing waiting", s.get("waiting") is None, str(s.get("waiting")))
    driver.check(f"{label}: no update notice left", s.get("updateNotice") is False,
                 str(s.get("allStatuses")))
    driver.check(f"{label}: no Update Now controls left",
                 not s.get("toastUpdateNow") and not s.get("headerUpdateNow"))
    check_caches(driver, label, expected_caches,
                 {name: build for name in expected_caches})
    return s


def reopen_saved_project(driver: Driver, label: str) -> None:
    """Prove the saved project survived adoption by reopening it from the library."""
    s = driver.state()
    if s.get("mainBoard"):
        driver.check(f"{label}: saved project open after reload", True)
        return
    ref = find_ref(driver.snapshot(), r'button "Preview [^"]+"')
    driver.cmd(f"saved project preview ref: {ref}")
    driver.check(f"{label}: saved project is listed in the library", ref is not None)
    if ref is None:
        return
    driver.run(["agent-browser", "click", ref])
    time.sleep(1)
    snap = driver.snapshot()
    ref = find_ref(snap, r'button "Open(?! demo)[^"]*"')
    driver.cmd(f"saved project open ref: {ref}")
    driver.check(f"{label}: saved project can be opened from its preview", ref is not None)
    if ref is None:
        return
    driver.run(["agent-browser", "click", ref])
    driver.poll(f"{label}: saved project open", "document.body.textContent.includes('Main board')", 60)
    driver.check(f"{label}: saved project reopened with its content",
                 driver.state().get("mainBoard") is True)


# ---------------------------------------------------------------------------
# Legs.
# ---------------------------------------------------------------------------

class Ctx:
    def __init__(self, driver, server, a, b, work, legacy_b, log):
        self.d = driver
        self.server = server
        self.a = a
        self.b = b
        self.work = work
        self.legacy_b = legacy_b
        self.log = log
        self.va = {r: worker_version(a, r) for r in ROUTES}
        self.vb = {r: worker_version(b, r) for r in ROUTES}

    def names(self, route: str, which: str) -> str:
        version = (self.va if which == "A" else self.vb)[route]
        return cache_name(route, version)


def leg_adopt(ctx: Ctx) -> None:
    d = ctx.d
    d.cmd("LEG adopt: sole editor, toast dismissal, header Update Now")
    ctx.server.swap(d, ctx.a, "serve A")
    start_session(d, "019-adopt", route_url(d.port, "root"))
    open_demo(d)
    mark = set_marker(d)
    s = d.state()
    d.check("A: controlled, no waiting, loaded document is A",
            bool(s.get("controller")) and s.get("waiting") is None and s.get("build") == "A",
            f"build={s.get('build')} waiting={s.get('waiting')}")
    d.check("A: single A cache", s.get("caches") == [ctx.names("root", "A")], str(s.get("caches")))
    d.check("A: served version is A's", serve_version(d.port) == ctx.va["root"])
    check_loaded_binary(d, "A", "A")
    d.note(f"save label before update: {s.get('saveLabel')}")
    check_caches(d, "A installed", {ctx.names("root", "A"): ctx.va["root"]})

    ctx.server.swap(d, ctx.b, "publish B at the same origin")
    d.check("B: served version is B's", serve_version(d.port) == ctx.vb["root"])
    publish_and_wait_waiting(d, "B published")
    s = expect_waiting_state(d, "A with B waiting", mark, "A")
    d.check("toast offers a keyboard-reachable Update Now", s.get("toastUpdateNow") is True)
    d.check("header notice absent while the toast is up", s.get("headerNotice") is False)
    check_caches(d, "B waiting", {ctx.names("root", "A"): ctx.va["root"],
                                  ctx.names("root", "B"): ctx.vb["root"]},
                 {ctx.names("root", "A"): "A", ctx.names("root", "B"): "B"})

    close_toast_with_keyboard(d)
    s = expect_waiting_state(d, "toast dismissed", mark, "A")
    d.check("dismissal: toast gone", s.get("toast") is False)
    d.check("dismissal: header notice with Update Now persists", s.get("headerUpdateNow") is True,
            str(s.get("headerNotice")))

    d.cmd("click header Update Now (sole editor, work settled)")
    d.click_selector(UPDATE_NOW_HEADER)
    wait_for_adoption_reload(d, "adopt")
    expect_adopted(d, "adopted B", "B", {ctx.names("root", "B"): ctx.vb["root"]})
    d.check("adopted: served version is still B's", serve_version(d.port) == ctx.vb["root"])
    reopen_saved_project(d, "adopted B")
    end_session(d)


def leg_second_editor(ctx: Ctx) -> None:
    d = ctx.d
    d.cmd("LEG second-editor: deferral, offline continuity, adoption once sole")
    ctx.server.swap(d, ctx.a, "serve A")
    start_session(d, "019-second", route_url(d.port, "root"))
    open_demo(d)
    mark1 = set_marker(d)
    d.tab_new("second", route_url(d.port, "root"))
    d.run(["agent-browser", "wait", "--text", "Your keyboards"], timeout=90)
    open_demo(d)
    mark2 = set_marker(d)
    s2 = d.state()
    d.check("second editor: loaded document is A and controlled",
            s2.get("build") == "A" and bool(s2.get("controller")), str(s2.get("build")))

    ctx.server.swap(d, ctx.b, "publish B at the same origin")
    d.tab_switch("t1")
    publish_and_wait_waiting(d, "B published (first editor)")
    s1 = expect_waiting_state(d, "first editor", mark1, "A")
    d.check("first editor: toast offers Update Now", s1.get("toastUpdateNow") is True)

    d.cmd("Update Now with a second A editor open must defer, never force")
    d.click_selector(UPDATE_NOW_TOAST)
    d.poll("deferral dialog shown", "!!document.querySelector('[role=alertdialog]')", 30)
    s1 = d.state()
    d.check("second editor open: warning names closing other tabs",
            "Please close any other open tabs" in (s1.get("dialogText") or ""),
            str(s1.get("dialogText")))
    d.check("second editor open: load marker unchanged (no reload)", s1.get("loadMark") == mark1)
    d.check("second editor open: B still waiting", s1.get("waiting") == "installed")
    d.check("second editor open: loaded document still A", s1.get("build") == "A")
    d.check("second editor open: editor alive", s1.get("editorAlive") is True)
    d.click_button("Cancel")
    time.sleep(1)
    s1 = d.state()
    d.check("cancel: warning closed", s1.get("dialogText") is None)
    d.check("cancel: still waiting, still A, editor intact",
            s1.get("waiting") == "installed" and s1.get("build") == "A" and s1.get("editorAlive") is True)
    d.check("cancel: load marker unchanged", s1.get("loadMark") == mark1)
    d.check("cancel: Update Now still offered", s1.get("toastUpdateNow") is True)

    d.cmd("second editor continues offline: uncached-asset request served by A's cache")
    d.tab_switch("second")
    pick = d.eval_file("asset-pick.js")
    d.check("second editor: found a manifest asset it has not used", bool(pick.get("asset")),
            str(pick))
    d.check("second editor: asset was not already requested by this page",
            pick.get("alreadyUsed") is False)
    asset_rel = pick.get("asset")
    ctx.server.stop()
    d.cmd("origin stopped: the request below can only be served from A's cache")
    fetched = d.eval_file("asset-fetch.js")
    expected_sha = sha256_file(site_dir(ctx.a, "root") / asset_rel) if asset_rel else None
    d.check("offline: request answered", fetched.get("fetched") is True, str(fetched))
    d.check("offline: status 200", fetched.get("status") == 200, str(fetched.get("status")))
    d.check("offline: bytes equal A's packaged asset", fetched.get("sha256") == expected_sha,
            f"got={fetched.get('sha256')} want={expected_sha}")
    ctx.server.start()
    s2 = d.state()
    d.check("second editor after deferral: marker unchanged", s2.get("loadMark") == mark2)
    d.check("second editor after deferral: editor alive, document A",
            s2.get("editorAlive") is True and s2.get("build") == "A")
    check_caches(d, "A caches intact", {ctx.names("root", "A"): ctx.va["root"],
                                         ctx.names("root", "B"): ctx.vb["root"]})

    d.cmd("close the second editor, then adopt from the first")
    d.tab_close("second")
    d.tab_switch("t1")
    time.sleep(3)
    d.click_selector(UPDATE_NOW_TOAST)
    wait_for_adoption_reload(d, "second-editor adopt")
    expect_adopted(d, "adopted B", "B", {ctx.names("root", "B"): ctx.vb["root"]})
    end_session(d)


def leg_subpath(ctx: Ctx) -> None:
    d = ctx.d
    d.cmd("LEG subpath: root and /boardstudio/ registrations stay isolated")
    ctx.server.swap(d, ctx.a, "serve A (both routes)")
    start_session(d, "019-subpath", route_url(d.port, "root"))
    open_demo(d)
    mark_root = set_marker(d)
    d.tab_new("sub", route_url(d.port, "subpath"))
    d.run(["agent-browser", "wait", "--text", "Your keyboards"], timeout=90)
    open_demo(d)
    mark_sub = set_marker(d)
    s = d.state()
    d.check("subpath editor: scope is /boardstudio/ and document is A",
            s.get("href", "").startswith(route_url(d.port, "subpath")) and s.get("build") == "A",
            str(s.get("href")))
    all_a = {ctx.names("root", "A"): ctx.va["root"], ctx.names("subpath", "A"): ctx.va["subpath"]}
    check_caches(d, "both routes installed", all_a)

    ctx.server.swap(d, ctx.b, "publish B for both routes")
    publish_and_wait_waiting(d, "subpath B published")
    d.tab_switch("t1")
    publish_and_wait_waiting(d, "root B published")
    all_ab = {**all_a, ctx.names("root", "B"): ctx.vb["root"],
              ctx.names("subpath", "B"): ctx.vb["subpath"]}
    check_caches(d, "both routes waiting", all_ab)

    sub_hex = "2f626f61726473747564696f2f"

    def sub_caches() -> list[str]:
        return [n for n in d.state().get("caches", []) if sub_hex in n]

    d.cmd("root Update Now with the subpath editor open: the origin-wide probe defers")
    d.click_selector(UPDATE_NOW_TOAST)
    d.poll("root deferral dialog", "!!document.querySelector('[role=alertdialog]')", 30)
    s = d.state()
    d.note("the sole-client probe counts every same-origin window (includeUncontrolled), "
           "so a subpath editor defers root adoption; recorded as conservative behavior")
    d.check("root deferred while subpath editor is open",
            "Please close any other open tabs" in (s.get("dialogText") or ""), str(s.get("dialogText")))
    d.check("root deferred: marker unchanged", s.get("loadMark") == mark_root)
    d.click_button("Cancel")
    check_caches(d, "root deferral touched no cache", all_ab)

    d.cmd("subpath Update Now with the root editor open also defers")
    d.tab_switch("sub")
    d.click_selector(UPDATE_NOW_TOAST)
    d.poll("subpath deferral dialog", "!!document.querySelector('[role=alertdialog]')", 30)
    s = d.state()
    d.check("subpath deferred while root editor is open",
            "Please close any other open tabs" in (s.get("dialogText") or ""), str(s.get("dialogText")))
    d.check("subpath deferred: marker unchanged", s.get("loadMark") == mark_sub)
    d.click_button("Cancel")
    check_caches(d, "subpath deferral touched no cache", all_ab)

    d.cmd("close the subpath editor; its waiting B activates by ordinary delayed activation")
    d.tab_close("sub")
    d.tab_switch("t1")
    time.sleep(5)
    pre_sub = sub_caches()
    d.note(f"subpath caches before root adoption: {pre_sub}")
    d.check("subpath scope kept a B cache", ctx.names("subpath", "B") in pre_sub, str(pre_sub))
    sub_a_gone = ctx.names("subpath", "A") not in pre_sub
    d.note("subpath A cache " + ("removed by the subpath scope's own ordinary activation"
                                 if sub_a_gone else "still present (subpath B not yet active)"))

    d.cmd("root adopts alone; the subpath caches must not change")
    d.click_selector(UPDATE_NOW_TOAST)
    wait_for_adoption_reload(d, "root adopt")
    s = d.state()
    d.check("root adopted: document is B", s.get("build") == "B", str(s.get("build")))
    d.check("root adopted: nothing waiting in the root scope", s.get("waiting") is None)
    d.check("root adoption left the subpath caches alone", sub_caches() == pre_sub,
            f"before={pre_sub} after={sub_caches()}")
    root_caches = [n for n in s.get("caches", []) if sub_hex not in n]
    d.check("root scope keeps only B's cache", root_caches == [ctx.names("root", "B")], str(root_caches))

    d.cmd("reopen the subpath editor: independent scope, its own release state")
    d.tab_new("sub2", route_url(d.port, "subpath"))
    d.run(["agent-browser", "wait", "--text", "Your keyboards"], timeout=90)
    set_marker(d)
    s = d.state()
    d.check("subpath scope is its own", (s.get("href") or "").startswith(route_url(d.port, "subpath")))
    d.check("subpath editor loads the release its scope activated",
            s.get("build") == ("B" if sub_a_gone else "A"), str(s.get("build")))
    d.check("subpath scope has nothing waiting after its activation",
            (s.get("waiting") is None) if sub_a_gone else (s.get("waiting") == "installed"),
            str(s.get("waiting")))
    check_caches(d, "both routes on B", {ctx.names("root", "B"): ctx.vb["root"],
                                         ctx.names("subpath", "B"): ctx.vb["subpath"]})
    end_session(d)


def run_install_failure(ctx: Ctx, kind: str, mark: str) -> None:
    d = ctx.d
    broken = derive_broken(ctx.b, ctx.work / f"broken-{kind}", kind, ctx.log)
    ctx.server.swap(d, broken, f"broken B ({kind})")
    failed = d.eval_file("failed.js")
    d.check(f"failed install ({kind}): update rejected or never waited",
            failed.get("updateThrew") is True or failed.get("waiting") is None, str(failed))
    d.check(f"failed install ({kind}): never advanced to waiting", failed.get("waiting") is None)
    d.check(f"failed install ({kind}): no notice or Update Now at any poll during the attempt",
            failed.get("controlsEverShown") is False, str(failed))
    s = d.state()
    d.check(f"failed install ({kind}): still A, controlled, no waiting",
            s.get("build") == "A" and bool(s.get("controller")) and s.get("waiting") is None)
    d.check(f"failed install ({kind}): no notice, no toast, no Update Now, no header notice",
            not s.get("updateNotice") and not s.get("toastUpdateNow") and not s.get("headerUpdateNow"))
    d.check(f"failed install ({kind}): no reload", s.get("loadMark") == mark)
    d.check(f"failed install ({kind}): editor alive", s.get("editorAlive") is True)
    extras = sorted(set(s.get("caches") or []) - {ctx.names("root", "A")})
    d.check(f"failed install ({kind}): A's cache intact", ctx.names("root", "A") in (s.get("caches") or []))
    if kind == "script":
        d.check(f"failed install ({kind}): no other cache exists", extras == [], str(extras))
    else:
        # cache.open(name) runs before addAll rejects, so the failed release leaves
        # an incomplete cache under its own name; it must never be the active one.
        d.check(f"failed install ({kind}): only the failed release's own incomplete cache is extra",
                extras in ([], [ctx.names("root", "B")]), str(extras))
        d.note(f"failed asset install leftover caches: {extras}")


def leg_failed_install(ctx: Ctx) -> None:
    d = ctx.d
    d.cmd("LEG failed-install: control that must not advance, then recover")
    ctx.server.swap(d, ctx.a, "serve A")
    start_session(d, "019-failed", route_url(d.port, "root"))
    open_demo(d)
    mark = set_marker(d)
    run_install_failure(ctx, "script", mark)
    run_install_failure(ctx, "asset", mark)
    ctx.server.swap(d, ctx.b, "repaired B")
    publish_and_wait_waiting(d, "repaired B")
    s = expect_waiting_state(d, "after repair", mark, "A")
    d.check("after repair: Update Now offered again", s.get("toastUpdateNow") is True)
    end_session(d)


def leg_unavailable_target(ctx: Ctx) -> None:
    d = ctx.d
    d.cmd("LEG unavailable-target: waiting worker cannot take over; report, no reload, recover")
    ctx.server.swap(d, ctx.a, "serve A")
    start_session(d, "019-unavail", route_url(d.port, "root"))
    open_demo(d)
    mark = set_marker(d)
    legacy = ctx.legacy_b
    d.check("target lacks the adoption protocol (control build)", legacy is not None
            and not worker_has_adopt_protocol(legacy), str(legacy))
    ctx.server.swap(d, legacy, "waiting target that cannot take over")
    publish_and_wait_waiting(d, "unavailable target waiting")
    d.click_selector(UPDATE_NOW_TOAST)
    d.poll("failure reported", "document.body.textContent.includes('could not be adopted')", 40)
    s = d.state()
    joined = " | ".join(s.get("report") or [])
    d.check("failure is reported, not silent", "could not be adopted" in joined, joined)
    d.check("failure offers a retry entry point in the report", "Try Update Now again" in joined, joined)
    d.check("failure: no reload, load marker unchanged", s.get("loadMark") == mark)
    d.check("failure: still A and controlled", s.get("build") == "A" and bool(s.get("controller")))
    d.check("failure: target still waiting (nothing advanced)", s.get("waiting") == "installed")
    d.check("failure: Update Now control is still offered",
            s.get("toastUpdateNow") is True or s.get("headerUpdateNow") is True)
    d.check("failure: editor alive", s.get("editorAlive") is True)

    d.cmd("recover: publish a repaired B and use the same entry point")
    ctx.server.swap(d, ctx.b, "repaired B")
    publish_and_wait_waiting(d, "repaired B")
    s = expect_waiting_state(d, "repaired B waiting", mark, "A")
    d.click_selector(UPDATE_NOW_TOAST if s.get("toastUpdateNow") else UPDATE_NOW_HEADER)
    wait_for_adoption_reload(d, "recovery adopt")
    expect_adopted(d, "recovered to B", "B", {ctx.names("root", "B"): ctx.vb["root"]})
    end_session(d)


def leg_late_adopt(ctx: Ctx) -> None:
    d = ctx.d
    d.cmd("LEG late-adopt: the waiting worker, not the page, decides; a second editor is open")
    ctx.server.swap(d, ctx.a, "serve A")
    start_session(d, "019-late", route_url(d.port, "root"))
    open_demo(d)
    mark1 = set_marker(d)
    d.tab_new("second", route_url(d.port, "root"))
    d.run(["agent-browser", "wait", "--text", "Your keyboards"], timeout=90)
    open_demo(d)
    mark2 = set_marker(d)
    ctx.server.swap(d, ctx.b, "publish B at the same origin")
    d.tab_switch("t1")
    publish_and_wait_waiting(d, "B published (first editor)")
    d.tab_switch("second")
    pick = d.eval_file("asset-pick.js")
    d.check("second editor: found a manifest asset it has not used", bool(pick.get("asset")), str(pick))
    asset_rel = pick.get("asset")
    d.tab_switch("t1")

    d.cmd("post the adoption request straight to the waiting worker (no page-side probe)")
    direct = d.eval_file("adopt-direct.js")
    d.check("adoption request reached the waiting worker", direct.get("posted") is True, str(direct))
    d.check("worker declined: B still waiting", direct.get("waiting") == "installed", str(direct))
    d.check("worker declined: nothing activated B", direct.get("active") == "activated")
    both = sorted([ctx.names("root", "A"), ctx.names("root", "B")])
    d.check("worker declined: A's cache survives next to B's", direct.get("caches") == both,
            str(direct.get("caches")))
    s1 = d.state()
    d.check("first editor untouched: marker, build A, editor alive",
            s1.get("loadMark") == mark1 and s1.get("build") == "A" and s1.get("editorAlive") is True)

    d.tab_switch("second")
    ctx.server.stop()
    d.cmd("origin stopped: the second editor's uncached-asset request can only come from A's cache")
    fetched = d.eval_file("asset-fetch.js")
    expected_sha = sha256_file(site_dir(ctx.a, "root") / asset_rel) if asset_rel else None
    d.check("offline: second editor's request answered from A's cache",
            fetched.get("status") == 200 and fetched.get("sha256") == expected_sha, str(fetched))
    ctx.server.start()
    s2 = d.state()
    d.check("second editor untouched: marker, build A, editor alive",
            s2.get("loadMark") == mark2 and s2.get("build") == "A" and s2.get("editorAlive") is True)

    d.cmd("close the second editor, then adopt through the page as usual")
    d.tab_close("second")
    d.tab_switch("t1")
    time.sleep(3)
    d.click_selector(UPDATE_NOW_TOAST)
    wait_for_adoption_reload(d, "late-adopt adopt")
    expect_adopted(d, "adopted B", "B", {ctx.names("root", "B"): ctx.vb["root"]})
    end_session(d)


BOARD_DRAFT = "Draft rename 019"


def board_name_ref(d: Driver) -> str:
    ref = find_ref(d.snapshot(), r'textbox "Board name"')
    if ref is None:
        d.fail("Board name field not found")
        raise Failure("no Board name field")
    return ref


def leg_draft_safety(ctx: Ctx) -> None:
    d = ctx.d
    d.cmd("LEG draft-safety: a real draft plus an invalid field defer, cancel is safe, then adopt")
    ctx.server.swap(d, ctx.a, "serve A")
    start_session(d, "019-draft", route_url(d.port, "root"))
    open_demo(d)
    mark = set_marker(d)
    ctx.server.swap(d, ctx.b, "publish B at the same origin")
    publish_and_wait_waiting(d, "B published")
    expect_waiting_state(d, "A with B waiting", mark, "A")

    d.cmd("type a real Board name draft (focus stays in the field), then add an invalid field")
    d.run(["agent-browser", "fill", board_name_ref(d), BOARD_DRAFT])
    d.eval_file("inject-invalid.js")
    d.click_selector(UPDATE_NOW_TOAST)
    d.poll("deferral dialog shown", "!!document.querySelector('[role=alertdialog]')", 30)
    s = d.state()
    d.check("invalid draft: warning names the invalid value",
            "invalid value" in (s.get("dialogText") or ""), str(s.get("dialogText")))
    d.check("invalid draft: no reload, still A, still waiting",
            s.get("loadMark") == mark and s.get("build") == "A" and s.get("waiting") == "installed")
    d.press("Enter")
    time.sleep(1)
    s = d.state()
    d.check("keyboard Enter on Cancel closed the warning", s.get("dialogText") is None)
    d.check("cancel returned keyboard focus to Update Now", s.get("focusOnUpdateNow") is True)
    d.check("cancel kept the editor", s.get("editorAlive") is True and s.get("loadMark") == mark)
    name = d.eval_file("board-name.js")
    d.check("cancel preserved the Board name draft", name.get("boardName") == BOARD_DRAFT, str(name))

    d.cmd("fix the invalid field and let the work settle and save")
    d.eval_file("remove-invalid.js")
    d.poll("project saved", "Array.from(document.querySelectorAll('button')).some(b => "
           "/^Saved locally/.test((b.getAttribute('aria-label') || b.textContent || '').trim()))", 40)
    time.sleep(1)
    d.click_selector(UPDATE_NOW_TOAST)
    wait_for_adoption_reload(d, "draft-safety adopt")
    expect_adopted(d, "adopted B", "B", {ctx.names("root", "B"): ctx.vb["root"]})
    d.poll("the saved project is restored in the reloaded document",
           "!!document.querySelector('[aria-label=\"Board Inspector\"] input[type=text]')", 60)
    name = d.eval_file("board-name.js")
    d.check("the saved project kept the renamed board after adoption (restored from local storage "
            "by the new document, not carried in memory)",
            name.get("boardName") == BOARD_DRAFT, str(name))
    end_session(d)


# (theme, appearance, viewport, reduced motion): every Theme and Appearance at
# desktop width, then narrow layouts and reduced motion on distinct looks.
LOOKS = [
    ("original", "light", (1280, 800), False),
    ("original", "dark", (1280, 800), False),
    ("shadcn", "light", (1280, 800), False),
    ("shadcn", "dark", (1280, 800), False),
    ("original", "light", (390, 800), False),
    ("shadcn", "dark", (390, 800), False),
    ("original", "dark", (1280, 800), True),
]


def apply_look(d: Driver, theme: str, appearance: str, viewport: tuple[int, int], reduced: bool) -> None:
    d.run(["agent-browser", "set", "viewport", str(viewport[0]), str(viewport[1])])
    media = ["agent-browser", "set", "media", appearance] + (["reduced-motion"] if reduced else [])
    d.run(media)
    # The app's own appearance listener rewrites data-theme/data-appearance when
    # the media preference changes; let it settle before applying the look.
    time.sleep(1.5)
    d.eval_expr(f"document.documentElement.setAttribute('data-theme','{theme}');"
                f"document.documentElement.setAttribute('data-appearance','{appearance}');'ok'")
    time.sleep(0.8)


def leg_keyboard_matrix(ctx: Ctx) -> None:
    d = ctx.d
    d.cmd("LEG keyboard-matrix: trusted Tab/Enter/Space across Theme x Appearance, narrow, motion")
    ctx.server.swap(d, ctx.a, "serve A")
    start_session(d, "019-keys", route_url(d.port, "root"))
    open_demo(d)
    mark = set_marker(d)
    ctx.server.swap(d, ctx.b, "publish B at the same origin")
    publish_and_wait_waiting(d, "B published")
    d.eval_file("inject-invalid.js")  # keeps Update Now deferring so the dialog is reproducible
    try:
        for theme, appearance, viewport, reduced in LOOKS:
            label = f"{theme}/{appearance}/{viewport[0]}x{viewport[1]}{'/reduced-motion' if reduced else ''}"
            d.cmd(f"look: {label}")
            apply_look(d, theme, appearance, viewport, reduced)
            geo = d.eval_file("geometry.js")
            d.check(f"{label}: look applied",
                    geo.get("theme") == theme and geo.get("appearance") == appearance
                    and geo.get("viewport") == list(viewport) and geo.get("reducedMotion") is reduced,
                    str(geo))
            toast = geo.get("toast") or {}
            d.check(f"{label}: toast Update Now is visible, inside the viewport and touch-sized",
                    bool(toast) and toast.get("inViewport") is True
                    and toast.get("w", 0) >= 24 and toast.get("h", 0) >= 24, str(toast))
            d.check(f"{label}: toast Update Now text contrast >= 4.5:1",
                    (toast.get("contrast") or 0) >= 4.5, str(toast))
            if reduced:
                d.check(f"{label}: no running animations under reduced motion",
                        geo.get("runningAnimations") == 0, str(geo.get("runningAnimations")))

            prep = d.eval_file("tab-prep.js")
            d.check(f"{label}: Update Now sits in the tab sequence before the toast close control",
                    prep.get("found") is True and prep.get("closeFollows") is True
                    and prep.get("positive") == 0, str(prep))
            d.press("Tab")
            focus = d.eval_file("focus-info.js")
            d.check(f"{label}: one real Tab from '{prep.get('predecessor')}' lands on Update Now",
                    focus.get("label") == "Update Now" and focus.get("inToast") is True, str(focus))

            d.press("Enter")
            d.poll(f"{label}: Enter opens the warning", "!!document.querySelector('[role=alertdialog]')", 15)
            focus = d.eval_file("focus-info.js")
            d.check(f"{label}: Enter opened the invalid-value warning",
                    "invalid value" in (focus.get("dialogText") or ""), str(focus.get("dialogText")))
            d.check(f"{label}: keyboard focus moved into the warning",
                    focus.get("inDialog") is True, str(focus))
            d.eval_file("key-log.js")
            d.press("Space")
            time.sleep(0.8)
            d.eval_file("key-log-read.js")
            focus = d.eval_file("focus-info.js")
            d.check(f"{label}: Space on Cancel closed the warning", focus.get("dialogOpen") is False,
                    str(focus))
            d.check(f"{label}: focus returned to Update Now after Space-cancel",
                    focus.get("label") == "Update Now", str(focus))

            d.press("Space")
            d.poll(f"{label}: Space on Update Now opens the warning",
                   "!!document.querySelector('[role=alertdialog]')", 15)
            d.press("Enter")
            time.sleep(0.8)
            focus = d.eval_file("focus-info.js")
            d.check(f"{label}: Enter on Cancel closed the warning and returned focus",
                    focus.get("dialogOpen") is False and focus.get("label") == "Update Now", str(focus))
            s = d.state()
            d.check(f"{label}: nothing reloaded, still A with B waiting",
                    s.get("loadMark") == mark and s.get("build") == "A" and s.get("waiting") == "installed")
    finally:
        d.eval_file("remove-invalid.js")
        d.run(["agent-browser", "set", "viewport", "1280", "800"], ok=True)
        d.run(["agent-browser", "set", "media", "light"], ok=True)
    end_session(d)


# ---------------------------------------------------------------------------
# Ticket 010: packaged Geometry tracer + diagnostic hold/fail + takeover + Layout.
# Reuses the A/B stamping, server, session, state and adoption helpers above.
# ---------------------------------------------------------------------------

GEO_BASE_NAME = "Tracer 010"
GEO_BASE_SOURCE = "// 010 baseline"
GEO_DRAFT_SOURCE = "// 010 baseline\n// DRAFT-010"
GEO_MESSAGE = "Geometry scripts has an unapplied edit"
GEO_REMEDY = "Apply the script edit, or cancel the edit in the form"
SAVE_IN_PROGRESS = "A save is still in progress"
DID_NOT_COMPLETE = "The last save did not complete"

ALL_010_LEGS = ["geometry-tracer", "persist-hold", "persist-fail", "persist-hold-fail",
                "takeover-draft", "no-build", "layout-constraint"]


def find_ref_retry(d: Driver, pattern: str, label: str, attempts: int = 6) -> str:
    for _ in range(attempts):
        ref = find_ref(d.snapshot(), pattern)
        if ref is not None:
            return ref
        time.sleep(1)
    d.fail(f"ref not found: {label} /{pattern}/")
    raise Failure(label)


def geo_values(d: Driver) -> dict:
    return d.eval_file("geometry-draft.js") or {}


def save_state(d: Driver) -> dict:
    return d.eval_file("save-state.js") or {}


def gate_snapshot(d: Driver) -> dict:
    return d.eval_file("persist-gate.js") or {}


def gate_arm(d: Driver, mode: str, reason: str = "diagnostic 010") -> None:
    if mode == "hold":
        d.eval_expr("window.__boardstudioPersistGate.arm_hold()")
    elif mode == "fail":
        d.eval_expr(f"window.__boardstudioPersistGate.arm_fail({json.dumps(reason)})")
    elif mode == "hold-then-fail":
        d.eval_expr(f"window.__boardstudioPersistGate.arm_hold_then_fail({json.dumps(reason)})")
    else:
        raise Failure(f"unknown gate mode {mode}")


def gate_release(d: Driver) -> None:
    d.eval_expr("window.__boardstudioPersistGate.release()")


def gate_disarm(d: Driver) -> None:
    d.eval_expr("window.__boardstudioPersistGate.disarm()")


def wait_gate_entered(d: Driver, label: str, timeout_s: int = 30) -> dict:
    deadline = time.time() + timeout_s
    last: dict = {}
    while time.time() < deadline:
        last = gate_snapshot(d)
        if last.get("entered") is True:
            if last.get("acked") is not None:
                d.fail(f"{label}: acknowledged before release", str(last))
            return last
        time.sleep(0.5)
    d.fail(f"{label}: save never entered the gate", str(last))
    raise Failure(f"{label}: no gate entry")


def wait_gate_acked(d: Driver, label: str, timeout_s: int = 30) -> dict:
    deadline = time.time() + timeout_s
    last: dict = {}
    while time.time() < deadline:
        last = gate_snapshot(d)
        if last.get("acked") is not None:
            return last
        time.sleep(0.5)
    d.fail(f"{label}: released save never acknowledged", str(last))
    raise Failure(f"{label}: no ack")


def wait_save(d: Driver, label: str, want: str, timeout_s: int = 60) -> None:
    ok = d.poll(f"{label}: save {want}",
                f"!!document.querySelector('[aria-label=\"{want}\"]')", timeout_s)
    d.check(f"{label}: save shows {want}", ok)


def wait_save_stable(d: Driver, label: str, timeout_s: int = 60) -> None:
    wait_save(d, label, "Saved locally", timeout_s)
    time.sleep(2)
    ss = save_state(d)
    d.check(f"{label}: save stable (Saved, not Saving/Failed)",
            ss.get("saved") is True and not ss.get("saving") and not ss.get("failed"), str(ss))
    g = gate_snapshot(d)
    d.note(f"{label}: gate after stable save: {g}")


def dialog_accept(d: Driver) -> None:
    d.run(["agent-browser", "dialog", "accept"])
    time.sleep(1)


def open_geometry_editor(d: Driver) -> None:
    d.click_button("Add object")
    opener = find_ref_retry(d, 'button "Geometry scripts', "Geometry opener")
    d.run(["agent-browser", "click", opener])


def close_all_toasts(d: Driver, label: str) -> None:
    """Dismiss obscuring non-update toasts, preserving the update toast (if any).
    Synthetic clicks are setup cleanup only; Update Now/Cancel assertions always
    use trusted input."""
    before = (d.state() or {}).get("toastUpdateNow")
    d.eval_expr("Array.from(document.querySelectorAll('[data-slot=toast]'))"
                ".filter(t=>!t.querySelector('[aria-label=\"Update Now\"]'))"
                ".forEach(t=>{const c=t.querySelector('[aria-label=\"close\"]');"
                " if(c) c.click();})")
    time.sleep(1)
    after = (d.state() or {}).get("toastUpdateNow")
    d.check(f"{label}: update toast preserved", before == after,
            f"before={before} after={after}")


def select_baseline_script(d: Driver, label: str, name: str = GEO_BASE_NAME) -> None:
    try:
        d.run(["agent-browser", "click",
                 find_ref_retry(d, 'button "Active script"', "Active script")])
        time.sleep(1)
        opt = find_ref_retry(d, 'option "' + name + '"', "script option")
        d.run(["agent-browser", "click", opt])
        time.sleep(1)
    except Failure:
        d.note(f"{label}: Active script selector not used; reading editor as-is")


def create_baseline_script(d: Driver, label: str, name: str = GEO_BASE_NAME,
                           source: str = GEO_BASE_SOURCE) -> None:
    open_geometry_editor(d)
    d.click_button("+ New script")
    time.sleep(1)
    d.run(["agent-browser", "fill", find_ref_retry(d, 'textbox "Name"', "Name"), name])
    d.run(["agent-browser", "fill", find_ref_retry(d, 'textbox "Rhai source"', "Rhai"), source])
    close_all_toasts(d, label)
    d.click_button("Apply script")
    wait_save(d, label, "Saved locally")
    time.sleep(1)
    gv = geo_values(d)
    d.check(f"{label}: accepted baseline in editor",
            gv.get("name") == name and gv.get("source") == source, str(gv))


def set_geometry_draft(d: Driver, label: str, draft: str) -> None:
    d.run(["agent-browser", "fill", find_ref_retry(d, 'textbox "Rhai source"', "Rhai draft"),
             draft])
    gv = geo_values(d)
    d.check(f"{label}: draft text present, unapplied", gv.get("source") == draft, str(gv))
    d.check(f"{label}: no invalid marker (no accidental blocker)", gv.get("invalid") == 0, str(gv))
    ss = save_state(d)
    d.check(f"{label}: still Saved with draft open (draft is local only)",
            ss.get("saved") is True, str(ss))
    d.cmd(f"{label}: trusted Tab out of Rhai source")
    d.press("Tab")
    gv = geo_values(d)
    d.check(f"{label}: draft survives focus transfer (no blur commit)",
            gv.get("source") == draft, str(gv))


def expect_draft_deferral(d: Driver, label: str, mark: str) -> dict:
    d.poll(f"{label}: deferral dialog", "!!document.querySelector('[role=alertdialog]')", 30)
    s = d.state()
    d.check(f"{label}: warning names Geometry scripts area",
            GEO_MESSAGE in (s.get("dialogText") or ""), str(s.get("dialogText")))
    d.check(f"{label}: warning names remedy",
            GEO_REMEDY in (s.get("dialogText") or ""), str(s.get("dialogText")))
    d.check(f"{label}: no reload, still A, B waiting",
            s.get("loadMark") == mark and s.get("build") == "A"
            and s.get("waiting") == "installed",
            f"mark={s.get('loadMark')} build={s.get('build')} waiting={s.get('waiting')}")
    d.check(f"{label}: editor alive", s.get("editorAlive") is True)
    return s


def cancel_preserves_draft(d: Driver, label: str, mark: str, draft: str) -> None:
    d.click_button("Cancel")
    time.sleep(1)
    s = d.state()
    d.check(f"{label}: warning closed", s.get("dialogText") is None)
    d.check(f"{label}: focus returned to Update Now", s.get("focusOnUpdateNow") is True)
    d.check(f"{label}: still A with B waiting, marker unchanged",
            s.get("build") == "A" and s.get("waiting") == "installed"
            and s.get("loadMark") == mark)
    gv = geo_values(d)
    d.check(f"{label}: exact draft preserved", gv.get("source") == draft, str(gv))


def export_project_document(d: Driver, directory: Path, label: str) -> dict:
    """Observe the accepted document through the user's normal portable export."""
    d.click_button("Export")
    ref = find_ref_retry(d, r'button "Download \.boardstudio project"', "project download")
    archive = directory / f"{label}.boardstudio"
    d.run(["agent-browser", "download", ref, str(archive)])
    with zipfile.ZipFile(archive) as bundle:
        document = json.loads(bundle.read("project.json"))
    d.note(f"{label}: exported project id={document['id']} revision={document['revision']}")
    d.click_button("Back to Layout")
    return document


def layout_fixture(path: Path) -> None:
    """Minimal data only: normal archive import/Runtime acceptance mounts the UI.

    Mirrors ProjectDoc::empty's v7 schema plus a board and two standalone parts;
    unlike matrix/key demos these satisfy layout_component_inspector_projection.
    """
    document = dict(
        format="boardstudio/v2", formatVersion=7, id="layout-010", name="Layout 010",
        revision=0, parameters={},
        definitions=[dict(id="component", name="Fixture controller", kind="controller",
                          courtyard=[dict(x=x, y=y) for x, y in
                                     [(-2, -2), (2, -2), (2, 2), (-2, 2)]], pads=[])],
        parts=[dict(id=id, definitionId="component", reference=ref,
                    pose=dict(at=dict(x=x, y=3), rotation=0), side="front")
               for id, ref, x in [("target", "U1", 4), ("source", "U2", 10)]],
        boards=[dict(id="board", name="Main board", partIds=["target", "source"],
                     outlineIds=[], netIds=[], thickness=1.6, traces=[], vias=[])],
        matrices=[], nets=[], outline=[], caseBodies=[], materials=[], assets=[],
        scripts=[], constraints=[])
    with zipfile.ZipFile(path, "w") as bundle:
        bundle.writestr("project.json", json.dumps(document))


def select_layout_constraint(d: Driver) -> dict:
    ref = find_ref_retry(d, r'button "U1 Fixture controller', "standalone U1 Objects button")
    d.run(["agent-browser", "click", ref])
    form = d.eval_file("layout-constraint.js") or {}
    if not form.get("present"):
        raise Failure("selected standalone part did not mount Layout constraint")
    if not form.get("expanded"):
        d.click_selector('[aria-label="Layout constraint"]')
    return d.eval_file("layout-constraint.js") or {}


def leg_layout_constraint(ctx: Ctx) -> None:
    d = ctx.d
    d.cmd("LEG layout-constraint: normal import, real draft deferral, saved constraint in B")
    ctx.server.swap(d, ctx.a, "serve A")
    # Explicitly owned disk profile: includes IDB/cache/worker, removed only after
    # successful close. Never clear workers/caches to obtain GREEN.
    directory = make_session_dir(d, "boardstudio-010-layout-")
    try:
        fixture = directory / "layout.boardstudio"
        layout_fixture(fixture)
        start_session(d, "010-layout", route_url(d.port, "root"), directory / "profile")
        d.run(["agent-browser", "upload", "#library-import-file", str(fixture)])
        d.wait_main_board()
        wait_save(d, "layout import", "Saved locally")
        form = select_layout_constraint(d)
        d.check("layout: real empty constraint mounted", form.get("saveLabel") == "Add constraint"
                and form.get("x") == "0", str(form))
        mark = set_marker(d)
        d.run(["agent-browser", "fill", '[aria-label="Offset X (mm)"]', "7.25"])
        d.press("Tab")
        form = d.eval_file("layout-constraint.js") or {}
        d.check("layout: valid unapplied draft survives blur", form.get("x") == "7.25"
                and form.get("invalid") == 0 and save_state(d).get("saved") is True, str(form))
        ctx.server.swap(d, ctx.b, "publish B at the same origin")
        publish_and_wait_waiting(d, "layout B published")
        for label, selector in [("toast", UPDATE_NOW_TOAST), ("header", UPDATE_NOW_HEADER)]:
            if label == "header":
                close_toast_with_keyboard(d)
            d.click_selector(selector)
            dialog = d.poll(f"layout {label}: draft defers",
                            "!!document.querySelector('[role=alertdialog]')", 15)
            d.check(f"layout {label}: unresolved draft blocks adoption", dialog)
            if not dialog:
                d.note(f"layout missing deferral outcome: {d.state()}")
                raise Failure("Layout draft adopted instead of deferring")
            state = d.state()
            d.check(f"layout {label}: named owner and resolution remedy",
                    "Layout constraint has an unapplied edit" in (state.get("dialogText") or "")
                    and "Save or remove the constraint, or restore its saved values"
                    in (state.get("dialogText") or ""), str(state.get("dialogText")))
            d.click_button("Cancel")
            state = expect_waiting_state(d, f"layout {label} Cancel", mark, "A")
            form = d.eval_file("layout-constraint.js") or {}
            d.check(f"layout {label}: Cancel preserves exact draft and focus",
                    form.get("x") == "7.25" and state.get("focusOnUpdateNow") is True, str(form))
        close_all_toasts(d, "layout resolve")
        gate_arm(d, "hold")
        d.click_button("Add constraint")
        entered = wait_gate_entered(d, "layout Add constraint")
        d.check("layout: held intended project/revision with no ack",
                entered.get("document_id") == "layout-010" and entered.get("revision") == 1
                and entered.get("acked") is None and save_state(d).get("saving") is True,
                str(entered))
        d.click_selector(UPDATE_NOW_HEADER)
        expect_save_deferral(d, "layout held", mark, SAVE_IN_PROGRESS)
        cancel_save_deferral(d, "layout held Cancel", mark)
        gate_release(d)
        ack = wait_gate_acked(d, "layout Add constraint")
        d.check("layout: real store completed after release", ack.get("acked") == "completed",
                str(ack))
        wait_save(d, "layout resolved", "Saved locally")
        saved_a = export_project_document(d, directory, "layout-A")
        d.check("layout A: exact accepted constraint and revision",
                saved_a["revision"] == 1 and len(saved_a["constraints"]) == 1
                and saved_a["constraints"][0].get("source") == {"kind": "part", "partId": "source"}
                and saved_a["constraints"][0].get("target") == {"kind": "part", "partId": "target"}
                and saved_a["constraints"][0].get("relation") == {
                    "kind": "offset", "offset": {"x": 7.25, "y": 0}, "rotation": 0},
                str(saved_a["constraints"]))
        d.click_selector(UPDATE_NOW_HEADER)
        wait_for_adoption_reload(d, "layout adopt")
        expect_adopted(d, "layout adopted B", "B", {ctx.names("root", "B"): ctx.vb["root"]})
        reopen_saved_project(d, "layout adopted B")
        form = select_layout_constraint(d)
        d.check("layout B: reopened real form holds saved constraint",
                form.get("x") == "7.25" and form.get("saveLabel") == "Save constraint", str(form))
        saved_b = export_project_document(d, directory, "layout-B")
        d.check("layout B: exact reopened project/value/revision matches A",
                saved_b == saved_a and saved_b["id"] == "layout-010", str(saved_b["constraints"]))
    finally:
        cleanup_owned_profile(d, directory, "layout")


def leg_geometry_tracer(ctx: Ctx) -> None:
    d = ctx.d
    d.cmd("LEG geometry-tracer: real Geometry draft defers toast+header, Apply+Saved adopts B")
    ctx.server.swap(d, ctx.a, "serve A")
    start_session(d, "010-tracer", route_url(d.port, "root"))
    open_demo(d)
    mark = set_marker(d)
    s = d.state()
    d.check("tracer A: controlled, no waiting, document A",
            bool(s.get("controller")) and s.get("waiting") is None and s.get("build") == "A")
    g0 = gate_snapshot(d)
    d.check("tracer A: diagnostic gate present (diagnostic build)", g0.get("present") is True,
            str(g0))
    create_baseline_script(d, "tracer baseline")
    set_geometry_draft(d, "tracer draft", GEO_DRAFT_SOURCE)
    ctx.server.swap(d, ctx.b, "publish B at the same origin")
    publish_and_wait_waiting(d, "tracer B published")
    expect_waiting_state(d, "tracer A with B waiting", mark, "A")
    gv = geo_values(d)
    d.check("tracer: draft still present with B waiting",
            gv.get("source") == GEO_DRAFT_SOURCE, str(gv))
    d.cmd("toast pointer: click Update Now with draft open")
    d.click_selector(UPDATE_NOW_TOAST)
    expect_draft_deferral(d, "toast pointer defers", mark)
    cancel_preserves_draft(d, "toast pointer cancel", mark, GEO_DRAFT_SOURCE)
    d.cmd("toast keyboard: Tab to Update Now, Enter opens, Space cancels")
    prep = d.eval_file("tab-prep.js")
    d.check("toast keyboard: Update Now in tab sequence",
            prep.get("found") is True and prep.get("positive") == 0, str(prep))
    d.press("Tab")
    focus = d.eval_file("focus-info.js")
    d.check("toast keyboard: Tab lands on Update Now",
            focus.get("label") == "Update Now" and focus.get("inToast") is True, str(focus))
    d.press("Enter")
    expect_draft_deferral(d, "toast Enter defers", mark)
    d.press("Space")
    time.sleep(1)
    s = d.state()
    d.check("toast Space-cancel: warning closed", s.get("dialogText") is None)
    d.check("toast Space-cancel: focus on Update Now", s.get("focusOnUpdateNow") is True)
    gv = geo_values(d)
    d.check("toast Space-cancel: draft preserved", gv.get("source") == GEO_DRAFT_SOURCE)
    d.cmd("toast keyboard: Space on Update Now opens, Enter on Cancel closes")
    d.press("Space")
    expect_draft_deferral(d, "toast Space defers", mark)
    d.press("Enter")
    time.sleep(1)
    s = d.state()
    d.check("toast Enter-cancel: closed and focus on Update Now",
            s.get("dialogText") is None and s.get("focusOnUpdateNow") is True)
    close_toast_with_keyboard(d)
    s = expect_waiting_state(d, "toast dismissed", mark, "A")
    d.check("header offered after dismissal", s.get("headerUpdateNow") is True)
    d.cmd("header pointer: click Update Now with same draft")
    d.click_selector(UPDATE_NOW_HEADER)
    expect_draft_deferral(d, "header pointer defers", mark)
    cancel_preserves_draft(d, "header pointer cancel", mark, GEO_DRAFT_SOURCE)
    d.cmd("header keyboard: Tab to header Update Now, Enter opens, Space cancels")
    prep = d.eval_file("tab-prep-header.js")
    d.check("header keyboard: Update Now in tab sequence",
            prep.get("found") is True and prep.get("positive") == 0, str(prep))
    d.press("Tab")
    focus = d.eval_file("focus-info.js")
    d.check("header keyboard: Tab lands on header Update Now",
            focus.get("label") == "Update Now" and not focus.get("inToast"), str(focus))
    d.press("Enter")
    expect_draft_deferral(d, "header Enter defers", mark)
    d.press("Space")
    time.sleep(1)
    s = d.state()
    d.check("header Space-cancel: closed, focus on Update Now, still A",
            s.get("dialogText") is None and s.get("focusOnUpdateNow") is True
            and s.get("build") == "A" and s.get("loadMark") == mark)
    d.cmd("header keyboard: Space on Update Now opens, Enter on Cancel closes")
    d.press("Space")
    expect_draft_deferral(d, "header Space defers", mark)
    d.press("Enter")
    time.sleep(1)
    s = d.state()
    d.check("header Enter-cancel: closed, focus returned, exact draft preserved",
            s.get("dialogText") is None and s.get("focusOnUpdateNow") is True
            and geo_values(d).get("source") == GEO_DRAFT_SOURCE)
    d.cmd("resolve: arm hold, Apply draft, capture revision, release, Saved")
    close_all_toasts(d, "tracer resolve")
    gate_arm(d, "hold")
    d.click_button("Apply script")
    entered = wait_gate_entered(d, "tracer Apply entered")
    rev = entered.get("revision")
    d.note(f"tracer intended revision: {rev} name={entered.get('document_name')}")
    d.check("tracer: entered intended project/revision with expected token",
            bool(entered.get("document_id")) and rev == 3
            and entered.get("expected_token_present") is True, str(entered))
    ss = save_state(d)
    d.check("tracer held: Saving, no ack yet", ss.get("saving") is True, str(ss))
    gate_release(d)
    acked = wait_gate_acked(d, "tracer Apply acked")
    d.check("tracer: released save completed", acked.get("acked") == "completed", str(acked))
    wait_save(d, "tracer resolved", "Saved locally")
    gv = geo_values(d)
    d.check("tracer: applied value in editor", gv.get("source") == GEO_DRAFT_SOURCE, str(gv))
    d.cmd("adopt from header (toast dismissed) after Apply+Saved")
    d.click_selector(UPDATE_NOW_HEADER)
    wait_for_adoption_reload(d, "tracer adopt")
    expect_adopted(d, "tracer adopted B", "B", {ctx.names("root", "B"): ctx.vb["root"]})
    reopen_saved_project(d, "tracer adopted B")
    open_geometry_editor(d)
    select_baseline_script(d, "tracer")
    gv = geo_values(d)
    d.check("tracer: reopened project holds intended Name", gv.get("name") == GEO_BASE_NAME,
            str(gv))
    d.check("tracer: reopened editor holds intended value",
            gv.get("source") == GEO_DRAFT_SOURCE, str(gv))
    check_reopened_geometry_save(d, "tracer-B", entered, GEO_DRAFT_SOURCE)
    end_session(d)


def expect_save_deferral(d: Driver, label: str, mark: str, message: str) -> dict:
    d.poll(f"{label}: deferral dialog", "!!document.querySelector('[role=alertdialog]')", 30)
    s = d.state()
    d.check(f"{label}: warning names save cause",
            message in (s.get("dialogText") or ""), str(s.get("dialogText")))
    d.check(f"{label}: no reload, still A, B waiting",
            s.get("loadMark") == mark and s.get("build") == "A"
            and s.get("waiting") == "installed")
    return s


def cancel_save_deferral(d: Driver, label: str, mark: str) -> None:
    d.click_button("Cancel")
    time.sleep(1)
    s = d.state()
    d.check(f"{label}: warning closed", s.get("dialogText") is None)
    d.check(f"{label}: still A with B waiting, marker unchanged",
            s.get("build") == "A" and s.get("waiting") == "installed"
            and s.get("loadMark") == mark)


def leg_persist_hold(ctx: Ctx) -> None:
    d = ctx.d
    d.cmd("LEG persist-hold: held save defers as save-in-progress, release adopts, B reuses gate")
    ctx.server.swap(d, ctx.a, "serve A")
    start_session(d, "010-hold", route_url(d.port, "root"))
    open_demo(d)
    mark = set_marker(d)
    g0 = gate_snapshot(d)
    d.check("hold A: gate present", g0.get("present") is True, str(g0))
    create_baseline_script(d, "hold baseline")
    v1 = GEO_BASE_SOURCE + "\n// HOLD-010"
    d.cmd("hold: arm, Apply v1 (held, no ack)")
    gate_arm(d, "hold")
    d.run(["agent-browser", "fill", find_ref_retry(d, 'textbox "Rhai source"', "Rhai v1"), v1])
    close_all_toasts(d, "hold v1")
    d.click_button("Apply script")
    entered = wait_gate_entered(d, "hold v1 entered")
    rev1 = entered.get("revision")
    d.note(f"hold intended revision: {rev1}")
    ss = save_state(d)
    d.check("hold: Saving, no ack, no durable change yet",
            ss.get("saving") is True and entered.get("acked") is None, str(ss))
    ctx.server.swap(d, ctx.b, "publish B at the same origin")
    publish_and_wait_waiting(d, "hold B published")
    expect_waiting_state(d, "hold A with B waiting", mark, "A")
    d.cmd("hold: Update Now defers as save-in-progress")
    d.click_selector(UPDATE_NOW_TOAST)
    expect_save_deferral(d, "hold defers", mark, SAVE_IN_PROGRESS)
    cancel_save_deferral(d, "hold cancel", mark)
    ss = save_state(d)
    d.check("hold cancel: still Saving, still held", ss.get("saving") is True, str(ss))
    d.cmd("hold: release, acked, Saved, adopt")
    gate_release(d)
    acked = wait_gate_acked(d, "hold v1 acked")
    d.check("hold: released save completed", acked.get("acked") == "completed", str(acked))
    wait_save(d, "hold resolved", "Saved locally")
    gv = geo_values(d)
    d.check("hold: applied v1 in editor", gv.get("source") == v1, str(gv))
    d.click_selector(UPDATE_NOW_TOAST)
    wait_for_adoption_reload(d, "hold adopt")
    expect_adopted(d, "hold adopted B", "B", {ctx.names("root", "B"): ctx.vb["root"]})
    reopen_saved_project(d, "hold adopted B")
    open_geometry_editor(d)
    select_baseline_script(d, "hold")
    gv = geo_values(d)
    d.note(f"hold B editor: {gv}")
    d.check("hold: B reopens intended value", gv.get("source") == v1, str(gv))
    d.check("hold: intended save is revision 3", rev1 == 3, str(entered))
    check_reopened_geometry_save(d, "hold-B", entered, v1)
    open_geometry_editor(d)
    select_baseline_script(d, "hold after export")
    d.cmd("hold: gate consumed again after takeover in B")
    g1 = gate_snapshot(d)
    d.check("hold B: gate present in B", g1.get("present") is True, str(g1))
    v2 = v1 + "\n// HOLD-B-010"
    gate_arm(d, "hold")
    d.run(["agent-browser", "fill", find_ref_retry(d, 'textbox "Rhai source"', "Rhai v2"), v2])
    close_all_toasts(d, "hold B v2")
    d.click_button("Apply script")
    entered2 = wait_gate_entered(d, "hold B v2 entered")
    d.check("hold B: second hold entered", entered2.get("entered") is True, str(entered2))
    gate_release(d)
    acked2 = wait_gate_acked(d, "hold B v2 acked")
    d.check("hold B: second hold completed", acked2.get("acked") == "completed", str(acked2))
    wait_save(d, "hold B resolved", "Saved locally")
    gv = geo_values(d)
    d.check("hold B: v2 saved in B", gv.get("source") == v2, str(gv))
    end_session(d)


def retry_geometry_save(d: Driver, label: str, failed: dict, intended: str) -> dict:
    """Literal Retry, with no navigation/reselection/export to repair the mounted form."""
    d.cmd(f"{label}: literal Retry save, held at real persistence entry")
    d.check(f"{label}: failure rolled the form back", geo_values(d).get("source") == GEO_BASE_SOURCE)
    gate_arm(d, "hold")
    d.click_button("Retry save")
    entered = wait_gate_entered(d, f"{label} Retry entered")
    d.check(f"{label}: retry holds intended revision with no acknowledgement",
            bool(entered.get("document_id")) and entered.get("document_name") == "Split 42"
            and entered.get("revision") == 3 and entered.get("expected_token_present") is True
            and entered.get("acked") is None and save_state(d).get("saving") is True,
            str(entered))
    # Immediate fail deliberately never enters persistence, so its snapshot has
    # no document metadata. Hold-then-fail did enter and must retain that identity.
    if failed.get("entered"):
        d.check(f"{label}: retry retains failed attempt's exact project/revision",
                entered.get("document_id") == failed.get("document_id")
                and entered.get("revision") == failed.get("revision"), str(failed))
    gate_release(d)
    ack = wait_gate_acked(d, f"{label} Retry acked")
    d.check(f"{label}: real retry store completed", ack.get("acked") == "completed", str(ack))
    wait_save(d, f"{label} Retry", "Saved locally")
    gv = geo_values(d)
    d.check(f"{label}: retry reconciles still-mounted form", gv.get("source") == intended, str(gv))
    return entered


def check_reopened_geometry_save(d: Driver, label: str, entered: dict, intended: str) -> None:
    # Only AFTER direct adoption: normal project export independently observes the
    # exact saved revision, not a save label or the gate's proposed document.
    # Keep the archive with the session's other owned outputs: delete only after
    # confirmed close, or retain it with the profile for --keep-artifacts/uncertain close.
    if d.session_dir is None:
        raise Failure("geometry export requires an owned session directory")
    document = export_project_document(d, d.session_dir, label)
    script = next((s for s in document["scripts"] if s["name"] == GEO_BASE_NAME), None)
    d.check(f"{label}: exact reopened saved project/source/revision",
            document["id"] == entered.get("document_id")
            and document["revision"] == entered.get("revision")
            and script is not None and script["source"] == intended,
            f"id={document['id']} revision={document['revision']} script={script}")


def leg_persist_fail(ctx: Ctx) -> None:
    d = ctx.d
    d.cmd("LEG persist-fail: failed save defers as did-not-complete, explicit retry adopts")
    ctx.server.swap(d, ctx.a, "serve A")
    start_session(d, "010-fail", route_url(d.port, "root"))
    open_demo(d)
    mark = set_marker(d)
    create_baseline_script(d, "fail baseline")
    v1 = GEO_BASE_SOURCE + "\n// FAIL-010"
    d.cmd("fail: arm fail, Apply v1 (fails, no durable write)")
    gate_arm(d, "fail", "diagnostic quota")
    d.run(["agent-browser", "fill", find_ref_retry(d, 'textbox "Rhai source"', "Rhai v1"), v1])
    close_all_toasts(d, "fail v1")
    d.click_button("Apply script")
    ok = d.poll("fail: save Failed", "!!document.querySelector('[aria-label=\"Local save failed\"]')", 30)
    d.check("fail: save shows Failed", ok)
    g1 = gate_snapshot(d)
    d.check("fail: gate acked aborted", (g1.get("acked") or "").startswith("aborted"), str(g1))
    ctx.server.swap(d, ctx.b, "publish B at the same origin")
    publish_and_wait_waiting(d, "fail B published")
    expect_waiting_state(d, "fail A with B waiting", mark, "A")
    d.cmd("fail: Update Now defers as did-not-complete")
    d.click_selector(UPDATE_NOW_TOAST)
    expect_save_deferral(d, "fail defers", mark, DID_NOT_COMPLETE)
    cancel_save_deferral(d, "fail cancel", mark)
    entered = retry_geometry_save(d, "fail", g1, v1)
    d.cmd("fail: direct Retry → Update Now (no export/navigation/reselection/re-apply)")
    d.click_selector(UPDATE_NOW_TOAST)
    wait_for_adoption_reload(d, "fail adopt")
    if d.state().get("build") != "B":
        raise Failure("literal Retry did not permit direct adoption")
    expect_adopted(d, "fail adopted B", "B", {ctx.names("root", "B"): ctx.vb["root"]})
    reopen_saved_project(d, "fail adopted B")
    open_geometry_editor(d)
    select_baseline_script(d, "fail")
    gv = geo_values(d)
    d.check("fail: B reopens retry value", gv.get("source") == v1, str(gv))
    check_reopened_geometry_save(d, "fail-B", entered, v1)
    end_session(d)


def leg_persist_hold_fail(ctx: Ctx) -> None:
    d = ctx.d
    d.cmd("LEG persist-hold-fail: single-armed hold-then-fail, release fails atomically, retry adopts")
    ctx.server.swap(d, ctx.a, "serve A")
    start_session(d, "010-holdfail", route_url(d.port, "root"))
    open_demo(d)
    mark = set_marker(d)
    create_baseline_script(d, "holdfail baseline")
    v1 = GEO_BASE_SOURCE + "\n// HOLDFAIL-010"
    d.cmd("holdfail: arm hold-then-fail, Apply v1 (held)")
    gate_arm(d, "hold-then-fail", "diagnostic quota")
    d.run(["agent-browser", "fill", find_ref_retry(d, 'textbox "Rhai source"', "Rhai v1"), v1])
    close_all_toasts(d, "holdfail v1")
    d.click_button("Apply script")
    entered = wait_gate_entered(d, "holdfail v1 entered")
    d.check("holdfail: held, no ack yet", entered.get("acked") is None, str(entered))
    ctx.server.swap(d, ctx.b, "publish B at the same origin")
    publish_and_wait_waiting(d, "holdfail B published")
    expect_waiting_state(d, "holdfail A with B waiting", mark, "A")
    d.cmd("holdfail: Update Now defers as save-in-progress while held")
    d.click_selector(UPDATE_NOW_TOAST)
    expect_save_deferral(d, "holdfail held defers", mark, SAVE_IN_PROGRESS)
    cancel_save_deferral(d, "holdfail held cancel", mark)
    d.cmd("holdfail: release enters failure atomically (no re-arm)")
    gate_release(d)
    acked = wait_gate_acked(d, "holdfail v1 acked")
    d.check("holdfail: released into failure atomically",
            (acked.get("acked") or "").startswith("aborted") and acked.get("released") is True,
            str(acked))
    ok = d.poll("holdfail: save Failed", "!!document.querySelector('[aria-label=\"Local save failed\"]')", 30)
    d.check("holdfail: save shows Failed after release", ok)
    d.cmd("holdfail: Update Now defers as did-not-complete after failure")
    d.click_selector(UPDATE_NOW_TOAST)
    expect_save_deferral(d, "holdfail failed defers", mark, DID_NOT_COMPLETE)
    cancel_save_deferral(d, "holdfail failed cancel", mark)
    entered = retry_geometry_save(d, "holdfail", acked, v1)
    d.cmd("holdfail: direct Retry → Update Now (no export/navigation/reselection/re-apply)")
    d.click_selector(UPDATE_NOW_TOAST)
    wait_for_adoption_reload(d, "holdfail adopt")
    if d.state().get("build") != "B":
        raise Failure("literal Retry did not permit direct adoption")
    expect_adopted(d, "holdfail adopted B", "B", {ctx.names("root", "B"): ctx.vb["root"]})
    reopen_saved_project(d, "holdfail adopted B")
    open_geometry_editor(d)
    select_baseline_script(d, "holdfail")
    gv = geo_values(d)
    d.check("holdfail: B reopens retry value", gv.get("source") == v1, str(gv))
    check_reopened_geometry_save(d, "holdfail-B", entered, v1)
    end_session(d)


def leg_takeover_draft(ctx: Ctx) -> None:
    d = ctx.d
    d.cmd("LEG takeover-draft: newer input while an Update Now attempt waits on a held save")
    ctx.server.swap(d, ctx.a, "serve A")
    directory = make_session_dir(d, "boardstudio-010-takeover-")
    try:
        start_session(d, "010-takeover", route_url(d.port, "root"), directory / "profile")
        open_demo(d)
        mark = set_marker(d)
        create_baseline_script(d, "takeover baseline")
        ctx.server.swap(d, ctx.b, "publish B at the same origin")
        publish_and_wait_waiting(d, "takeover B published")
        expect_waiting_state(d, "takeover clean with B waiting", mark, "A")
        held_value = GEO_BASE_SOURCE + "\n// TAKEOVER-HELD-010"
        newer_value = held_value + "\n// TAKEOVER-NEWER-010"
        set_geometry_draft(d, "takeover first edit", held_value)
        close_all_toasts(d, "takeover first Apply")
        gate_arm(d, "hold")
        d.click_button("Apply script")
        entered = wait_gate_entered(d, "takeover first Apply")
        d.check("takeover: intended r3 is held with no acknowledgement",
                bool(entered.get("document_id")) and entered.get("revision") == 3
                and entered.get("acked") is None and save_state(d).get("saving") is True,
                str(entered))
        d.click_selector(UPDATE_NOW_TOAST)
        expect_save_deferral(d, "takeover first attempt", mark, SAVE_IN_PROGRESS)
        cancel_save_deferral(d, "takeover first attempt Cancel", mark)
        # The earlier attempt is deferred, not a post-permit worker-probe race.
        # Real input arrives before its outstanding save is released.
        d.run(["agent-browser", "fill", find_ref_retry(d, 'textbox "Rhai source"', "newer takeover input"),
               newer_value])
        d.press("Tab")
        gv = geo_values(d)
        held = gate_snapshot(d)
        d.check("takeover: newer valid input exists while the earlier save remains unanswered",
                gv.get("source") == newer_value and gv.get("invalid") == 0
                and held.get("acked") is None and held.get("released") is False
                and save_state(d).get("saving") is True, f"draft={gv} gate={held}")
        gate_release(d)
        ack = wait_gate_acked(d, "takeover older Apply")
        d.check("takeover: older save completed", ack.get("acked") == "completed", str(ack))
        wait_save(d, "takeover older Apply", "Saved locally")
        d.check("takeover: older landing preserves newer input",
                geo_values(d).get("source") == newer_value, str(geo_values(d)))
        d.click_selector(UPDATE_NOW_TOAST)
        expect_draft_deferral(d, "takeover newer draft defers again after save settles", mark)
        cancel_preserves_draft(d, "takeover newer draft Cancel", mark, newer_value)
        close_all_toasts(d, "takeover resolve newer draft")
        gate_arm(d, "hold")
        d.click_button("Apply script")
        final_entry = wait_gate_entered(d, "takeover newer Apply")
        d.check("takeover: newer draft submits the same project at intended r4",
                final_entry.get("document_id") == entered.get("document_id")
                and final_entry.get("revision") == 4 and final_entry.get("acked") is None,
                str(final_entry))
        gate_release(d)
        final_ack = wait_gate_acked(d, "takeover newer Apply")
        d.check("takeover: newer save completed", final_ack.get("acked") == "completed", str(final_ack))
        wait_save(d, "takeover resolved", "Saved locally")
        d.click_selector(UPDATE_NOW_TOAST)
        wait_for_adoption_reload(d, "takeover adopt")
        expect_adopted(d, "takeover adopted B", "B", {ctx.names("root", "B"): ctx.vb["root"]})
        reopen_saved_project(d, "takeover adopted B")
        open_geometry_editor(d)
        select_baseline_script(d, "takeover")
        d.check("takeover: B reopens the newer value", geo_values(d).get("source") == newer_value)
        check_reopened_geometry_save(d, "takeover-B", final_entry, newer_value)
    finally:
        cleanup_owned_profile(d, directory, "takeover")


def leg_no_build(ctx: Ctx) -> None:
    d = ctx.d
    d.cmd("LEG no-build: no new build published, no Update Now, no adoption")
    ctx.server.swap(d, ctx.a, "serve A only")
    start_session(d, "010-nobuild", route_url(d.port, "root"))
    open_demo(d)
    mark = set_marker(d)
    s = d.state()
    d.check("no-build A: controlled, no waiting, document A",
            bool(s.get("controller")) and s.get("waiting") is None and s.get("build") == "A")
    d.check("no-build A: no Update Now offered",
            not s.get("toastUpdateNow") and not s.get("headerUpdateNow"))
    result = d.eval_file("update-check.js")
    d.check("no-build: update check finds no waiting", result.get("waiting") is None, str(result))
    d.check("no-build: no notice shown", result.get("notice") is not True, str(result))
    time.sleep(3)
    s = d.state()
    d.check("no-build: still A, no waiting, no reload",
            s.get("build") == "A" and s.get("waiting") is None and s.get("loadMark") == mark)
    d.check("no-build: still no Update Now",
            not s.get("toastUpdateNow") and not s.get("headerUpdateNow"))
    d.check("no-build: editor alive", s.get("editorAlive") is True)
    end_session(d)


def worker_has_adopt_protocol(build: Path) -> bool:
    """True when the packaged worker wasm imports skipWaiting (the 019 adoption path)."""
    import base64
    text = (site_dir(build, "root") / "service-worker.js").read_text()
    match = re.search(r'atob\("([^"]+)"\)', text)
    if not match:
        return False
    return ADOPT_IMPORT in base64.b64decode(match.group(1))


LEGS = {
    "adopt": leg_adopt,
    "second-editor": leg_second_editor,
    "subpath": leg_subpath,
    "failed-install": leg_failed_install,
    "unavailable-target": leg_unavailable_target,
    "late-adopt": leg_late_adopt,
    "draft-safety": leg_draft_safety,
    "keyboard-matrix": leg_keyboard_matrix,
    "geometry-tracer": leg_geometry_tracer,
    "persist-hold": leg_persist_hold,
    "persist-fail": leg_persist_fail,
    "persist-hold-fail": leg_persist_hold_fail,
    "takeover-draft": leg_takeover_draft,
    "no-build": leg_no_build,
    "layout-constraint": leg_layout_constraint,
}


# ---------------------------------------------------------------------------
# RED control: a real pre-019 packaged pair has no Update Now action.
# ---------------------------------------------------------------------------

def scenario_pre(driver: Driver, server: Server, legacy_a: Path, legacy_b: Path, log) -> None:
    driver.cmd("RED control: pre-019 packaged pair must offer Update Now (it cannot)")
    server.swap(driver, legacy_a, "serve pre-019 A")
    served_a = serve_version(driver.port)
    start_session(driver, "019-pre", route_url(driver.port, "root"))
    open_demo(driver)
    mark = set_marker(driver)
    server.swap(driver, legacy_b, "publish pre-019 B")
    served_b = serve_version(driver.port)
    driver.check("pre-019 A and B are distinct releases", served_a != served_b,
                 f"A={served_a} B={served_b}")
    driver.check("control build lacks the adoption protocol",
                 not worker_has_adopt_protocol(legacy_b))
    result = driver.eval_file("update.js")
    driver.check("pre-019: waiting worker present (018 behavior)", result.get("waiting") == "PRESENT")
    driver.check("pre-019: notice shown (018 behavior)", result.get("notice") is True)
    s = driver.state()
    driver.check("RED: toast offers Update Now", s.get("toastUpdateNow") is True,
                 str(s.get("allStatuses")))
    close_toast_with_keyboard(driver)
    s = driver.state()
    driver.check("RED: header notice offers Update Now after dismissal",
                 s.get("headerUpdateNow") is True, f"headerNotice={s.get('headerNotice')}")
    time.sleep(3)
    s = driver.state()
    driver.check("RED: adoption happened (B active, waiting cleared)",
                 s.get("waiting") is None and s.get("loadMark") != mark,
                 f"waiting={s.get('waiting')} loadMark={s.get('loadMark')}")
    end_session(driver)


# ---------------------------------------------------------------------------
# 010 RED: a real pre-tracer packaged pair (002 builds) has Update Now but no
# draft deferral. The tracer assertions must FAIL here (adoption + loss) and
# PASS on current builds. Control only: no 008 code exists in those builds.
# ---------------------------------------------------------------------------

def scenario_pre010(driver: Driver, server: Server, legacy_a: Path, legacy_b: Path) -> None:
    driver.cmd("010 RED: pre-tracer A/B must defer the Geometry draft (it cannot)")
    server.swap(driver, legacy_a, "serve pre-tracer A")
    served_a = serve_version(driver.port)
    start_session(driver, "010-pre", route_url(driver.port, "root"))
    open_demo(driver)
    mark = set_marker(driver)
    server.swap(driver, legacy_b, "publish pre-tracer B")
    served_b = serve_version(driver.port)
    driver.check("pre-tracer A and B are distinct releases", served_a != served_b,
                 f"A={served_a} B={served_b}")
    driver.check("control build has the adoption protocol (019 present)",
                 worker_has_adopt_protocol(legacy_b))
    g0 = gate_snapshot(driver)
    driver.check("control build lacks the diagnostic gate (pre-009)",
                 g0.get("present") is not True, str(g0))
    create_baseline_script(driver, "pre010 baseline")
    set_geometry_draft(driver, "pre010 draft", GEO_DRAFT_SOURCE)
    publish_and_wait_waiting(driver, "pre010 B published")
    expect_waiting_state(driver, "pre010 A with B waiting", mark, "A")
    d = driver
    d.cmd("RED: toast Update Now must defer (pre-tracer adopts instead)")
    d.click_selector(UPDATE_NOW_TOAST)
    outcome = None
    deadline = time.time() + 20
    while time.time() < deadline:
        s = d.state()
        if s.get("dialogText"):
            outcome = "dialog"
            break
        if s.get("loadMark") != mark:
            outcome = "reloaded"
            break
        time.sleep(0.5)
    d.note(f"pre010 toast outcome: {outcome}")
    d.check("RED: toast defers with Geometry warning", outcome == "dialog", str(outcome))
    if outcome == "dialog":
        s = d.state()
        d.check("RED: warning names Geometry area", GEO_MESSAGE in (s.get("dialogText") or ""))
        d.click_button("Cancel")
        time.sleep(1)
        gv = geo_values(d)
        d.check("RED: Cancel preserves draft", gv.get("source") == GEO_DRAFT_SOURCE, str(gv))
    elif outcome == "reloaded":
        wait_for_adoption_reload(d, "pre010 loss")
        reopen_saved_project(d, "pre010 adopted")
        open_geometry_editor(d)
        select_baseline_script(d, "pre010")
        gv = geo_values(d)
        d.check("RED: draft preserved after adoption (it is lost pre-tracer)",
                gv.get("source") == GEO_DRAFT_SOURCE, str(gv))
    else:
        d.fail("pre010: neither dialog nor reload within 20s")
    end_session(driver)


# ---------------------------------------------------------------------------

def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("mode", choices=["pre", "post", "pre010"],
                        help="pre: pre-019 RED control; post: GREEN legs; "
                             "pre010: pre-tracer (002) RED for the 010 Geometry assertions")
    parser.add_argument("--port", type=int, default=4733)
    parser.add_argument("--src-dir", type=Path, default=ROOT / "web" / "target" / "site-019-a",
                        help="one real build-web.py output (--routes root,subpath)")
    parser.add_argument("--work-dir", type=Path, default=None,
                        help="output workspace for the stamped A/B copies: a path that does not "
                             "exist yet, created and removed by this command (default: a new "
                             "unique directory under the git-ignored web/target)")
    parser.add_argument("--legacy-a-dir", type=Path, default=ROOT / "web" / "target" / "site-018-a")
    parser.add_argument("--legacy-b-dir", type=Path, default=ROOT / "web" / "target" / "site-018-b")
    parser.add_argument("--legacy010-a-dir", type=Path,
                        default=ROOT / "web" / "target" / "site-002-ab" / "a",
                        help="pre-tracer A for pre010 RED (immutable 002 artifact)")
    parser.add_argument("--legacy010-b-dir", type=Path,
                        default=ROOT / "web" / "target" / "site-002-ab" / "b",
                        help="pre-tracer B for pre010 RED (immutable 002 artifact)")
    parser.add_argument("--legs", default=",".join(ALL_LEGS), help="comma-separated subset of legs")
    parser.add_argument("--keep-artifacts", action="store_true",
                        help="diagnosis only: keep this run's generated outputs and browser "
                             "profiles (after the usual confirmed close) and print their paths; "
                             "the run exits 3 and is never an acceptance pass")
    parser.add_argument("--serve-log", type=Path,
                        default=ROOT / ".evidence" / "ui-integration-recovery" / "019-adoption-serve.log")
    args = parser.parse_args(argv)
    if shutil.which("agent-browser") is None:
        print("agent-browser CLI not found", flush=True)
        return 2
    refusal = inherited_settings_refusal()
    if refusal:
        print(refusal, flush=True)
        return 2
    if not port_free(args.port):
        print(f"dedicated origin port {args.port} is occupied; refusing to disturb it", flush=True)
        return 2
    legs = [leg.strip() for leg in args.legs.split(",") if leg.strip()]
    unknown = [leg for leg in legs if leg not in LEGS]
    if unknown:
        print(f"unknown legs: {unknown}", flush=True)
        return 2
    if args.mode == "pre":
        needed = [args.legacy_a_dir, args.legacy_b_dir]
    elif args.mode == "pre010":
        needed = [args.legacy010_a_dir, args.legacy010_b_dir]
    else:
        needed = [args.src_dir]
    for build in needed:
        for route in (["root"] if args.mode in ("pre", "pre010") else ROUTES):
            if not (site_dir(build, route) / "service-worker.js").is_file():
                print(f"incomplete packaged build, missing {site_dir(build, route)}/service-worker.js",
                      flush=True)
                return 2
    if args.mode == "post":
        refusal = workspace_refusal(args.work_dir, [
            args.src_dir, args.legacy_a_dir, args.legacy_b_dir, args.legacy010_a_dir,
            args.legacy010_b_dir, ROOT / ".evidence"])
        if refusal:
            print(refusal, flush=True)
            return 2
    driver = Driver(args.port, sys.stdout, args.keep_artifacts)
    args.serve_log.parent.mkdir(parents=True, exist_ok=True)
    args.serve_log.write_text("")
    if args.mode == "pre":
        first = args.legacy_a_dir
    elif args.mode == "pre010":
        first = args.legacy010_a_dir
    else:
        first = args.src_dir
    server = Server(first, args.port, args.serve_log)
    workspace: Path | None = None
    if args.mode == "post":
        try:
            workspace = create_workspace(args.work_dir)
        except OSError as error:
            print(f"refusing output workspace: cannot create it: {error}", flush=True)
            return 2
    try:
        if args.mode == "pre":
            server.start()
            scenario_pre(driver, server, args.legacy_a_dir, args.legacy_b_dir, sys.stdout)
        elif args.mode == "pre010":
            server.start()
            scenario_pre010(driver, server, args.legacy010_a_dir, args.legacy010_b_dir)
        else:
            a, b = workspace / "a", workspace / "b"
            stamp_build(args.src_dir, a, "A", sys.stdout)
            stamp_build(args.src_dir, b, "B", sys.stdout)
            packaged_diff(a, b, sys.stdout, driver)
            for route in ROUTES:
                print(f"identity {route}: A={worker_version(a, route)} B={worker_version(b, route)}",
                      flush=True)
            for route in ROUTES:
                for label, build in (("src", args.src_dir), ("A", a), ("B", b)):
                    print(f"sha256 {label} {route} index.html: "
                          f"{sha256_file(site_dir(build, route) / 'index.html')}", flush=True)
            server.build_dir = a
            server.start()
            ctx = Ctx(driver, server, a, b, workspace,
                      args.legacy_b_dir if args.legacy_b_dir.is_dir() else None, sys.stdout)
            for leg in legs:
                try:
                    LEGS[leg](ctx)
                except Failure as error:
                    driver.fail(f"leg {leg} aborted: {error}")
                    end_session(driver)
    except Failure as error:
        driver.fail(f"aborted: {error}")
    finally:
        end_session(driver)
        server.stop()
        driver.cmd("verify dedicated port is free again")
        port_ok = port_free(args.port)
        driver.check("port free after run", port_ok)
        if workspace is not None:
            if (driver.session_closed and driver.owned_profile is None
                    and driver.session_dir is None and port_ok):
                remove_owned_tree(driver, workspace, "workspace")
            else:
                driver.fail("workspace cleanup: browser, profile or server not confirmed gone; "
                            "retain owned directory", str(workspace))
    failures = driver.failures
    print(f"=== {args.mode}: {len(failures)} failures ===", flush=True)
    for name in failures:
        print(f"FAILED: {name}", flush=True)
    for path in (driver.session_dir, workspace):
        if path is not None and path.exists() and path not in driver.kept:
            print(f"RETAINED (owned output, remove after inspection): {path}", flush=True)
    for path in driver.kept:
        print(f"DIAGNOSTIC ONLY (--keep-artifacts, not an acceptance run): {path}", flush=True)
    if failures:
        return 1
    if args.keep_artifacts:
        print(f"=== {args.mode}: NOT ACCEPTANCE (--keep-artifacts retains outputs; exit 3) ===",
              flush=True)
        return 3
    print(f"=== {args.mode}: ACCEPTANCE PASS (assertions held, outputs removed) ===", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
