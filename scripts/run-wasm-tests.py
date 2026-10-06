#!/usr/bin/env python3
"""Run the headless-Chrome `#[wasm_bindgen_test]` suite (the tests native `cargo test` never compiles).

  run-wasm-tests.py --files web/src/presentation/foo.rs ...   run only the modules those files define
  run-wasm-tests.py --all                                     run everything

Known failures (scripts/wasm-known-failures.json) are executed but do not fail the
run; a known failure that now passes is reported so the entry can be removed.

wasm-bindgen-test-runner accepts a single FILTER, so each module filter is one runner invocation.
Override the runner with BOARDSTUDIO_WASM_TEST_COMMAND (shell-split; the filter, if any, is appended).
"""

import argparse
from collections import Counter
from contextlib import contextmanager, nullcontext
import glob
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import shlex
import shutil
import subprocess
import sys
import time
from tempfile import TemporaryDirectory

ROOT = Path(__file__).resolve().parent.parent
KNOWN_FAILURES = Path(__file__).with_name("wasm-known-failures.json")
TEST_OWNERS = Path(__file__).with_name("wasm-test-owners.json")
RUNNER_ENV = "BOARDSTUDIO_WASM_TEST_COMMAND"
WASM_PACK = ["wasm-pack", "test", "--headless", "--chrome", "--mode", "no-install", "web",
             "--no-default-features", "--features", "page", "--bin", "boardstudio-web", "--"]
CHROME_BINARIES = ("google-chrome-stable", "google-chrome", "chromium", "chromium-browser", "chrome")
WEBDRIVER_CONFIG_ENV = "WASM_BINDGEN_TEST_WEBDRIVER_JSON"
TEST_LINE = re.compile(r"^test (\S+) \.\.\. (ok|FAILED|FAIL)\b", re.M)
INVOKED_LINE = re.compile(r"^\s*Invoking test: (\S+)", re.M)
class RunnerError(RuntimeError):
    pass


class DuplicateTestListingError(RunnerError):
    def __init__(self, names):
        self.listed_tests = names
        self.duplicate_names = sorted(name for name, count in Counter(names).items() if count > 1)
        super().__init__("duplicate test names in wasm listing: " + ", ".join(self.duplicate_names))


def path_attr_modules(root):
    """Map active wasm `#[path]` modules to their declaring file and module name."""
    found = {}
    root = Path(root).resolve()
    base = root / "web/src"
    lint_path = Path(__file__).with_name("check-wasm-tests.py")
    spec = importlib.util.spec_from_file_location("check_wasm_tests_for_runner", lint_path)
    lint = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(lint)
    roots = [path.resolve() for path in (base / "main.rs", base / "lib.rs") if path.is_file()]
    compiled = lint.reachable(roots, lint.WASM, {})
    root_files = set(roots)
    for source, record in compiled.items():
        for declaration in record["mods"]:
            if declaration["path"] is None:
                continue
            if not all(lint.eval_cfg(cfg, lint.WASM) for cfg in declaration["cfgs"]):
                continue
            target = lint.resolve(source, declaration, root_files)
            if target and target.is_file():
                found[target.relative_to(root).as_posix()] = (
                    source.relative_to(root).as_posix(), declaration["name"])
    return found


def module_filter(path, path_modules=None):
    """Map a repo-relative web/src/**.rs path to a test-name prefix filter ('' means every test)."""
    path = path.replace("\\", "/")
    if path.startswith("web/crates/") and path.endswith(".rs"):
        crate = path.split("/")[2]
        raise RunnerError(f"{path} belongs to the web/crates/{crate} crate; run its browser tests with "
                          f"`wasm-pack test --headless --chrome web/crates/{crate} --lib` "
                          "(python3 scripts/check.py browser runs them)")
    if not path.startswith("web/src/") or not path.endswith(".rs"):
        raise RunnerError(f"not a web/src Rust file: {path}")
    if path_modules and path in path_modules:
        declarer, name = path_modules[path]
        return module_filter(declarer, path_modules) + f"{name}::"
    parts = path[len("web/src/"):-len(".rs")].split("/")
    if parts[-1] in ("mod", "main", "lib"):
        parts.pop()
    return "".join(f"{part}::" for part in parts)


