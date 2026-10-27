#!/usr/bin/env python3
"""Finite 011 preparation/capture. No builds or browser launches without a run subcommand.

The Cadrum paths are kept deliberately: they let the recorded old-versus-new baseline be
reproduced from a historical checkout. Production code no longer uses Cadrum.
"""

from __future__ import annotations

import argparse
import ast
import csv
import difflib
import gzip
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import platform
import re
import statistics
import subprocess
import sys
import threading
import time
from http.server import ThreadingHTTPServer

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
import browser_tools  # noqa: E402

HISTORICAL = Path("/home/chris/.t3/worktrees/ts-boardstudio2/t3-2088b1aa")
OLD_HEAD = "6c9c4867755bd2316f751783729a3ff9778fc04a"
FIXTURES = ROOT / "cad/test/fixtures/performance"
PROFILES = ("checked", "release")
INCREMENTAL_COMMENT = "// BoardStudio CAD performance incremental rebuild probe.\n"
PAIRED = tuple(f"{name}:{op}" for name in
               ("C1plate", "C2trayboss", "C3gasket", "P1analyticPCB")
               for op in ("preview", "step")) + (
                   "V1switch:import", "V2vendor:import", "K1keycap:preview", "K1keycap:print")
UNPAIRED = ("V2vendor:refine", "Qvendor:default", "Qvendor:tight")


def module_at(path, name):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def git(root, *args):
    return subprocess.check_output(["git", "-C", str(root), *args], text=True).strip()


def write_csv(path, fields, rows):
    with path.open("w") as output:
        writer = csv.DictWriter(output, fieldnames=fields)
        writer.writeheader()
        writer.writerows(rows)


def schedule(build_cycles=1, runs=1, backends=("monstertruck",)):
    """Finite capture schedule. The default is the Monstertruck-only measurement
    (one cold-plus-five-warm session per workload); a two-backend tuple keeps the
    historical alternating comparison available for old captures."""
    rows = []
    for profile in PROFILES:
        for pair in range(1, runs + 1):
            order = backends[::-1] if pair % 2 == 0 else backends
            for workload in (*PAIRED, "startup:cold", "startup:warm"):
                for backend in order:
                    rows.append(dict(profile=profile, pair=pair, backend=backend,
                                     workload=workload, state="cold+5warm" if not
                                     workload.startswith("startup:") else workload.split(":")[1]))
        for backend in backends:
            for workload in PAIRED:
                rows.append(dict(profile=profile, pair=0, backend=backend,
                                 workload=workload, state="growth20"))
        for workload in UNPAIRED:
            for pair in range(1, runs + 1):
                rows.append(dict(profile=profile, pair=pair, backend="monstertruck",
                                 workload=workload, state="cold+5warm;unpaired"))
            rows.append(dict(profile=profile, pair=0, backend="monstertruck",
                             workload=workload, state="growth20;unpaired"))
        for pair in range(1, build_cycles + 1):
            order = backends[::-1] if pair % 2 == 0 else backends
            for backend in order:
                for state in ("clean", "noop", "incremental"):
                    rows.append(dict(profile=profile, pair=pair, backend=backend,
                                     workload="build", state=state))
    return rows


def existing_artifacts(root, receipt, profile):
    """Validate the existing integration receipt, without inventing a timed build record."""
    content = receipt.read_text()
    match = re.search(r"Current combined source ([0-9a-f]{40});", content)
    if not match or profile != "checked" or "--no-opt" not in content or "Dioxus page debug" not in content:
        raise ValueError("existing receipt must identify tested source and checked/no-opt artifacts")
    tested = match[1]
    changes = git(root, "diff", "--name-only", tested, "HEAD").splitlines()
    dirty = git(root, "diff", "--name-only", "HEAD").splitlines()
    if any(not path.endswith(".md") for path in changes + dirty):
        raise ValueError("measured checkout has production changes beyond the receipt's tested source")
    site = root / "web/target/site/site-subpath/boardstudio"
    provider = "boardstudio_cadrum_wasm" if (root / "cad/scripts/cadrum_build.py").is_file() else "boardstudio_cad_wasm"
    pairs = (
        (f"cad/{provider}_bg.wasm", f"cad/wasm/pkg/{provider}_bg.wasm"),
        (f"cad/{provider}.js", f"cad/wasm/pkg/{provider}.js"),
        ("cad-worker/m1_cad_worker_bg.wasm", "web/target/providers/cad-worker/m1_cad_worker_bg.wasm"),
        ("cad-worker/m1_cad_worker.js", "web/target/providers/cad-worker/m1_cad_worker.js"),
    )
    for packaged, original in pairs:
        expected = re.search(re.escape(f"BYTE_IDENTICAL packaged assets/{packaged} == {original}; ") + r"(\d+) bytes\.", content)
        if not expected:
            raise ValueError(f"existing receipt omits {packaged}")
        data = (root / original).read_bytes()
        if len(data) != int(expected[1]) or (site / "assets" / packaged).read_bytes() != data:
            raise ValueError(f"packaged/original artifact differs from receipt: {packaged}")
    if not (site / "index.html").is_file():
        raise ValueError("packaged page index missing")
    return f"validation_only=true\ntested_source={tested}\ncheckout_head={git(root, 'rev-parse', 'HEAD')}\ndocs_only_successors={changes!r}\nreceipt={receipt}\n{content}"


