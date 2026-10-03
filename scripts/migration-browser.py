#!/usr/bin/env python3
"""Small owned-session wrapper for migration browser qualification."""

import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[1]
RUN = ROOT / "docs/migration/dioxus-frontend-v1-run.json"
PROGRESS_PATH = ROOT / ".scratch/dioxus-frontend-v1/progress.py"
MAX_OUTPUT = 12_000
TRUNCATION_MARKER = "\n...[output truncated]"


class BrowserError(RuntimeError):
    pass


def bounded(value, limit):
    if len(value) <= limit:
        return value
    marker = TRUNCATION_MARKER[:limit]
    return value[:max(0, limit - len(marker))] + marker


def _progress():
    spec = importlib.util.spec_from_file_location("migration_progress", PROGRESS_PATH)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def run_browser(args, *, timeout=None, env=None):
    """Run the installed CLI without a shell and keep its response bounded."""
    command = ["agent-browser", *args]
    try:
        result = subprocess.run(command, cwd=ROOT, text=True, capture_output=True,
                                timeout=timeout, check=False, env=env)
    except (OSError, subprocess.TimeoutExpired) as error:
        raise BrowserError(f"agent-browser could not complete: {error}") from error
    if result.returncode:
        detail = bounded((result.stderr or result.stdout).strip(), MAX_OUTPUT)
        raise BrowserError(f"agent-browser exited {result.returncode}: {detail}")
    output = result.stdout.strip()
    return bounded(output, MAX_OUTPUT)


def owned_session(build_id, purpose):
    if not re.fullmatch(r"[A-Za-z0-9._-]{1,80}", build_id or ""):
        raise BrowserError("build id must contain only letters, digits, dot, underscore or hyphen")
    if not re.fullmatch(r"[A-Za-z0-9._-]{1,64}", purpose or ""):
        raise BrowserError("purpose must contain only letters, digits, dot, underscore or hyphen")
    checkout = str(ROOT.resolve())
    digest = hashlib.sha256(f"{checkout}\0{build_id}\0{purpose}".encode()).hexdigest()[:12]
    return f"bs-mig-{digest}"


def candidate(build_id):
    try:
        progress = json.loads(RUN.read_text())["current_progress"]
        served = progress["served_candidate"]
    except (OSError, KeyError, TypeError, json.JSONDecodeError) as error:
        raise BrowserError(f"cannot read current served candidate: {error}") from error
    if served.get("build_id") != build_id:
        raise BrowserError(f"candidate {build_id!r} is not the currently served candidate")
    proof_rel = served.get("package_proof")
    if not isinstance(proof_rel, str):
        raise BrowserError("served candidate has no package proof")
    try:
        proof_path = _progress().repository_file(proof_rel, "package proof")
        module = _progress()
        proof, provenance = module.validate_provenance(proof_path)
        if (proof["build_id"] != build_id
                or proof["source_commit"] != served.get("source_commit")
                or proof["provenance_sha256"] != served.get("provenance_sha256")):
            raise ValueError("current run record does not match package proof identity")
        module.verify_candidate_routes(
            proof, provenance, served["root_url"], served["subpath_url"])
    except (ValueError, OSError, KeyError, TypeError, json.JSONDecodeError) as error:
        raise BrowserError(f"candidate proof verification failed: {error}") from error
    return served


def session_args(build_id, purpose):
    return ["--session", owned_session(build_id, purpose)]


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)

    candidate_cmd = commands.add_parser("candidate", help="verify or open the current candidate")
    candidate_actions = candidate_cmd.add_subparsers(dest="action", required=True)
    verify = candidate_actions.add_parser("verify")
    verify.add_argument("--build-id", required=True)
    open_cmd = candidate_actions.add_parser("open")
    open_cmd.add_argument("--build-id", required=True)
    open_cmd.add_argument("--purpose", required=True)
    open_cmd.add_argument("--route", choices=("root", "subpath"), default="root")

    for name in ("upload", "wait", "inspect", "close"):
        sub = commands.add_parser(name)
        sub.add_argument("--build-id", required=True)
        sub.add_argument("--purpose", required=True)
        if name == "upload":
            sub.add_argument("--selector", required=True)
            sub.add_argument("file")
        elif name == "wait":
            sub.add_argument("--fn", required=True, help="caller-provided JavaScript predicate")
            sub.add_argument("--timeout-ms", type=int, default=25_000)
        elif name == "inspect":
            sub.add_argument("--selector", required=True)
            sub.add_argument("--html", action="store_true")
            sub.add_argument("--max-output", type=int, default=MAX_OUTPUT)

    args = parser.parse_args(argv)
    try:
        if args.command == "candidate":
            served = candidate(args.build_id)
            if args.action == "verify":
                print(json.dumps({"build_id": args.build_id, "verified": True}))
                return 0
            session = owned_session(args.build_id, args.purpose)
            route_url = served[f"{args.route}_url"]
            run_browser([*session_args(args.build_id, args.purpose), "open", route_url])
            print(json.dumps({"build_id": args.build_id, "session": session,
                              "route": args.route, "url": route_url, "opened": True}))
            return 0

        session = session_args(args.build_id, args.purpose)
        if args.command == "upload":
            path = Path(args.file).expanduser().resolve(strict=True)
            if not path.is_file() or not os.access(path, os.R_OK):
                raise BrowserError(f"upload path is not a readable regular file: {path}")
            output = run_browser([*session, "upload", args.selector, str(path)])
        elif args.command == "wait":
            if args.timeout_ms < 1:
                raise BrowserError("timeout-ms must be positive")
            env = os.environ.copy()
            env["AGENT_BROWSER_DEFAULT_TIMEOUT"] = str(args.timeout_ms)
            output = run_browser([*session, "wait", "--fn", args.fn],
                                 timeout=args.timeout_ms / 1000 + 5, env=env)
        elif args.command == "inspect":
            if args.max_output < 1:
                raise BrowserError("max-output must be positive")
            output = run_browser([*session, "snapshot", "--selector", args.selector])
            if args.html:
                html = run_browser([*session, "get", "html", args.selector])
                output = bounded(output + "\n" + html, args.max_output)
            else:
                output = bounded(output, args.max_output)
        else:
            output = run_browser([*session, "close"])
        if output:
            print(output)
        else:
            print(json.dumps({"ok": True, "command": args.command,
                              "session": owned_session(args.build_id, args.purpose)}))
        return 0
    except (BrowserError, OSError, ValueError, subprocess.SubprocessError) as error:
        print(f"migration-browser: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