def parent_filter(filter_):
    parts = [p for p in filter_.split("::") if p]
    # Do not widen to a top-level module such as `presentation::`; that is nearly the whole suite.
    return "".join(f"{p}::" for p in parts[:-1]) if len(parts) > 2 else None


def filters_for_files(files, root=ROOT):
    path_modules = path_attr_modules(root)
    filters = sorted({module_filter(f, path_modules) for f in files})
    if "" in filters:
        return [""]
    return [f for f in filters if not any(o != f and f.startswith(o) for o in filters)]


def load_known_failures(path=None):
    try:
        return {entry["test"]: entry.get("reason", "") for entry in json.loads(Path(path or KNOWN_FAILURES).read_text())}
    except FileNotFoundError:
        return {}


def load_test_owners(path=None):
    """Load explicit, reviewed source-to-browser-test mappings; invalid entries fail closed."""
    try:
        mapping = json.loads(Path(path or TEST_OWNERS).read_text())
    except FileNotFoundError:
        return {}
    if not isinstance(mapping, dict) or mapping.get("schema_version") != 1:
        raise RunnerError("WASM test owner map must use schema_version 1")
    sources = mapping.get("sources")
    if not isinstance(sources, dict):
        raise RunnerError("WASM test owner map must contain a sources object")
    normalized = {}
    for source, entry in sources.items():
        if not isinstance(source, str) or not source.startswith("web/src/") or not source.endswith(".rs"):
            raise RunnerError(f"invalid source path in WASM test owner map: {source!r}")
        if not isinstance(entry, dict) or not isinstance(entry.get("tests"), list):
            raise RunnerError(f"owner map entry for {source} must contain a tests list")
        tests = entry["tests"]
        if not tests or any(not isinstance(name, str) or not name for name in tests):
            raise RunnerError(f"owner map entry for {source} must map at least one exact test name")
        if len(tests) != len(set(tests)):
            raise RunnerError(f"owner map entry for {source} contains duplicate test names")
        if not isinstance(entry.get("coverage"), str) or not entry["coverage"].strip():
            raise RunnerError(f"owner map entry for {source} must explain its coverage limits")
        base_sha256 = entry.get("base_sha256")
        current_sha256 = entry.get("current_sha256")
        if (base_sha256 is None) != (current_sha256 is None):
            raise RunnerError(f"owner map entry for {source} must provide both base_sha256 and current_sha256")
        for key, digest in (("base_sha256", base_sha256), ("current_sha256", current_sha256)):
            if digest is not None and (not isinstance(digest, str) or not re.fullmatch(r"[0-9a-f]{64}", digest)):
                raise RunnerError(f"owner map entry for {source} has an invalid {key}")
        normalized[source] = {
            "tests": tests,
            "coverage": entry["coverage"],
            "base_sha256": base_sha256,
            "current_sha256": current_sha256,
        }
    return normalized


def locked_wasm_bindgen_version(root=ROOT):
    lock = (Path(root) / "Cargo.lock").read_text()
    match = re.search(r'name = "wasm-bindgen"\nversion = "([^"]+)"', lock)
    return match.group(1) if match else None


def wasm_bindgen_dir(root=ROOT):
    candidates = [Path(p).parent for p in glob.glob(os.path.expanduser(
        "~/.cache/.wasm-pack/wasm-bindgen-*/wasm-bindgen-test-runner"))]
    if not candidates:
        raise RunnerError("no cached wasm-bindgen-test-runner under ~/.cache/.wasm-pack; run "
                          "`wasm-pack test --headless --chrome web ...` once with network access to populate it")
    version = locked_wasm_bindgen_version(root)
    for directory in sorted(candidates):
        try:
            out = subprocess.run([str(directory / "wasm-bindgen"), "--version"], capture_output=True, text=True).stdout
        except OSError:
            continue
        if version and out.split()[-1:] == [version]:
            return directory
    return sorted(candidates)[-1]