def binding(root, profile):
    """Reuse the existing receipt digest, official helper/stamp and Git; no source manifest."""
    receipt = module_at(ROOT / "scripts/run-drag-browser.py", "perf_receipt")
    driver_head, driver_source = receipt.source_identity()
    receipt.ROOT = root
    head, source = receipt.source_identity()
    old = (root / "cad/scripts/cadrum_build.py").is_file()
    helper = module_at(root / ("cad/scripts/cadrum_build.py" if old else
                              "cad/scripts/cad_build.py"), "perf_cad_helper")
    stamp = root / f"cad/.cache/{'cadrum' if old else 'cad'}/wasm-build.sha256"
    inputs = helper.wasm_inputs_digest(no_opt=profile == "checked")
    lines = [f"checkout={root}", f"backend={'cadrum' if old else 'monstertruck'}",
             f"head={head}", f"source_sha256={source}",
             f"branch={git(root, 'branch', '--show-current') or 'detached'}", f"profile={profile}",
             f"provider_profile={'release-no-opt' if profile == 'checked' else 'release'}",
             f"driver_head={driver_head}", f"driver_source_sha256={driver_source}",
             f"driver_status={git(ROOT, 'status', '--porcelain=v1')!r}",
             f"status={git(root, 'status', '--porcelain=v1')!r}",
             f"official_provider_inputs={inputs}",
             f"provider_stamp={stamp.read_text().strip() if stamp.is_file() else 'MISSING'}"]
    for name in ("target", "cad/wasm/target", "web/target", "renderer/target"):
        path = root / name
        lines.append(f"resolved_{name}={path.resolve()} exists={path.exists()}")
    if old:
        lines.append(f"historical_containerfile_blob={git(root, 'rev-parse', 'HEAD:cad/wasm/Containerfile')}")
        try:
            image = json.loads(subprocess.check_output(
                [os.environ.get("CADRUM_CONTAINER_RUNTIME", "podman"), "image", "inspect", helper.IMAGE], text=True, timeout=15))[0]
            lines.append("historical_image=" + json.dumps({key: image.get(key) for key in
                         ("Id", "Digest", "RepoDigests", "Architecture", "Os")}, sort_keys=True))
        except (OSError, ValueError, subprocess.SubprocessError) as error:
            lines.append(f"historical_image=UNAVAILABLE {error}")
        name, expected = helper.BUILDS["wasm"]
        archive = root / "cad/.cache/cadrum/wasm" / name
        if archive.is_file():
            with archive.open("rb") as source:
                actual = hashlib.file_digest(source, "sha256").hexdigest()
            lines.append(f"historical_prebuilt_archive={archive} bytes={archive.stat().st_size} pinned={expected} actual={actual}")
        else:
            lines.append(f"historical_prebuilt_archive=MISSING {archive} pinned={expected}")
    for name in ("cad/wasm/pkg", "web/target/site/site-subpath/boardstudio"):
        base = root / name
        files = sorted(p for p in base.rglob("*") if p.is_file())
        lines.append(f"artifact_{name}={receipt.digest_files(files, base) if files else 'MISSING'}")
    for path in sorted(path for path in FIXTURES.iterdir() if path.suffix in (".json", ".boardstudio")):
        relative = path.relative_to(ROOT).as_posix()
        lines.append(f"workload_blob_{path.name}={git(ROOT, 'hash-object', relative)}")
    lines.append("owned_diff:\n" + git(root, "diff", "--binary", "HEAD"))
    return lines, stamp.is_file() and stamp.read_text().strip() == inputs


