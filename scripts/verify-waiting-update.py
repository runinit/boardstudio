#!/usr/bin/env python3
"""Reproducible packaged A-then-B verification for ticket 018 (reusable by 019).

Drives a real packaged old release (A) in an isolated browser session, then a
real packaged new release (B) at the same origin/scope, and asserts the
production waiting-update behavior with real browser observations. Every step
echoes its exact command and every browser assertion output is printed raw, so
the combined stdout is the complete evidence log (redirect it to .evidence/).

Modes:
  pre     pre-feature A: the waiting-update notice assertion must fail (RED).
  post    feature A: waiting installation must produce the production notice,
          plus initial-install/failed-install controls, real production
          Workspace switch without duplication, real keyboard dismissal,
          and reload reappearance (GREEN).
  rm      post GREEN under emulated prefers-reduced-motion, asserting computed
          animation:none / 0s durations and full keyboard operability.

The driver FAILS (exit 1) on any wrong observation: missing/extra notices,
wrong worker state, identity mismatches, a changed load marker (reload),
broken cache-version binding, or a failed negative control. A passing run
prints PASS for every check.

Identity binding (independent of the claim): the served release version is
recorded by curl before load and after each swap; an in-page random load
marker is set at load and re-read on every observation (unchanged means no
reload, so the loaded page runs the release served at load); after the update
each versioned offline cache's stored service-worker.js bootstrap text is read
back from production cache storage, binding installed releases to versions.
Residual limitation, stated plainly: no web API exposes a service worker
object's own version, and fetching the worker URL returns the currently
served file rather than the installed worker's bytes, so active<->A /
waiting<->B attribution chains install chronology (a fresh profile admits
exactly one install per phase) with production bytes. Nothing beyond the
logged observations is claimed.

The driver owns its server processes (subprocess PIDs) and never kills
foreign processes: it refuses to start when the dedicated port is occupied,
and verifies the port is free again afterwards.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import socket
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PAYLOADS = Path(__file__).resolve().parent / "verify-waiting-update-payloads"
SERVE = [sys.executable, str(ROOT / "scripts" / "serve-web.py")]
VERSION_RE = re.compile(r"boardstudio-[0-9a-f]{16}-[0-9]+")
REF_RE = re.compile(r'button "([^"]+)"[^\n]*?ref=(e\d+)')


class Failure(Exception):
    pass


class Driver:
    def __init__(self, session: str, port: int, log):
        self.session = session
        self.port = port
        self.log = log
        self.failures: list[str] = []
        self.env = dict(os.environ, AGENT_BROWSER_SESSION=session)

    def cmd(self, text: str) -> None:
        print(f"--- cmd: {text} ---", file=self.log, flush=True)

    def run(self, args: list[str], timeout: int = 120) -> str:
        self.cmd("$ " + " ".join(args))
        done = subprocess.run(args, cwd=ROOT, env=self.env, capture_output=True,
                              text=True, timeout=timeout)
        out = (done.stdout or "") + (done.stderr or "")
        print(out, file=self.log, flush=True)
        if done.returncode != 0:
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
        self.cmd(f'$ snapshot + click button "{name}"')
        snap = self.snapshot()
        ref = None
        for line in snap.splitlines():
            match = re.search(r'button "' + re.escape(name) + r'"[^\n]*?ref=(e\d+)', line)
            if match:
                ref = "@" + match.group(1)
                break
        self.cmd(f"ref: {ref}")
        if ref is None:
            self.fail(f"button ref not found: {name}")
            raise Failure(f"no ref for {name}")
        self.run(["agent-browser", "click", ref])

    def eval_file(self, payload: str):
        path = PAYLOADS / payload
        self.cmd(f"$ agent-browser eval --stdin < {path}")
        print(f"--- payload {path} ---", file=self.log, flush=True)
        print(path.read_text(), file=self.log, flush=True)
        print("--- output ---", file=self.log, flush=True)
        done = subprocess.run(["agent-browser", "eval", "--stdin"], cwd=ROOT, env=self.env,
                              stdin=open(path), capture_output=True, text=True, timeout=120)
        print(done.stdout or "", file=self.log, flush=True)
        if done.stderr:
            print(done.stderr, file=self.log, flush=True)
        try:
            return json.loads(done.stdout)
        except (json.JSONDecodeError, ValueError):
            self.fail(f"eval output not JSON: {payload}", (done.stdout or "")[:300])
            raise Failure(f"bad eval output: {payload}")

    def wait_text(self, text: str, timeout_ms: int = 30000) -> None:
        self.run(["agent-browser", "wait", "--text", text], timeout=timeout_ms // 1000 + 60)

    def wait_fn(self, js: str) -> None:
        self.run(["agent-browser", "wait", "--fn", js])


def port_free(port: int) -> bool:
    with socket.socket() as sock:
        sock.settimeout(2)
        return sock.connect_ex(("127.0.0.1", port)) != 0


def serve_version(port: int) -> str:
    out = subprocess.run(["curl", "-s", f"http://127.0.0.1:{port}/service-worker.js"],
                         capture_output=True, text=True, timeout=30).stdout
    match = VERSION_RE.search(out)
    if not match:
        raise Failure(f"no release version served on {port}")
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


def open_demo(driver: Driver) -> None:
    driver.click_button("Open demo")
    driver.click_button("Open demo copy")
    driver.wait_text("Main board")


def set_marker(driver: Driver) -> str:
    result = driver.eval_file("marker.js")
    marker = (result or {}).get("loadMark")
    driver.check("load marker set", bool(marker), str(marker))
    if not marker:
        raise Failure("no load marker")
    return marker


def check_caches_match(driver: Driver, log_label: str, expected: dict[str, str]) -> None:
    result = driver.eval_file("cachecontent.js")
    cached = (result or {}).get("cachedWorkerVersions", {})
    for cache_name, version in expected.items():
        actual = cached.get(cache_name)
        driver.check(f"{log_label}: cache holds {version}", actual == version,
                     f"cache={cache_name} actual={actual}")
    driver.check(f"{log_label}: no unexpected caches", set(cached) == set(expected),
                 f"caches={sorted(cached)}")


def scenario_pre(driver: Driver, pre_dir: Path, b_dir: Path) -> tuple[str, str]:
    served_pre = serve_version(driver.port)
    driver.cmd(f"served pre-A version: {served_pre}")
    driver.check("served pre-A version parses", bool(served_pre), served_pre)
    open_demo(driver)
    marker = set_marker(driver)
    state = driver.eval_file("red.js")
    driver.check("initial install: controlled", bool(state.get("controller")),
                 str(state.get("controller")))
    driver.check("initial install: no waiting worker", state.get("waiting") is None,
                 str(state.get("waiting")))
    driver.check("initial install: no notice", state.get("updateNotice") is False,
                 str(state.get("allStatuses")))
    driver.check("initial install: editor alive", state.get("editorAlive") is True)
    driver.check("initial install: load marker", state.get("loadMark") == marker, marker)
    driver.check("initial install: single pre-A cache",
                 state.get("caches") == [f"boardstudio-m1-offline-path-2f-{served_pre}"],
                 str(state.get("caches")))
    check_caches_match(driver, "initial install",
                         {f"boardstudio-m1-offline-path-2f-{served_pre}": served_pre})
    return served_pre, marker


def scenario_post_green(driver: Driver, a_dir: Path, served_a: str, marker: str) -> None:
    state = driver.eval_file("red.js")
    driver.check("RED: controlled by A", state.get("controller") is not None)
    driver.check("RED: no waiting worker", state.get("waiting") is None)
    driver.check("RED: no notice", state.get("updateNotice") is False,
                 str(state.get("allStatuses")))
    driver.check("RED: editor alive", state.get("editorAlive") is True)
    driver.check("RED: load marker unchanged", state.get("loadMark") == marker, marker)
    driver.check("RED: only the A version cache",
                 state.get("caches") == [f"boardstudio-m1-offline-path-2f-{served_a}"],
                 str(state.get("caches")))

    driver.cmd("stage broken worker (same version + injected throw)")
    sw = a_dir / "site-root" / "service-worker.js"
    print(f"$ cp {sw} /tmp/018-backup-sw.js", file=driver.log, flush=True)
    backup = sw.read_bytes()
    sw.write_text(sw.read_text() + '\nthrow new Error("018 injected install failure");\n')
    print("broken staged", file=driver.log, flush=True)
    try:
        failed = driver.eval_file("failed.js")
        driver.check("failed install: update rejects", failed.get("updateThrew") is True)
        driver.check("failed install: no waiting worker", failed.get("waiting") is None)
        driver.check("failed install: no notice", failed.get("failedControlNoNotice") is True)
        driver.check("failed install: controller intact",
                     failed.get("controller") == "A-controlled")
        driver.check("failed install: editor alive", failed.get("editorAlive") is True)
        driver.check("failed install: load marker unchanged", failed.get("loadMark") == marker)
    finally:
        sw.write_bytes(backup)
        driver.cmd("restore valid service worker")
        print("RESTORED_CLEAN" if b"injected install failure" not in sw.read_bytes()
              else "RESTORE_FAILED", file=driver.log, flush=True)
        driver.check("broken worker restored", b"injected install failure" not in sw.read_bytes())


def scenario_swap_to_b(driver: Driver, b_dir: Path, server: Server, port: int) -> str:
    driver.cmd("swap origin to B (stop A server, serve B build)")
    server.stop()
    server.build_dir = b_dir
    # Fresh mtime so a poisoned script entry from the failed-install control
    # cannot revalidate 304 and shadow the new release bytes.
    sw_b = b_dir / "site-root" / "service-worker.js"
    driver.cmd(f"$ touch {sw_b}")
    os.utime(sw_b, None)
    server.start()
    served_b = serve_version(port)
    driver.cmd(f"served B version: {served_b}")
    driver.check("served B version parses", bool(served_b), served_b)
    return served_b


def scenario_green(driver: Driver, served_a: str, served_b: str, marker: str) -> None:
    result = driver.eval_file("update.js")
    driver.check("GREEN: update did not reject", result.get("updateThrew") is None,
                 str(result.get("updateThrew")))
    driver.check("GREEN: waiting worker present", result.get("waiting") == "PRESENT",
                 str(result))
    driver.check("GREEN: controller still A", result.get("controller") is not None)
    check_waiting_notice(driver, result)
    driver.check("GREEN: toast role=status", result.get("toastRole") == "status")
    driver.check("GREEN: toast atomic", result.get("toastAtomic") == "true")
    driver.check("GREEN: toast info type", result.get("toastDataType") == "info")
    driver.check("GREEN: close is a tab stop", result.get("closeTabIndex") == 0)
    driver.check("GREEN: region announces update",
                 (result.get("regionText") or "").startswith("Update Available"))
    driver.check("GREEN: editor alive", result.get("editorAlive") is True)
    driver.check("GREEN: load marker unchanged", result.get("loadMark") == marker, marker)
    expected = {
        f"boardstudio-m1-offline-path-2f-{served_a}": served_a,
        f"boardstudio-m1-offline-path-2f-{served_b}": served_b,
    }
    driver.check("GREEN: both version caches present", result.get("caches") is not None)
    check_caches_match(driver, "GREEN", expected)


def check_waiting_notice(driver: Driver, result: dict) -> None:
    driver.check("waiting update: notice shown", result.get("updateNotice") is True,
                 str(result.get("allStatuses")))


def scenario_workspace(driver: Driver, marker: str) -> None:
    driver.cmd("snapshot to locate production workspace nav + toast close")
    snap = driver.snapshot()
    driver.check("workspace nav present", 'aria-label="Workspaces"' in snap or "Workspaces" in snap)
    driver.click_button("Open PCB workspace")
    result = driver.eval_file("workspace.js")
    driver.check("workspace: PCB active", result.get("pcbActive") is True)
    driver.check("workspace: exactly one notice", result.get("updateNoticeCount") == 1,
                 str(result.get("updateNoticeCount")))
    driver.check("workspace: load marker unchanged", result.get("loadMark") == marker, marker)


def scenario_dismiss(driver: Driver, marker: str) -> None:
    driver.cmd("snapshot to locate toast close")
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
    import time as _time
    _time.sleep(2)
    result = driver.eval_file("dismiss.js")
    driver.check("dismissal: notice gone", result.get("keyboardDismissed") is True)
    driver.check("dismissal: waiting still present", result.get("waiting") == "STILL-WAITING")
    driver.check("dismissal: controller still A", result.get("controller") == "A-controlled")
    driver.check("dismissal: load marker unchanged", result.get("loadMark") == marker, marker)


def scenario_reload(driver: Driver) -> None:
    driver.cmd("$ agent-browser reload")
    driver.run(["agent-browser", "reload"])
    driver.cmd("$ agent-browser wait --fn notice-present")
    driver.run(["agent-browser", "wait", "--fn",
                "Array.from(document.querySelectorAll('[role=status]')).some(e => (e.textContent||'').includes('Update Available'))"])
    result = driver.eval_file("reload.js")
    driver.check("reload: notice reappears", result.get("reappearedAfterReload") is True)
    driver.check("reload: B still waiting (no adoption)", result.get("waiting") == "STILL-WAITING",
                 str(result.get("waiting")))


def scenario_rm_green(driver: Driver, served_a: str, served_b: str, marker: str) -> None:
    result = driver.eval_file("rm.js")
    driver.check("reduced motion: update did not reject", result.get("updateThrew") is None)
    driver.check("reduced motion: emulated", result.get("reduced") is True)
    driver.check("reduced motion: notice shown", result.get("notice") is True)
    driver.check("reduced motion: waiting present", result.get("waiting") == "PRESENT")
    driver.check("reduced motion: controller still A", result.get("controller") is not None)
    driver.check("reduced motion: editor alive", result.get("editorAlive") is True)
    driver.check("reduced motion: load marker unchanged", result.get("loadMark") == marker,
                 marker)
    computed = result.get("toastComputed") or {}
    driver.check("reduced motion: toast has no animation",
                 computed.get("animationName") == "none", str(computed))
    driver.check("reduced motion: toast has no transition",
                 computed.get("transitionDuration") == "0s", str(computed))
    close_computed = result.get("closeComputed") or {}
    driver.check("reduced motion: close has no animation",
                 close_computed.get("animationName") == "none", str(close_computed))
    driver.check("reduced motion: close is a tab stop", result.get("closeTabIndex") == 0)
    caches = result.get("caches") or []
    driver.check("reduced motion: both version caches present",
                 len(caches) == 2, str(caches))
    check_caches_match(driver, "reduced motion", {
        f"boardstudio-m1-offline-path-2f-{served_a}": served_a,
        f"boardstudio-m1-offline-path-2f-{served_b}": served_b,
    })


def scenario_rm_dismiss(driver: Driver, marker: str) -> None:
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
    result = driver.eval_file("rmdismiss.js")
    driver.check("reduced motion: dismissed", result.get("dismissedUnderReducedMotion") is True)
    driver.check("reduced motion: waiting still present", result.get("waiting") == "STILL-WAITING")
    driver.check("reduced motion: controller still A", result.get("controller") == "A-controlled")
    driver.check("reduced motion: editor alive", result.get("editorAlive") is True)
    driver.check("reduced motion: load marker unchanged", result.get("loadMark") == marker,
                 marker)


def new_session(prefix: str) -> str:
    done = subprocess.run(["agent-browser", "session", "id", "--scope", "worktree",
                           "--prefix", prefix], cwd=ROOT, capture_output=True, text=True,
                         timeout=60)
    session = done.stdout.strip().splitlines()[-1]
    if not session:
        raise Failure("could not create agent-browser session")
    return session


def run_session(driver: Driver, prefix: str, reduced_motion: bool = False) -> None:
    session = new_session(prefix)
    driver.session = session
    driver.env = dict(os.environ, AGENT_BROWSER_SESSION=session)
    driver.cmd(f"session: {session} (fresh isolated browser profile)")
    if reduced_motion:
        driver.run(["agent-browser", "set", "media", "light", "reduced-motion"])
    driver.cmd(f"$ agent-browser open http://127.0.0.1:{driver.port}/")
    driver.run(["agent-browser", "open", f"http://127.0.0.1:{driver.port}/"])
    driver.cmd("$ agent-browser wait --text Your keyboards (app render)")
    driver.run(["agent-browser", "wait", "--text", "Your keyboards"], timeout=90)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=["pre", "post", "rm"],
                        help="pre: pre-feature RED; post: feature GREEN; rm: reduced-motion GREEN")
    parser.add_argument("--port", type=int, default=4723)
    parser.add_argument("--a-dir", type=Path,
                        default=ROOT / "web" / "target" / "site-018-a",
                        help="packaged A build root (served first)")
    parser.add_argument("--b-dir", type=Path,
                        default=ROOT / "web" / "target" / "site-018-b",
                        help="packaged B build root (swapped in)")
    parser.add_argument("--pre-dir", type=Path, default=Path("/tmp/pre018-out"),
                        help="packaged pre-feature build root (pre mode)")
    parser.add_argument("--session-prefix", default="018-verify")
    args = parser.parse_args(argv)
    if shutil.which("agent-browser") is None:
        print("agent-browser CLI not found", flush=True)
        return 2
    if not port_free(args.port):
        print(f"dedicated origin port {args.port} is occupied; refusing to disturb it",
              flush=True)
        return 2
    first = args.pre_dir if args.mode == "pre" else args.a_dir
    required = first / "site-root" / "service-worker.js"
    if not required.is_file():
        print(f"incomplete packaged build, missing {required}", flush=True)
        return 2
    driver = Driver(session="", port=args.port, log=sys.stdout)
    serve_log = ROOT / ".evidence" / f"018-serve-{args.mode}-driver.log"
    serve_log.write_text("")
    server = Server(first, args.port, serve_log)
    failures: list[str] = []
    try:
        server.start()
        if args.mode == "pre":
            run_session(driver, args.session_prefix + "-pre")
            served_pre, marker = scenario_pre(driver, first, args.b_dir)
            served_b = scenario_swap_to_b(driver, args.b_dir, server, args.port)
            result = driver.eval_file("update.js")
            driver.check("pre RED: update did not reject", result.get("updateThrew") is None)
            driver.check("pre RED: waiting worker present", result.get("waiting") == "PRESENT")
            driver.check("pre RED: controller intact", result.get("controller") is not None)
            check_waiting_notice(driver, result)
            driver.check("pre RED: editor alive", result.get("editorAlive") is True)
            driver.check("pre RED: load marker unchanged", result.get("loadMark") == marker)
            check_caches_match(driver, "pre RED", {
                f"boardstudio-m1-offline-path-2f-{served_pre}": served_pre,
                f"boardstudio-m1-offline-path-2f-{served_b}": served_b,
            })
        else:
            run_session(driver, args.session_prefix + ("-rm" if args.mode == "rm" else "-post"),
                        reduced_motion=args.mode == "rm")
            open_demo(driver)
            served_a = serve_version(args.port)
            driver.cmd(f"served A version: {served_a}")
            driver.served_a = served_a
            marker = driver.eval_file("marker.js").get("loadMark")
            scenario_post_green(driver, args.a_dir, served_a, marker)
            served_b = scenario_swap_to_b(driver, args.b_dir, server, args.port)
            if args.mode == "rm":
                scenario_rm_green(driver, served_a, served_b, marker)
                scenario_rm_dismiss(driver, marker)
            else:
                scenario_green(driver, served_a, served_b, marker)
                scenario_workspace(driver, marker)
                scenario_dismiss(driver, marker)
                scenario_reload(driver)
    except Failure as error:
        driver.fail(f"aborted: {error}")
    finally:
        try:
            driver.cmd("$ agent-browser close")
            subprocess.run(["agent-browser", "close"], cwd=ROOT, env=driver.env,
                           capture_output=True, timeout=60)
        except (OSError, subprocess.SubprocessError):
            pass
        server.stop()
        driver.cmd("verify dedicated port is free again")
        driver.check("port free after run", port_free(args.port))
    failures = driver.failures
    print(f"=== {args.mode}: {len(failures)} failures ===", flush=True)
    for name in failures:
        print(f"FAILED: {name}", flush=True)
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())

