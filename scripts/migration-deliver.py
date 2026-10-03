#!/usr/bin/env python3
"""Small local guard for commits and builds in the registered migration checkout.

This is an accidental-misuse guard for cooperating tools running as one OS user.
It does not protect file edits and can be bypassed by a deliberate user.
"""

from contextlib import contextmanager
import argparse
import fcntl
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import uuid


MARKER = "# migration-deliver-hook-v1"


class GuardError(RuntimeError):
    pass


def git_output(root, *args):
    return subprocess.check_output(["git", *args], cwd=root, text=True, stderr=subprocess.PIPE).strip()


def canonical_root(root):
    root = Path(root).resolve()
    return Path(git_output(root, "rev-parse", "--show-toplevel")).resolve()


def common_dir(root):
    raw = Path(git_output(root, "rev-parse", "--git-common-dir"))
    return (Path(root).resolve() / raw).resolve() if not raw.is_absolute() else raw.resolve()


def registry_path(root):
    return common_dir(root) / "migration-deliver-registry.json"


def state_lock(root):
    return common_dir(root) / "migration-deliver-state.lock"


def _locked(root):
    lock_path = state_lock(root)
    lock_path.parent.mkdir(parents=True, exist_ok=True)
    stream = lock_path.open("a+")
    fcntl.flock(stream.fileno(), fcntl.LOCK_EX)
    return stream


def _read_registry(root):
    path = registry_path(root)
    if not path.exists():
        return {"version": 1, "checkouts": {}}
    try:
        value = json.loads(path.read_text())
    except (OSError, json.JSONDecodeError) as error:
        raise GuardError(f"cannot read local migration registry {path}: {error}") from error
    if value.get("version") != 1 or not isinstance(value.get("checkouts"), dict):
        raise GuardError(f"unsupported or malformed migration registry: {path}")
    return value


def _write_registry(root, value):
    path = registry_path(root)
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, temporary = tempfile.mkstemp(prefix="migration-deliver-", dir=path.parent)
    try:
        with os.fdopen(fd, "w") as stream:
            json.dump(value, stream, indent=2)
            stream.write("\n")
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def _current_branch(root):
    return git_output(root, "branch", "--show-current")


def _entry(root, registry=None):
    root = canonical_root(root)
    registry = registry if registry is not None else _read_registry(root)
    entry = registry["checkouts"].get(str(root))
    if entry is None:
        return root, None
    branch = _current_branch(root)
    if branch != entry.get("branch"):
        raise GuardError(
            f"registered checkout {root} is on branch {branch!r}; expected {entry.get('branch')!r}"
        )
    if entry.get("role") not in ("author", "coordinator"):
        raise GuardError(f"invalid registered role for {root}")
    return root, entry


def validate_checkout(root, *, required=False):
    root, entry = _entry(root)
    if entry is None and required:
        raise GuardError(f"checkout is not registered for migration delivery: {root}")
    return root, entry


def _freeze_path(root):
    return common_dir(root) / "migration-build-freezes" / (str(uuid.uuid5(uuid.NAMESPACE_URL, str(canonical_root(root)))) + ".json")


def _freeze_lock_path(root):
    return _freeze_path(root).with_suffix(".lock")


def _freeze_active(root):
    path = _freeze_path(root)
    if not path.exists():
        return False
    # The context manager holds this advisory kernel lock for the whole build.
    # A free lock proves a leftover marker is stale, including after SIGKILL;
    # this avoids unsafe PID-liveness guesses and PID-reuse ambiguity.
    lock = _freeze_lock_path(root).open("a+")
    try:
        try:
            # Shared admission permits an active delivery commit to run its
            # pre-commit hook while retaining the lock until `git commit` exits.
            fcntl.flock(lock.fileno(), fcntl.LOCK_SH | fcntl.LOCK_NB)
        except BlockingIOError:
            return True
        path.unlink(missing_ok=True)
        return False
    finally:
        try:
            fcntl.flock(lock.fileno(), fcntl.LOCK_UN)
        finally:
            lock.close()


def _freeze_active_synchronized(root):
    lock = _locked(root)
    try:
        return _freeze_active(root)
    finally:
        fcntl.flock(lock.fileno(), fcntl.LOCK_UN)
        lock.close()