def build_binding(text):
    """Read the ordinary receipt, including its exact Git patch; never infer success from exit alone."""
    header, separator, owned_diff = text.partition("\nowned_diff:\n")
    if not separator:
        raise ValueError("missing build binding owned patch")
    fields = {}
    for line in header.splitlines():
        key, separator, value = line.partition("=")
        if not separator or not value or key in fields:
            raise ValueError("incomplete/duplicate build binding field")
        fields[key] = value
    required = {"backend", "head", "source_sha256", "status", "profile", "provider_profile",
                "driver_head", "driver_source_sha256", "driver_status", "official_provider_inputs",
                "provider_stamp", "artifact_cad/wasm/pkg", "artifact_web/target/site/site-subpath/boardstudio"}
    required.update(f"workload_blob_{path.name}" for path in FIXTURES.iterdir()
                    if path.suffix in (".json", ".boardstudio"))
    if not required <= fields.keys():
        raise ValueError(f"missing build binding fields: {sorted(required - fields.keys())}")
    hashes = {"head", "driver_head", "source_sha256", "driver_source_sha256", "official_provider_inputs",
              "provider_stamp", "artifact_cad/wasm/pkg", "artifact_web/target/site/site-subpath/boardstudio"}
    hashes.update(key for key in required if key.startswith("workload_blob_"))
    for key in hashes:
        length = 40 if key in ("head", "driver_head") or key.startswith("workload_blob_") else 64
        if fields[key] == "MISSING" and key.startswith(("provider_stamp", "artifact_")):
            continue
        if not re.fullmatch(f"[0-9a-f]{{{length}}}", fields[key]):
            raise ValueError(f"invalid build binding {key}")
    for key in ("status", "driver_status"):
        try:
            if not isinstance(ast.literal_eval(fields[key]), str):
                raise ValueError("status must be a string")
        except (SyntaxError, ValueError) as error:
            raise ValueError(f"invalid build binding {key}") from error
    fields["owned_diff"] = owned_diff.rstrip("\n")
    return fields


def validate_build_bindings(before_text, after_text, backend, profile, state):
    before, after = build_binding(before_text), build_binding(after_text)
    if backend not in ("cadrum", "monstertruck") or profile not in PROFILES or state not in ("clean", "noop", "incremental"):
        raise ValueError("invalid build row binding backend/profile/state")
    for bound in (before, after):
        if (bound["backend"], bound["profile"], bound["provider_profile"]) != (
                backend, profile, "release-no-opt" if profile == "checked" else "release"):
            raise ValueError("build row backend/profile binding mismatch")
    # A build creates output directories/stamps/artifacts. Everything else must stay fixed.
    def fixed(bound):
        return {key: value.split(" exists=", 1)[0] if key.startswith("resolved_") else value
                for key, value in bound.items() if key != "provider_stamp" and not key.startswith("artifact_")}
    if fixed(before) != fixed(after):
        raise ValueError("build source/harness/workload binding drift")
    diff = before["owned_diff"]
    if state == "incremental":
        path = "cad/wasm/src/lib.rs"
        try:
            base = subprocess.check_output(["git", "show", f"{before['head']}:{path}"], cwd=ROOT, text=True)
        except subprocess.CalledProcessError as error:
            raise ValueError("incremental binding base Git object unavailable") from error
        if not base.endswith("\n") or INCREMENTAL_COMMENT in base:
            raise ValueError("incremental binding base cannot accept the owned comment patch")
        expected = list(difflib.unified_diff(base.splitlines(), (base + INCREMENTAL_COMMENT).splitlines(),
                                           fromfile=f"a/{path}", tofile=f"b/{path}", lineterm=""))
        lines = diff.splitlines()
        # Git adds a function label to hunk headers; it does not change the owned patch.
        actual = [re.sub(r"^(@@ -[0-9,]+ \+[0-9,]+ @@).*$", r"\1", line) for line in lines[2:]]
        if (len(lines) < 3 or lines[0] != f"diff --git a/{path} b/{path}"
                or not re.fullmatch(r"index [0-9a-f]{7,40}\.\.[0-9a-f]{7,40} 100644", lines[1])
                or actual != expected):
            raise ValueError("build binding has an unauthorized incremental patch")
    elif diff or ast.literal_eval(before["status"]) != "":
        raise ValueError("clean/noop build binding must have no owned patch")
    expected_source = hashlib.sha256((diff + "\n" if diff else "").encode()).hexdigest()
    if before["source_sha256"] != expected_source:
        raise ValueError("build binding source digest includes an unowned patch/untracked input")
    if after["provider_stamp"] != after["official_provider_inputs"] or after["provider_stamp"] == "MISSING":
        raise ValueError("build binding has no current official provider stamp")
    if any(after[key] == "MISSING" for key in ("artifact_cad/wasm/pkg", "artifact_web/target/site/site-subpath/boardstudio")):
        raise ValueError("build binding has missing packaged artifacts")
    source = (after["head"], after["profile"], after["provider_profile"])
    harness = tuple(sorted((key, value) for key, value in after.items()
                           if key.startswith(("driver_", "workload_blob_"))))
    return source, harness, (after["source_sha256"], diff, after["official_provider_inputs"])