def runner_environment(root=ROOT):
    env = os.environ.copy()
    # Broad mounted batches exceed the runner's small per-invocation default.
    env.setdefault("WASM_BINDGEN_TEST_TIMEOUT", "120")
    driver = env.get("CHROMEDRIVER") or shutil.which("chromedriver")
    if not driver:
        raise RunnerError("chromedriver not found; install it (e.g. `pacman -S chromium`/`apt install chromium-driver`) "
                          "or set CHROMEDRIVER=/path/to/chromedriver")
    if not any(shutil.which(name) for name in CHROME_BINARIES):
        raise RunnerError("Chrome/Chromium not found on PATH; install google-chrome or chromium "
                          "(chromedriver needs a matching browser)")
    if not shutil.which("wasm-pack"):
        raise RunnerError("wasm-pack not found on PATH; install it with `cargo install wasm-pack`")
    env["CHROMEDRIVER"] = driver
    env["PATH"] = f"{wasm_bindgen_dir(root)}{os.pathsep}{env['PATH']}"
    return env


def run_filter(filter_, env, root=ROOT, skip_filters=()):
    override = os.environ.get(RUNNER_ENV)
    command = shlex.split(override) if override else list(WASM_PACK)
    if filter_:
        command.append(filter_)
    for skip_filter in skip_filters:
        command.extend(["--skip", skip_filter])
    result = subprocess.run(command, cwd=root, env=env, text=True, capture_output=True)
    output = result.stdout + "\n" + result.stderr
    outcomes = {}
    for name, status in TEST_LINE.findall(output):
        outcomes.setdefault(name, "FAILED" if status == "FAIL" else status)
    # Keep interrupted invocations distinct from assertion failures so the known-failure
    # allowlist cannot turn an incomplete browser run into a pass.
    for name in INVOKED_LINE.findall(output):
        outcomes.setdefault(name, "INCOMPLETE")
    return result.returncode, outcomes, output


LIST_LINE = re.compile(r"^(\S+): test$", re.M)


def list_wasm_tests(env, root=ROOT):
    override = os.environ.get(RUNNER_ENV)
    command = (shlex.split(override) if override else list(WASM_PACK)) + ["--list"]
    result = subprocess.run(command, cwd=root, env=env, text=True, capture_output=True)
    names = LIST_LINE.findall(result.stdout + "\n" + result.stderr)
    if result.returncode or not names:
        detail = f" (runner exit {result.returncode})" if result.returncode else ""
        raise RunnerError("could not list wasm tests" + detail + ":\n"
                          + "\n".join((result.stdout + result.stderr).strip().splitlines()[-30:]))
    if len(names) != len(set(names)):
        raise DuplicateTestListingError(names)
    return names


def terminal_observations(output):
    observations = [
        {"test": name, "status": _result_status("FAILED" if status == "FAIL" else status)}
        for name, status in TEST_LINE.findall(output)
    ]
    terminal_names = {item["test"] for item in observations}
    observations.extend(
        {"test": name, "status": "incomplete"}
        for name in INVOKED_LINE.findall(output)
        if name not in terminal_names
    )
    return observations


def duplicate_terminal_names(observations):
    names = [item["test"] for item in observations if item["status"] in ("passed", "failed")]
    return sorted(name for name, count in Counter(names).items() if count > 1)


def test_module_filter(name, depth=None):
    parts = name.split("::")[:-1]
    return "".join(f"{part}::" for part in (parts[:depth] if depth else parts)) or name


