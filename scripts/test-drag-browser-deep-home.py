#!/usr/bin/env python3
"""Run the browser consumer controls with an isolated, deliberately deep HOME."""

import ctypes
import os
from pathlib import Path
import shutil
import signal
import subprocess
import tempfile
import time


class ProcessGroupCleanupError(RuntimeError):
    pass


PR_SET_CHILD_SUBREAPER = 36
PR_GET_CHILD_SUBREAPER = 37


def live_group_members(group):
    members = []
    for path in Path('/proc').iterdir():
        if not path.name.isdecimal():
            continue
        try:
            stat = (path / 'stat').read_text()
            fields = stat[stat.rfind(')') + 2:].split()
            if fields[0] != 'Z' and int(fields[2]) == group:
                members.append(int(path.name))
        except (FileNotFoundError, ProcessLookupError, PermissionError):
            continue
    return members


def child_pids():
    children = set()
    for task in Path('/proc/self/task').iterdir():
        try:
            children.update(int(pid) for pid in (task / 'children').read_text().split())
        except FileNotFoundError:
            continue
    return children


def reap_owned_children(baseline, group):
    deadline = time.monotonic() + 5
    while True:
        adopted = child_pids() - baseline - {group}
        for pid in adopted:
            try:
                fd = os.pidfd_open(pid)
            except ProcessLookupError:
                continue
            try:
                stat = Path(f'/proc/{pid}/stat').read_text()
                fields = stat[stat.rfind(')') + 2:].split()
                if int(fields[1]) == os.getpid():
                    if fields[0] != 'Z':
                        signal.pidfd_send_signal(fd, signal.SIGKILL)
                    try:
                        os.waitpid(pid, os.WNOHANG)
                    except ChildProcessError:
                        pass
            except (FileNotFoundError, ProcessLookupError):
                pass
            finally:
                os.close(fd)
        members = live_group_members(group)
        adopted = child_pids() - baseline - {group}
        if not members and not adopted:
            return
        if time.monotonic() >= deadline:
            raise ProcessGroupCleanupError(f"control still has live or unreaped processes: {members}, {sorted(adopted)}")
        time.sleep(0.05)


def run_control(command, *, cwd, env, timeout, pass_fds=()):
    # Chromium's crashpad can leave the process group; adopt its orphaned children.
    libc = ctypes.CDLL(None, use_errno=True)
    previous = ctypes.c_int()
    if (libc.prctl(PR_GET_CHILD_SUBREAPER, ctypes.byref(previous), 0, 0, 0)
            or libc.prctl(PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0)):
        raise OSError(ctypes.get_errno(), "cannot enable child subreaper")
    try:
        # This single-control wrapper starts no other children while it waits.
        baseline = child_pids()
        process = subprocess.Popen(command, cwd=cwd, env=env, pass_fds=pass_fds,
                                   start_new_session=True)
        try:
            try:
                code = process.wait(timeout=timeout)
            except BaseException:
                try:
                    os.killpg(process.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                process.wait(timeout=5)
                raise
        finally:
            try:
                reap_owned_children(baseline, process.pid)
            except BaseException as error:
                raise ProcessGroupCleanupError(f"could not verify control cleanup: {error}") from error
        return subprocess.CompletedProcess(command, code)
    finally:
        if libc.prctl(PR_SET_CHILD_SUBREAPER, previous.value, 0, 0, 0):
            raise ProcessGroupCleanupError("cannot restore child subreaper state")


def main():
    root = Path(__file__).resolve().parents[1]
    real_home = Path.home()
    lock = real_home / ".cache/tmp/boardstudio-build.lock"
    if not lock.is_file():
        raise SystemExit(f"deep HOME control needs the shared build lock: {lock}")

    directory = tempfile.mkdtemp(prefix="bs-home-", dir="/tmp")
    cleanup = True
    try:
        home = Path(directory) / ("deep-home-" + "x" * 110)
        lock_link = home / ".cache/tmp/boardstudio-build.lock"
        lock_link.parent.mkdir(parents=True)
        lock_link.symlink_to(lock)
        sentinel = home / "sentinel"
        sentinel.write_text("keep")
        env = os.environ.copy()
        env.update(HOME=str(home), CARGO_HOME=env.get("CARGO_HOME", str(real_home / ".cargo")),
                   RUSTUP_HOME=env.get("RUSTUP_HOME", str(real_home / ".rustup")))
        inherited = env.get("BOARDSTUDIO_BUILD_LOCK_FD")
        result = run_control(["python3", "-u", "scripts/test-drag-browser-consumer-controls.py"],
                             cwd=root, env=env, timeout=150,
                             pass_fds=(int(inherited),) if inherited is not None else ())
        if sentinel.read_text() != "keep":
            raise SystemExit("deep HOME control modified its sentinel")
        if result.returncode:
            raise SystemExit(f"deep HOME consumer controls failed: exit {result.returncode}")
        print("PASS deep HOME consumer controls; sentinel intact")
    except ProcessGroupCleanupError:
        cleanup = False
        print(f"preserving deep HOME after incomplete process cleanup: {directory}", flush=True)
        raise
    finally:
        if cleanup:
            shutil.rmtree(directory)


if __name__ == "__main__":
    main()