def environment():
    lines = [f"platform={platform.platform()}", f"python={sys.version}",
             f"load={os.getloadavg()}", "memory:\n" + Path("/proc/meminfo").read_text()]
    for key in ("CARGO_TARGET_DIR", "CARGO_BUILD_JOBS", "CARGO_INCREMENTAL",
                "CARGO_PROFILE_DEV_DEBUG", "RUSTFLAGS", "RUSTUP_TOOLCHAIN", "NODE_PATH",
                "CHROME_BIN", "CADRUM_IMAGE_READY", "CADRUM_CONTAINER_RUNTIME"):
        lines.append(f"env_{key}={os.environ.get(key, '<unset>')}")
    for command in (("rustc", "--version"), ("cargo", "--version"), ("wasm-pack", "--version"),
                    ("wasm-bindgen", "--version"), ("wasm-opt", "--version"),
                    ("dx", "--version"), ("node", "--version"), ("chromium", "--version")):
        try:
            result = subprocess.run(command, capture_output=True, text=True, timeout=15)
            lines.append(f"{command[0]}={result.stdout.strip()} {result.stderr.strip()} exit={result.returncode}")
        except (OSError, subprocess.TimeoutExpired) as error:
            lines.append(f"{command[0]}=UNAVAILABLE {error}")
    for path in (Path("/proc/cpuinfo"), Path("/proc/pressure/cpu"),
                 Path("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor")):
        if path.is_file():
            lines.append(f"{path}:\n{path.read_text()}")
    lines.append("processes:\n" + subprocess.check_output(
        ["ps", "-eo", "pid,ppid,comm,pcpu,pmem", "--sort=-pcpu"], text=True))
    return "\n".join(lines) + "\n"


def sizes(root, output):
    """Each group is a separate product view. Only site is the nonduplicated shipped total."""
    rows = []
    for name in ("cad/wasm/pkg", "web/target/providers", "renderer/pkg",
                 "web/target/site/site-subpath/boardstudio"):
        base = root / name
        for file in sorted(base.rglob("*")):
            if file.is_file():
                data = file.read_bytes()
                rows.append(dict(group=name, path=file.relative_to(base).as_posix(),
                                 bytes=len(data), gzip9_bytes=len(gzip.compress(data, compresslevel=9, mtime=0))))
    write_csv(output, ("group", "path", "bytes", "gzip9_bytes"), rows)