def active_owner_mapping(source, entry, root):
    """Return a guarded owner only while both reviewed source revisions still match."""
    if entry["base_sha256"] is None:
        return True
    current = Path(root) / source
    try:
        current_hash = hashlib.sha256(current.read_bytes()).hexdigest()
        base = subprocess.run(["git", "-C", str(root), "show", f"HEAD:{source}"],
                              capture_output=True, check=True).stdout
    except (OSError, subprocess.CalledProcessError):
        return False
    base_hash = hashlib.sha256(base).hexdigest()
    return base_hash == entry["base_sha256"] and current_hash == entry["current_sha256"]


def selection_for_listed_files(files, env, root=ROOT, owners_path=None):
    """Resolve files only to their declared wasm modules or exact reviewed owner tests."""
    path_modules = path_attr_modules(root)
    owner_map = load_test_owners(owners_path)
    pairs = sorted({(file.replace("\\", "/"), module_filter(file, path_modules)) for file in files})
    active_owners = {source: owner for source, owner in owner_map.items()
                     if active_owner_mapping(source, owner, root)}
    # A direct broad module filter covers its descendants. An exact owner mapping does not:
    # preserve child requests so independently changed components keep their reviewed tests.
    requests = [pair for pair in pairs if not any(
        other != pair and pair[1].startswith(other[1]) and other[0] not in active_owners
        for other in pairs)]
    names = list_wasm_tests(env, root)
    available = sorted({test_module_filter(name) for name in names})
    selected_modules, selected_owner_tests, unmatched, diagnostics = set(), set(), [], []
    for source, prefix in requests:
        owner = active_owners.get(source)
        if owner:
            absent = sorted(set(owner["tests"]) - set(names))
            if absent:
                raise RunnerError(f"owner map tests for {source} are absent from the WASM list: "
                                  + ", ".join(absent))
            selected_owner_tests.update(owner["tests"])
            diagnostics.append(f"{source} -> owner map ({owner['coverage']}): "
                               + ", ".join(owner["tests"]))
            continue
        inactive_guard = source in owner_map
        matches = [module for module in available if not prefix or module.startswith(prefix)]
        if matches:
            selected_modules.update(matches)
            reason = " (guard mismatch; conservative fallback)" if inactive_guard else ""
            diagnostics.append(f"{source} -> direct module prefix {prefix!r}{reason}: " + ", ".join(matches))
        else:
            unmatched.append(prefix)
            diagnostics.append(f"{source} -> no direct module or reviewed owner mapping; fail closed")
    modules = sorted(module for module in selected_modules
                     if not any(other != module and module.startswith(other) for other in selected_modules))
    # A selected owner test already covered by a direct module is executed once under that module.
    owner_tests = sorted(test for test in selected_owner_tests
                         if not any(test.startswith(module) for module in modules))
    filters = sorted([*modules, *owner_tests])
    expected = {
        filter_: ([name for name in names if name.startswith(filter_)] if filter_ in modules else [filter_])
        for filter_ in filters
    }
    return filters, expected, unmatched, names, diagnostics


def filters_for_listed_files(files, env, root=ROOT):
    """Backward-compatible triple for callers that only need filters and expected outcomes."""
    filters, expected, unmatched, _names, _diagnostics = selection_for_listed_files(files, env, root)
    return filters, expected, unmatched


def all_module_selection(env, root=ROOT, depth=None, isolated=()):
    """One filter per test module (or per `depth` leading path segments): a single unfiltered run
    shares one page, so a crash or leaked state in one module masks or breaks the rest."""
    if depth is not None and depth <= 0:
        raise RunnerError("--depth must be a positive integer")
    names = list_wasm_tests(env, root)
    modules = sorted({test_module_filter(name, depth) for name in names})
    filters = [m for m in modules if not any(o != m and m.startswith(o) for o in modules)]
    isolated = sorted(set(isolated))
    if isolated and depth is None:
        raise RunnerError("--isolate requires --depth so isolated tests have a grouped parent")
    if any(not prefix.endswith("::") for prefix in isolated):
        raise RunnerError("isolated modules must end with '::'")
    for index, prefix in enumerate(isolated):
        if any(other != prefix and (other.startswith(prefix) or prefix.startswith(other))
               for other in isolated[:index]):
            raise RunnerError(f"isolated module filters overlap: {prefix}")
        matches = [name for name in names if name.startswith(prefix)]
        if not matches:
            raise RunnerError(f"isolated module has no listed tests: {prefix}")
        parent = next((module for module in filters if prefix.startswith(module) and prefix != module), None)
        if parent is None:
            raise RunnerError(f"isolated module is not a strict descendant of a depth group: {prefix}")
    expected = {
        module: [name for name in names if name.startswith(module)
                 and not any(name.startswith(prefix) for prefix in isolated)]
        for module in filters
    }
    for prefix in isolated:
        expected[prefix] = [name for name in names if name.startswith(prefix)]
    filters = sorted([*filters, *isolated])
    return filters, expected