@contextmanager
def build_freeze(root):
    """Block coordinator integration commits for this registered checkout."""
    root, entry = validate_checkout(root)
    if entry is None:
        yield
        return
    if entry["role"] != "coordinator":
        raise GuardError("only a registered coordinator checkout may start a migration build freeze")
    path = _freeze_path(root)
    path.parent.mkdir(parents=True, exist_ok=True)
    lock = _locked(root)
    freeze_lock = _freeze_lock_path(root).open("a+")
    token = uuid.uuid4().hex
    lease_acquired = False
    try:
        try:
            fcntl.flock(freeze_lock.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError as error:
            raise GuardError(f"a build freeze is already active for {root}") from error
        lease_acquired = True
        # Any prior marker is stale because this process owns the kernel lock.
        path.unlink(missing_ok=True)
        marker = {"root": str(root), "pid": os.getpid(), "started_unix": time.time(), "token": token}
        fd, temporary = tempfile.mkstemp(prefix="freeze-", dir=path.parent)
        try:
            with os.fdopen(fd, "w") as stream:
                json.dump(marker, stream)
                stream.write("\n")
                stream.flush()
                os.fsync(stream.fileno())
            os.replace(temporary, path)
        finally:
            if os.path.exists(temporary):
                os.unlink(temporary)
    except BaseException:
        if lease_acquired:
            fcntl.flock(freeze_lock.fileno(), fcntl.LOCK_UN)
        freeze_lock.close()
        raise
    finally:
        fcntl.flock(lock.fileno(), fcntl.LOCK_UN)
        lock.close()
    try:
        yield
    finally:
        lock = _locked(root)
        try:
            if path.exists():
                try:
                    marker = json.loads(path.read_text())
                except (OSError, json.JSONDecodeError):
                    marker = {}
                if marker.get("token") == token and marker.get("root") == str(root):
                    path.unlink()
            fcntl.flock(freeze_lock.fileno(), fcntl.LOCK_UN)
        finally:
            fcntl.flock(lock.fileno(), fcntl.LOCK_UN)
            lock.close()
            freeze_lock.close()


def _check_commit(root, intent=None):
    root, entry = validate_checkout(root)
    if entry is None:
        return
    if entry["role"] == "coordinator":
        if intent != "integration":
            raise GuardError("coordinator commits must use migration-deliver.py commit --intent integration")
        if _freeze_active_synchronized(root):
            raise GuardError("coordinator integration commit blocked while this checkout is build-frozen")
    elif intent == "integration":
        raise GuardError("author role cannot create an integration commit")
    elif intent not in (None, "delivery"):
        raise GuardError("author commits must use delivery intent")


def _git_hook(root):
    try:
        _check_commit(root, os.environ.get("MIGRATION_DELIVERY_INTENT"))
    except (GuardError, subprocess.CalledProcessError) as error:
        print(f"migration-deliver: {error}", file=sys.stderr)
        return 1
    return 0


def _install(root):
    root, entry = validate_checkout(root, required=True)
    if entry["role"] != "coordinator":
        raise GuardError("only the registered coordinator checkout may install the migration hook")
    configured_hooks = subprocess.run(
        ["git", "config", "--show-origin", "--get-all", "core.hooksPath"],
        cwd=root, text=True, capture_output=True,
    )
    if configured_hooks.returncode == 0 and configured_hooks.stdout.strip():
        raise GuardError(
            "core.hooksPath is configured; refusing to install into an unused default hooks directory: "
            + configured_hooks.stdout.strip().replace("\n", "; ")
        )
    if configured_hooks.returncode not in (0, 1):
        raise GuardError(f"could not inspect core.hooksPath: {configured_hooks.stderr.strip()}")
    hooks = common_dir(root) / "hooks"
    hooks.mkdir(parents=True, exist_ok=True)
    hook = hooks / "pre-commit"
    chain = hooks / "pre-commit.migration-deliver-chain"
    existing = hook.read_text() if hook.exists() else None
    if existing and MARKER not in existing:
        if chain.exists():
            raise GuardError(f"refusing to overwrite existing hook chain target {chain}")
        os.replace(hook, chain)
    script = Path(__file__).resolve()
    shim = "\n".join((
        "#!/bin/sh",
        MARKER,
        f"CHAIN={_shell_quote(str(chain))}",
        f"{_shell_quote(sys.executable)} {_shell_quote(str(script))} hook || exit $?",
        'if [ -x "$CHAIN" ]; then exec "$CHAIN" "$@"; fi',
        "exit 0",
        "",
    ))
    fd, temporary = tempfile.mkstemp(prefix="pre-commit-", dir=hooks, text=True)
    try:
        with os.fdopen(fd, "w") as stream:
            stream.write(shim)
        os.chmod(temporary, 0o755)
        os.replace(temporary, hook)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)
    return hook