def summarize(inputs, output, build_cycles=1, runs=1):
    """Session medians keep five warm values from pretending to be five cold observations."""
    sessions = {}
    failures = []
    seen = set()
    source_bindings, harness_bindings = {}, {}
    build_sources, build_harnesses, build_states = {}, {}, {}
    for directory in inputs:
        if (directory / "diagnostic.csv").exists():
            raise ValueError("validation-only diagnostic captures cannot become benchmark samples")
        log = (directory / "command.log").read_text()
        if "\nexit=0\n" not in log:
            failures.append(str(directory))
            continue
        if (directory / "runtime.csv").is_file():
            if (directory / "binding-before.log").read_text() != (directory / "binding-after.log").read_text():
                failures.append(str(directory) + " (binding drift)")
                continue
            with (directory / "runtime.csv").open() as source:
                rows = list(csv.DictReader(source))
                if not rows:
                    raise ValueError(f"empty runtime capture: {directory}")
                observation = tuple(rows[0][k] for k in ("backend", "profile", "workload", "pair"))
                if observation in seen:
                    raise ValueError(f"duplicate observation: {observation}")
                seen.add(observation)
                binding_lines = (directory / "binding-before.log").read_text().splitlines()
                source_key = observation[:2]
                source_signature = tuple(line for line in binding_lines if line.startswith(
                    ("head=", "source_sha256=", "profile=", "official_provider_inputs=", "provider_stamp=", "artifact_")))
                harness_signature = tuple(line for line in binding_lines if line.startswith(
                    ("driver_head=", "driver_source_sha256=", "workload_blob_")))
                if not source_signature or not harness_signature:
                    raise ValueError(f"missing source/harness binding: {directory}")
                if source_bindings.setdefault(source_key, source_signature) != source_signature:
                    raise ValueError(f"mixed source/artifacts for {source_key}")
                if harness_bindings.setdefault(observation[1], harness_signature) != harness_signature:
                    raise ValueError("matched comparison has different harness/workload bindings")
                for row in rows:
                    key = tuple(row[k] for k in ("backend", "profile", "workload", "pair", "state", "metric", "unit"))
                    sessions.setdefault(key, []).append(float(row["value"]))
        else:
            with (directory / "build.csv").open() as source:
                for row in csv.DictReader(source):
                    if row["status"] != "ok":
                        failures.append(str(directory))
                        continue
                    try:
                        before_text = (directory / "binding-before.log").read_text()
                        after_text = (directory / "binding-after.log").read_text()
                    except FileNotFoundError as error:
                        raise ValueError(f"missing build binding: {directory}") from error
                    source, harness, patch = validate_build_bindings(
                        before_text, after_text, row["backend"], row["profile"], row["state"])
                    source_key = (row["backend"], row["profile"])
                    if build_sources.setdefault(source_key, source) != source:
                        raise ValueError(f"mixed build source/profile binding for {source_key}")
                    patch_key = (*source_key, "incremental" if row["state"] == "incremental" else "base")
                    if build_states.setdefault(patch_key, patch) != patch:
                        raise ValueError(f"mixed build patch/provider inputs for {source_key}")
                    if build_harnesses.setdefault(row["profile"], harness) != harness:
                        raise ValueError("matched comparison has different harness/workload bindings")
                    key = (row["backend"], row["profile"], "build", row["pair"], row["state"], "wall", "s")
                    if key in seen:
                        raise ValueError(f"duplicate observation: {key}")
                    seen.add(key)
                    sessions.setdefault(key, []).append(float(row["seconds"]))
    grouped, paired = {}, {}
    for key, values in sessions.items():
        backend, profile, workload, pair, state, metric, unit = key
        value = statistics.median(values)
        group = (backend, profile, workload, state, metric, unit)
        grouped.setdefault(group, []).append(value)
        if pair != "0":
            paired.setdefault((profile, workload, pair, state, metric, unit), {})[backend] = value
    fields = ("backend", "profile", "workload", "state", "metric", "unit")
    write_csv(output / "summary.csv", (*fields, "sessions", "median", "min", "max"), [
        dict(zip(fields, key), sessions=len(values), median=statistics.median(values), min=min(values), max=max(values))
        for key, values in sorted(grouped.items())])
    fields = ("profile", "workload", "pair", "state", "metric", "unit")
    write_csv(output / "pairs.csv", (*fields, "cadrum", "monstertruck", "new_minus_old", "new_over_old"), [
        dict(zip(fields, key), cadrum=values["cadrum"], monstertruck=values["monstertruck"],
             new_minus_old=values["monstertruck"] - values["cadrum"],
             new_over_old=values["monstertruck"] / values["cadrum"] if values["cadrum"] else "undefined")
        for key, values in sorted(paired.items()) if set(values) == {"cadrum", "monstertruck"}])
    coverage = []
    observed_backends = tuple(sorted({key[0] for key in sessions}, reverse=True)) or ("monstertruck",)
    for row in schedule(build_cycles, runs, observed_backends):
        if row["workload"] == "build":
            phases = ((row["state"], "wall", "s", 1),)
        elif row["workload"].startswith("startup:"):
            phases = ((row["state"], "navigation-to-usable-landing", "ms", 1),
                      ("worker-startup", "worker-create-to-ready", "ms", 1),
                      ("cold", "dispatch-to-validated-reply", "ms", 1))
        elif row["state"].startswith("growth20"):
            phases = (("growth", "dispatch-to-validated-reply", "ms", 20),)
        else:
            phases = (("cold", "dispatch-to-validated-reply", "ms", 1),
                      ("warm", "dispatch-to-validated-reply", "ms", 5))
        for state, metric, unit, expected in phases:
            key = (row["backend"], row["profile"], row["workload"], str(row["pair"]), state, metric, unit)
            count = len(sessions.get(key, []))
            coverage.append(dict(backend=row["backend"], profile=row["profile"], workload=row["workload"],
                                 pair=row["pair"], state=state, metric=metric, expected=expected, observed=count,
                                 status="complete" if count == expected else "missing-or-extra"))
    write_csv(output / "coverage.csv", ("backend", "profile", "workload", "pair", "state", "metric", "expected", "observed", "status"), coverage)
    (output / "report.log").write_text(f"input_captures={len(inputs)} failed_captures={len(failures)}\n"
                                      f"complete_schedule_cells={sum(row['status'] == 'complete' for row in coverage)}\n"
                                      f"missing_or_extra_schedule_cells={sum(row['status'] != 'complete' for row in coverage)}\n"
                                      "Descriptive capture only. U1/downloads and independent quality/admission remain separate.\n")
    (output / "failures.log").write_text("\n".join(failures) + "\n")
    if failures:
        raise ValueError("failed observations retained; diagnose and invalidate their whole pairs before reporting wins")