@contextmanager
def desktop_webdriver_config(env, root=ROOT):
    """Set a deterministic desktop Chrome viewport unless the caller supplied capabilities."""
    env = dict(env)
    if env.get(WEBDRIVER_CONFIG_ENV):
        yield env
        return
    existing = next((path for path in (Path(root) / "web/webdriver.json", Path(root) / "webdriver.json")
                     if path.is_file()), None)
    if existing:
        env[WEBDRIVER_CONFIG_ENV] = str(existing.resolve())
        yield env
        return
    with TemporaryDirectory(prefix="boardstudio-wasm-webdriver-") as temporary:
        config = Path(temporary) / "webdriver.json"
        config.write_text(json.dumps({"goog:chromeOptions": {"args": ["--window-size=1280,900"]}}))
        env[WEBDRIVER_CONFIG_ENV] = str(config)
        yield env


def run(filters, known, root=ROOT, expected_tests=None, env=None, unmatched_filters=None,
        listed_tests=None, selection=None, result_json_path=None):
    """Return execution summary and logs; optionally write a structured result report."""
    env = env if env is not None else (os.environ.copy() if os.environ.get(RUNNER_ENV) else runner_environment(root))
    outcomes, problems, logs = {}, [], []
    if not filters and not unmatched_filters:
        problems.append("no wasm tests selected; refusing an empty passing run")
    for filter_ in unmatched_filters or []:
        problems.append(f"zero tests executed for filter {filter_ or '<all tests>'}; "
                        "no matching module appeared in the wasm test list")
    filter_reports, observed_terminal_outcomes = [], []
    for index, filter_ in enumerate(filters, 1):
        label = filter_ or "<all tests>"
        print(f"run-wasm-tests: [{index}/{len(filters)}] {label}", file=sys.stderr)
        started = time.monotonic()
        expected_for_filter = set((expected_tests or {}).get(filter_, []))
        skip_filters = sorted(name for name in (listed_tests or [])
                              if filter_ in name and name not in expected_for_filter)
        unsafe_skips = [skip for skip in skip_filters
                        if any(skip in name for name in expected_for_filter)]
        if unsafe_skips:
            problems.append(f"cannot safely exclude substring collision for filter {label}: "
                            + ", ".join(unsafe_skips))
            continue
        if skip_filters:
            code, found, output = run_filter(filter_, env, root, skip_filters=skip_filters)
        else:
            code, found, output = run_filter(filter_, env, root)
        runner_filter = filter_
        if (not found and (expected_tests is None or filter_ not in expected_tests)
                and filter_.count("::") > 2 and parent_filter(filter_)):
            parent = parent_filter(filter_)
            print(f"run-wasm-tests: no tests under {label}; widening to {parent}", file=sys.stderr)
            code, found, output = run_filter(parent, env, root)
            runner_filter = parent
            label = f"{label} (via {parent})"
        observations = terminal_observations(output)
        observed_terminal_outcomes.extend(observations)
        filter_reports.append({
            "filter": filter_,
            "runner_filter": runner_filter,
            "expected_tests": sorted((expected_tests or {}).get(filter_, [])),
            "terminal_outcomes": observations,
            "duration_ms": round((time.monotonic() - started) * 1000),
        })
        if result_json_path:
            log_dir = Path(str(result_json_path) + ".logs")
            try:
                log_dir.mkdir(parents=True, exist_ok=True)
                (log_dir / f"filter-{index:03d}.log").write_text(output)
                filter_reports[-1]["log_path"] = str(log_dir / f"filter-{index:03d}.log")
            except OSError as error:
                problems.append(f"could not write full WASM invocation log: {error}")
        if not found:
            problems.append(f"zero tests executed for filter {label}"
                            + ("" if code == 0 else f" (runner exit {code})")
                            + "; add a #[wasm_bindgen_test] for this module or check the runner output")
            logs.append(f"--- {label} ---\n" + "\n".join(output.strip().splitlines()[-40:]))
        incomplete = sorted(name for name, status in found.items() if status == "INCOMPLETE")
        if incomplete:
            problems.append("incomplete execution; invoked tests did not report a terminal result: "
                            + ", ".join(incomplete))
            logs.append(f"--- {label} ---\n" + "\n".join(output.strip().splitlines()[-40:]))
        elif code != 0 and "FAILED" not in found.values():
            problems.append(f"runner exited {code} for filter {label} without a failing test (build or harness error)")
            logs.append(f"--- {label} ---\n" + "\n".join(output.strip().splitlines()[-40:]))
        failed_here = sorted(name for name, status in found.items() if status == "FAILED")
        if failed_here:
            logs.append(f"--- {label} ---\n" + "\n".join(output.strip().splitlines()[-40:]))
        if expected_tests is not None and filter_ in expected_tests:
            terminal = {name for name, status in found.items() if status in ("ok", "FAILED")}
            missing = sorted(set(expected_tests[filter_]) - terminal)
            unexpected = sorted(set(found) - set(expected_tests[filter_]))
            if missing:
                problems.append(f"listed tests were not completed for filter {label}: " + ", ".join(missing))
                logs.append(f"--- {label} ---\n" + "\n".join(output.strip().splitlines()[-40:]))
            if unexpected:
                problems.append(f"unlisted tests matched filter {label}: " + ", ".join(unexpected))
                logs.append(f"--- {label} ---\n" + "\n".join(output.strip().splitlines()[-40:]))
        for name, status in found.items():
            outcomes.setdefault(name, status)
    duplicate_terminal = duplicate_terminal_names(observed_terminal_outcomes)
    if duplicate_terminal:
        problems.append("duplicate terminal outcomes for tests: " + ", ".join(duplicate_terminal))
    short = lambda name: name.rsplit("::", 1)[-1]
    failed, passing_known = [], []
    for name, status in sorted(outcomes.items()):
        if short(name) in known:
            if status == "ok":
                passing_known.append(name)
        elif status == "FAILED":
            failed.append(name)
    if result_json_path:
        expected = sorted({name for names in (expected_tests or {}).values() for name in names})
        terminal = observed_terminal_outcomes
        terminal_names = {item["test"] for item in terminal if item["status"] in ("passed", "failed")}
        report = {
            "schema_version": 1,
            "selection": selection or {},
            "listed_tests": sorted(listed_tests or []),
            "duplicate_listed_tests": [],
            "expected_tests": expected,
            "terminal_outcomes": terminal,
            "duplicate_terminal_outcomes": duplicate_terminal,
            "filters": filter_reports,
            "passed_count": sum(item["status"] == "passed" for item in terminal),
            "failed_count": sum(item["status"] == "failed" for item in terminal),
            "incomplete_count": sum(item["status"] == "incomplete" for item in terminal),
            "failed_known_exclusions": sorted(name for name, status in outcomes.items()
                                               if status == "FAILED" and short(name) in known),
            "problems": problems,
            "complete": bool(expected) and set(expected).issubset(terminal_names) and not problems,
        }
        try:
            result_path = Path(result_json_path)
            result_path.parent.mkdir(parents=True, exist_ok=True)
            result_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
        except OSError as error:
            problems.append(f"could not write structured WASM result {result_json_path}: {error}")
    return len(outcomes), failed, passing_known, problems, logs