def _shell_quote(value):
    return "'" + value.replace("'", "'\\''") + "'"


def _commit(root, intent, message, paths):
    root, entry = validate_checkout(root, required=True)
    if entry["role"] == "coordinator" and intent != "integration":
        raise GuardError("coordinator commits require --intent integration")
    if entry["role"] == "author" and intent != "delivery":
        raise GuardError("author role can only commit with --intent delivery")
    # Reap a marker whose kernel lease ended after an interrupted build before
    # this commit takes its own shared lease. The check is repeated below after
    # locking, so a build that wins the gap still blocks the commit.
    _check_commit(root, intent)
    lock_path = _freeze_lock_path(root)
    lock_path.parent.mkdir(parents=True, exist_ok=True)
    commit_lock = lock_path.open("a+")
    try:
        try:
            fcntl.flock(commit_lock.fileno(), fcntl.LOCK_SH | fcntl.LOCK_NB)
        except BlockingIOError as error:
            raise GuardError("guarded commit blocked while this checkout is build-frozen or committing") from error
        _check_commit(root, intent)
        environment = os.environ.copy()
        environment["MIGRATION_DELIVERY_INTENT"] = intent
        args = ["git", "commit", "-m", message]
        if paths:
            args.extend(["--", *paths])
        subprocess.run(args, cwd=root, env=environment, check=True)
    finally:
        try:
            fcntl.flock(commit_lock.fileno(), fcntl.LOCK_UN)
        finally:
            commit_lock.close()


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    register = commands.add_parser("register", help="register this checkout path, branch, and role locally")
    register.add_argument("--role", choices=("author", "coordinator"), required=True)
    register.add_argument("--branch", help="expected branch (defaults to current branch)")
    register.add_argument("--replace-registration", action="store_true", help="explicitly replace this path's existing role/branch")
    commands.add_parser("install", help="install the repo-local pre-commit guard, preserving an existing hook")
    commands.add_parser("check", help="validate this checkout's registration and active freeze state")
    commit = commands.add_parser("commit", help="make a guarded migration delivery/integration commit")
    commit.add_argument("--intent", choices=("delivery", "integration"), required=True)
    commit.add_argument("-m", "--message", required=True)
    commit.add_argument("paths", nargs="*", help="optional pathspecs; otherwise commit the current index")
    commands.add_parser("hook", help="internal entry point used by the installed pre-commit hook")
    args = parser.parse_args(argv)
    try:
        root = canonical_root(Path.cwd())
        if args.command == "hook":
            return _git_hook(root)
        if args.command == "register":
            branch = args.branch or _current_branch(root)
            if not branch:
                raise GuardError("cannot register a detached checkout; check out its intended branch first")
            current_branch = _current_branch(root)
            if branch != current_branch:
                raise GuardError(f"--branch {branch!r} does not match current branch {current_branch!r}")
            lock = _locked(root)
            try:
                registry = _read_registry(root)
                existing = registry["checkouts"].get(str(root))
                requested = {"branch": branch, "role": args.role}
                if existing and existing != requested and not args.replace_registration:
                    raise GuardError(
                        f"checkout already registered as {existing}; pass --replace-registration to change it"
                    )
                if existing != requested:
                    registry["checkouts"][str(root)] = requested
                    _write_registry(root, registry)
            finally:
                fcntl.flock(lock.fileno(), fcntl.LOCK_UN)
                lock.close()
            print(f"registered {root} on {branch} as {args.role}")
        elif args.command == "install":
            print(f"installed repo-local hook at {_install(root)}")
        elif args.command == "check":
            _, entry = validate_checkout(root, required=True)
            print(f"registered {root} on {entry['branch']} as {entry['role']}; build_frozen={_freeze_active(root)}")
        elif args.command == "commit":
            _commit(root, args.intent, args.message, args.paths)
        return 0
    except (GuardError, subprocess.CalledProcessError, OSError) as error:
        print(f"migration-deliver: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