def process_memory(pid):
    """Return PSS/RSS KiB from one live process, never process high-water marks."""
    values = {}
    for line in Path(f"/proc/{pid}/smaps_rollup").read_text().splitlines():
        key, _, value = line.partition(":")
        if key in ("Pss", "Rss"):
            values[key] = int(value.split()[0])
    return values["Pss"], values["Rss"]


def sample_tree(pid):
    parents, commands = {}, {}
    for path in Path("/proc").iterdir():
        if not path.name.isdigit():
            continue
        try:
            tail = (path / "stat").read_text().rsplit(")", 1)[1].split()
            parents[int(path.name)] = int(tail[1])
            commands[int(path.name)] = (path / "cmdline").read_bytes().split(b"\0")
        except (OSError, ValueError):
            continue
    owned = {pid}
    while True:
        children = {p for p, parent in parents.items() if parent in owned} - owned
        if not children:
            break
        owned.update(children)
    # Playwright's own browser descendant has this launch flag. Keep the Node controller separate.
    browsers = {p for p in owned if b"--remote-debugging-pipe" in commands.get(p, [])}
    browser_tree = set(browsers)
    while True:
        children = {p for p in owned if parents.get(p) in browser_tree} - browser_tree
        if not children:
            break
        browser_tree.update(children)
    rows = []
    for process in sorted(owned):
        try:
            pss, rss = process_memory(process)
            rows.append((process, "browser" if process in browser_tree else "controller", pss, rss, "ok"))
        except (OSError, KeyError, ValueError):
            rows.append((process, "browser" if process in browser_tree else "controller", "", "", "unavailable"))
    return rows