def _result_status(status):
    return {"ok": "passed", "FAILED": "failed", "INCOMPLETE": "incomplete"}.get(status, "incomplete")


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--files", nargs="+", metavar="FILE")
    group.add_argument("--all", action="store_true")
    parser.add_argument("--depth", type=int, default=None, metavar="N",
                        help="with --all: group tests by their first N module segments instead of one run per module "
                             "(fewer, faster runs; less isolation)")
    parser.add_argument("--isolate", action="append", default=[], metavar="MODULE",
                        help="with --all --depth: run this module prefix separately, e.g. presentation::panels::")
    parser.add_argument("--root", default=str(ROOT))
    parser.add_argument("--result-json", metavar="PATH",
                        help="write the exact listed, selected and terminal outcomes as JSON")
    args = parser.parse_args(argv)
    root = Path(args.root).resolve()
    try:
        known = load_known_failures()
        custom_runner = bool(os.environ.get(RUNNER_ENV))
        env = os.environ.copy() if custom_runner else runner_environment(root)
        browser_setup = not custom_runner
        with (desktop_webdriver_config(env, root) if browser_setup else nullcontext(env)) as env:
            if args.files:
                filters, expected_tests, unmatched_filters, listed_tests, diagnostics = \
                    selection_for_listed_files(args.files, env, root)
                selection = {"mode": "files", "requested_files": args.files,
                             "resolved_filters": filters, "diagnostics": diagnostics}
            else:
                filters, expected_tests = all_module_selection(env, root, args.depth, args.isolate)
                listed_tests = sorted({name for names in expected_tests.values() for name in names})
                unmatched_filters = []
                diagnostics = [f"--all -> {filter_ or '<all tests>'}" for filter_ in filters]
                selection = {"mode": "all", "depth": args.depth, "isolated": args.isolate,
                             "resolved_filters": filters, "diagnostics": diagnostics}
            for diagnostic in diagnostics:
                print(f"run-wasm-tests: selection: {diagnostic}", file=sys.stderr)
            executed, failed, passing_known, problems, logs = run(
                filters, known, root, expected_tests, env, unmatched_filters,
                listed_tests, selection, args.result_json)
    except RunnerError as error:
        print(f"run-wasm-tests: {error}", file=sys.stderr)
        if args.result_json:
            report = {
                "schema_version": 1,
                "selection": {"mode": "files" if args.files else "all",
                              "requested_files": args.files, "depth": args.depth},
                "listed_tests": [],
                "duplicate_listed_tests": [],
                "expected_tests": [],
                "terminal_outcomes": [],
                "duplicate_terminal_outcomes": [],
                "filters": [],
                "passed_count": 0,
                "failed_count": 0,
                "incomplete_count": 0,
                "failed_known_exclusions": [],
                "problems": [str(error)],
                "complete": False,
            }
            if hasattr(error, "listed_tests"):
                report["listed_tests"] = error.listed_tests
                report["duplicate_listed_tests"] = error.duplicate_names
            try:
                result_path = Path(args.result_json)
                result_path.parent.mkdir(parents=True, exist_ok=True)
                result_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
            except OSError as write_error:
                print(f"run-wasm-tests: could not write structured WASM result "
                      f"{args.result_json}: {write_error}", file=sys.stderr)
        return 2
    for name in passing_known:
        print(f"run-wasm-tests: note: known failure now passes, remove it from "
              f"scripts/wasm-known-failures.json: {name}")
    print(f"run-wasm-tests: executed {executed}, failed {len(failed)} "
          f"(filters: {', '.join(f or '<all>' for f in filters)})")
    for name in failed:
        print(f"  FAILED {name}")
    for problem in problems:
        print(f"run-wasm-tests: {problem}", file=sys.stderr)
    if failed or problems:
        for log in logs:
            print(log, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