def capture(command, root, out, timeout, env=None):
    (out / "environment-before.log").write_text(environment())
    started = time.monotonic()
    with (out / "command.log").open("w") as log, (out / "memory.csv").open("w") as memory, \
            (out / "stdout-timing.csv").open("w") as timing:
        log.write(f"cwd={root}\nargv={command!r}\nsampling_interval_seconds=0.05\n")
        log.flush()
        writer = csv.writer(memory)
        writer.writerow(("monotonic_s", "unix_s", "pid", "scope", "pss_kib", "rss_kib", "status"))
        lock_fd = browser_tools._inherited_build_lock()
        process = subprocess.Popen(command, cwd=root, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                                   env=env, start_new_session=True, text=True, bufsize=1,
                                   pass_fds=(lock_fd,) if lock_fd is not None else ())
        done = threading.Event()
        timed_lines = csv.writer(timing)
        timed_lines.writerow(("monotonic_s", "unix_s", "line"))

        def drain():
            for line in process.stdout:
                timed_lines.writerow((time.monotonic(), time.time(), line.rstrip("\n")))
                log.write(line)
                log.flush()

        reader = threading.Thread(target=drain, daemon=True)
        reader.start()

        def sample():
            while not done.is_set():
                now = time.monotonic()
                unix = time.time()
                for row in sample_tree(process.pid):
                    writer.writerow((now, unix, *row))
                memory.flush()
                done.wait(max(0, .05 - (time.monotonic() - now)))

        thread = threading.Thread(target=sample, daemon=True)
        thread.start()
        try:
            code = process.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            import signal
            os.killpg(process.pid, signal.SIGTERM)
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait()
            code = 124
        finally:
            done.set()
            thread.join()
            reader.join(timeout=10)
            if reader.is_alive():
                import signal
                try:
                    os.killpg(process.pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass
                reader.join(timeout=2)
                code = 125
                log.write("stdout remained open after child exit; observation invalid\n")
        if not reader.is_alive():
            process.stdout.close()
        elapsed = time.monotonic() - started
        log.write(f"\nexit={code}\nwall_seconds={elapsed}\n")
    (out / "environment-after.log").write_text(environment())
    return code, elapsed


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("plan", "bind", "build", "runtime", "validate", "report"))
    parser.add_argument("--checkout", type=Path, default=ROOT)
    parser.add_argument("--source-head", help="root's frozen full source commit; required for execution")
    parser.add_argument("--profile", choices=PROFILES, default="checked")
    parser.add_argument("--output", type=Path, required=True, help="new directory under this lane's .evidence")
    parser.add_argument("--workload", choices=(*PAIRED, *UNPAIRED, "startup:cold", "startup:warm", "U1:freeze", "U1:page"))
    parser.add_argument("--pair", type=int, choices=range(0, 6), default=1)
    parser.add_argument("--growth", action="store_true")
    parser.add_argument("--state", choices=("clean", "noop", "incremental"))
    parser.add_argument("--build-run", type=Path, help="successful measure-cad build capture directory for the served artifacts")
    parser.add_argument("--inputs", type=Path, nargs="+", help="completed capture directories for report")
    parser.add_argument("--admission-note", type=Path, help="the actual environment and load record for this frozen source/artifacts; no quiet-window claim is implied")
    parser.add_argument("--artifact-receipt", type=Path, help="existing official integration artifact receipt, validation only")
    parser.add_argument("--sessions", type=int, choices=range(1, 6), default=1,
                        help="cold-plus-five-warm sessions per workload (default 1: descriptive single-backend measurement)")
    parser.add_argument("--build-cycles", type=int, choices=(1, 3), default=1,
                        help="one matched initial build cycle; three is the retained upper proposal, only after root requests it")
    args = parser.parse_args()
    root, out = args.checkout.resolve(), args.output.resolve()
    if not out.is_relative_to(ROOT / ".evidence"):
        parser.error("capture outputs must be under this lane's .evidence")
    if out.exists():
        parser.error("use a new output directory; failed observations are never overwritten")
    out.mkdir(parents=True)
    if args.action == "plan":
        write_csv(out / "schedule.csv", ("profile", "pair", "backend", "workload", "state"),
                  schedule(args.build_cycles, args.sessions))
        print(f"Untimed finite schedule: {out / 'schedule.csv'}; root clearance/admission still required")
        return 0
    if args.action == "report":
        if not args.inputs or not args.admission_note or not args.admission_note.is_file():
            parser.error("report requires --inputs and --admission-note; an observed reply is not geometry admission")
        (out / "admission.log").write_text(args.admission_note.read_text())
        summarize(args.inputs, out, args.build_cycles, args.sessions)
        print(f"Descriptive samples: {out}; Monstertruck-only unless historical two-backend captures were supplied")
        return 0
    before, current = binding(root, args.profile)
    (out / "binding-before.log").write_text("\n".join(before) + "\n")
    if args.action == "bind":
        sizes(root, out / "sizes.csv")
        print(f"Read-only binding: {out}; official stamp matches={current}")
        return 0
    if not args.source_head or git(root, "rev-parse", "HEAD") != args.source_head:
        parser.error("execution requires the exact root-cleared --source-head")
    if args.growth and args.pair != 0:
        parser.error("growth sequence is separate from the five pairs; pass --pair 0")
    old = (root / "cad/scripts/cadrum_build.py").is_file()
    backend = "cadrum" if old else "monstertruck"
    if args.action == "build":
        if root == HISTORICAL:
            parser.error("historical baseline lane/artifacts are read-only; provision a measurement workspace")
        if not args.state:
            parser.error("build requires --state clean/noop/incremental")
        if args.state == "clean" and any((root / p).exists() for p in (
                "target", "cad/wasm/target", "cad/wasm/pkg", "web/target", "renderer/pkg", "renderer/target")):
            parser.error("clean cycle requires fresh dedicated output directories; this tool never cleans")
        if old and os.environ.get("CADRUM_IMAGE_READY") != "1":
            parser.error("verify pinned image/archive first, then set CADRUM_IMAGE_READY=1")
        if old and (not any(line.startswith("historical_image=") and
                            "569918e5874f8908957a76e2b2df7518d9c7886f5d224c5536ec2ca9896fd2f0" in line for line in before)
                    or not any(line.startswith("historical_prebuilt_archive=") and
                               "actual=8149e781acdbd21507cfb29937e48e5fdbe628ee5c37007f720dcf5d4000e6ad" in line for line in before)):
            parser.error("pinned historical image/archive provenance unavailable; no automatic provisioning fallback")
        browser_tools.locate()  # Fail before expensive compilation if the retained browser prerequisites drifted.
        command = [sys.executable, "scripts/build-web.py", *(["--fast"] if args.profile == "checked" else [])]
        code, elapsed = capture(command, root, out, 7200)
        sizes(root, out / "sizes.csv")
    else:
        if not args.workload or not current:
            parser.error("runtime requires --workload and the selected profile's current official CAD stamp")
        if old and args.workload in UNPAIRED:
            parser.error("historical Cadrum has no admitted refinement/owned-query counterpart")
        validation = args.action == "validate"
        if args.workload == "U1:freeze" and not validation:
            parser.error("U1:freeze is validation-only; use the committed frozen archive")
        if validation:
            if args.growth or not args.artifact_receipt:
                parser.error("validate requires --artifact-receipt and never runs repetition/growth sequences")
            (out / "artifact-validation.log").write_text(existing_artifacts(root, args.artifact_receipt, args.profile))
        else:
            if not args.admission_note or not args.admission_note.is_file():
                parser.error("runtime needs root's --admission-note for this frozen source/artifacts/quiet window")
            (out / "admission.log").write_text(args.admission_note.read_text())
            if not args.build_run or not (args.build_run / "binding-after.log").is_file():
                parser.error("runtime requires --build-run from a successful source-bound full build capture")
            build_binding = (args.build_run / "binding-after.log").read_text()
            if "\nexit=0\n" not in (args.build_run / "command.log").read_text():
                parser.error("the referenced full build failed")
            bound_fields = ("head=", "source_sha256=", "profile=", "official_provider_inputs=", "provider_stamp=", "artifact_")
            selected = lambda lines: [line for line in lines if line.startswith(bound_fields)]
            if selected(build_binding.splitlines()) != selected(before):
                parser.error("full build binding does not match these served source/profile/artifacts")
        package = root / "cad/wasm/pkg"
        staged = root / "web/target/site/site-subpath/boardstudio/assets/cad"
        for file in package.glob("*.js"):
            if not (staged / file.name).is_file() or file.read_bytes() != (staged / file.name).read_bytes():
                parser.error("packaged CAD JS differs from the official source-bound package")
        for file in package.glob("*.wasm"):
            if not (staged / file.name).is_file() or file.read_bytes() != (staged / file.name).read_bytes():
                parser.error("packaged CAD WASM differs from the official source-bound package")
        site = root / "web/target/site"
        if not (site / "site-subpath/boardstudio/assets/cad-worker/entry.js").is_file():
            parser.error("runtime requires a fresh source-bound full page/worker package; provider-only is insufficient")
        node, chrome = browser_tools.locate()
        serve = module_at(ROOT / "scripts/serve-web.py", "perf_serve")
        base_handler = serve.handler_for(site)

        class Handler(base_handler):
            def translate_path(self, path):
                url = path.partition("?")[0]
                if url.startswith("/__perf/"):
                    relative = url.removeprefix("/__perf/")
                    file = (ROOT / relative).resolve()
                    return str(file) if file.is_relative_to(ROOT) else "/dev/null"
                if url.startswith("/__source/"):
                    file = (root / url.removeprefix("/__source/")).resolve()
                    return str(file) if file.is_relative_to(root) else "/dev/null"
                return super().translate_path(path)

            def log_message(self, *args):
                pass

        server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        command = [node, str(ROOT / "cad/scripts/measure-cad-runtime.cjs"),
                   f"http://127.0.0.1:{server.server_port}", backend, args.profile,
                   args.workload, str(out), str(args.pair), "validation" if validation else "growth20" if args.growth else "latency"]
        try:
            if validation:
                with (out / "validation.log").open("w") as log:
                    log.write("Validation only; all observed values are diagnostic, not benchmark results.\n")
                    log.flush()
                    process = subprocess.Popen(command, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT,
                                               env=dict(os.environ, CHROME_BIN=chrome), start_new_session=True)
                    try:
                        code = process.wait(timeout=600)
                    except subprocess.TimeoutExpired:
                        import signal
                        os.killpg(process.pid, signal.SIGTERM)
                        try:
                            process.wait(timeout=10)
                        except subprocess.TimeoutExpired:
                            os.killpg(process.pid, signal.SIGKILL)
                            process.wait()
                        code = 124
                    log.write(f"\nvalidation_exit={code}; diagnostic-only, never a build/runtime benchmark receipt\n")
            else:
                code, _ = capture(command, root, out, 5400, dict(os.environ, CHROME_BIN=chrome))
        finally:
            server.shutdown()
            server.server_close()
            thread.join(timeout=3)
    after, _ = binding(root, args.profile)
    (out / "binding-after.log").write_text("\n".join(after) + "\n")
    if args.action == "build":
        status = "ok" if code == 0 else f"exit-{code}"
        if code == 0:
            try:
                validate_build_bindings("\n".join(before) + "\n", "\n".join(after) + "\n",
                                        backend, args.profile, args.state)
            except ValueError as error:
                (out / "binding-rejection.log").write_text(str(error) + "\n")
                code, status = 1, "binding-rejected"
        write_csv(out / "build.csv", ("backend", "profile", "pair", "state", "seconds", "status"),
                  [dict(backend=backend, profile=args.profile, pair=args.pair, state=args.state,
                        seconds=elapsed, status=status)])
    if args.action in ("runtime", "validate") and before != after:
        raise SystemExit("source/artifact/stamp drift: invalidate this observation and its pair")
    print(f"Captured {args.action}, exit={code}, output={out}; admission/review required before comparison")
    return code


if __name__ == "__main__":
    raise SystemExit(main())
