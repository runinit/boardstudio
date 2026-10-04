#!/usr/bin/env python3
"""Run the headless-Chrome `#[wasm_bindgen_test]` suite (the tests native `cargo test` never compiles).

  run-wasm-tests.py --files web/src/presentation/foo.rs ...   run only the modules those files define
  run-wasm-tests.py --all                                     run everything

Known failures (scripts/wasm-known-failures.json, tracked as RF-033) are executed but do not fail the
run; a known failure that now passes is reported so the entry can be removed.

wasm-bindgen-test-runner accepts a single FILTER, so each module filter is one runner invocation.
Override the runner with BOARDSTUDIO_WASM_TEST_COMMAND (shell-split; the filter, if any, is appended).
"""

import argparse
from contextlib import contextmanager, nullcontext
import glob
import importlib.util
import json
import os
from pathlib import Path
import re
import shlex
import shutil
import subprocess
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from tempfile import TemporaryDirectory
from threading import Thread
from urllib.parse import urlsplit

ROOT = Path(__file__).resolve().parent.parent
KNOWN_FAILURES = Path(__file__).with_name("wasm-known-failures.json")
RUNNER_ENV = "BOARDSTUDIO_WASM_TEST_COMMAND"
WASM_PACK = ["wasm-pack", "test", "--headless", "--chrome", "--mode", "no-install", "web",
             "--no-default-features", "--features", "page", "--bin", "boardstudio-web", "--"]
CHROME_BINARIES = ("google-chrome-stable", "google-chrome", "chromium", "chromium-browser", "chrome")
GENERATOR_SOURCE_FILES = (
    "web/src/bundled_models.rs",
    "web/src/presentation/parts/catalogue.rs",
)
GENERATOR_MODULE_URL_ENV = "BOARDSTUDIO_TEST_LAYOUT_GENERATOR_MODULE_URL"
WEBDRIVER_CONFIG_ENV = "WASM_BINDGEN_TEST_WEBDRIVER_JSON"
GENERATOR_ASSETS = (
    "/layout-generators/src/index.js",
    "/layout-generators/generated/catalogue.mjs",
)
TEST_LINE = re.compile(r"^test (\S+) \.\.\. (ok|FAILED|FAIL)\b", re.M)
INVOKED_LINE = re.compile(r"^\s*Invoking test: (\S+)", re.M)
class RunnerError(RuntimeError):
    pass


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


def locked_wasm_bindgen_version(root=ROOT):
    lock = (Path(root) / "web/Cargo.lock").read_text()
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


def run_filter(filter_, env, root=ROOT):
    override = os.environ.get(RUNNER_ENV)
    command = shlex.split(override) if override else list(WASM_PACK)
    if filter_:
        command.append(filter_)
    result = subprocess.run(command, cwd=root, env=env, text=True, capture_output=True)
    output = result.stdout + "\n" + result.stderr
    outcomes = {name: status for name, status in TEST_LINE.findall(output)}
    # Keep interrupted invocations distinct from assertion failures so the known-failure
    # allowlist cannot turn an incomplete browser run into a pass.
    for name in INVOKED_LINE.findall(output):
        outcomes.setdefault(name, "INCOMPLETE")
    outcomes = {name: ("FAILED" if status == "FAIL" else status) for name, status in outcomes.items()}
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
    return names


def test_module_filter(name, depth=None):
    parts = name.split("::")[:-1]
    return "".join(f"{part}::" for part in (parts[:depth] if depth else parts)) or name


def filters_for_listed_files(files, env, root=ROOT):
    """Resolve source prefixes to listed test modules without substring overreach."""
    requested = filters_for_files(files, root)
    names = list_wasm_tests(env, root)
    available = sorted({test_module_filter(name) for name in names})
    selected, unmatched = set(), []
    for prefix in requested:
        matches = [module for module in available if not prefix or module.startswith(prefix)]
        if not matches:
            parent = parent_filter(prefix)
            if parent:
                matches = [module for module in available if module.startswith(parent)]
        if matches:
            selected.update(matches)
        else:
            unmatched.append(prefix)
    filters = sorted(module for module in selected
                     if not any(other != module and module.startswith(other) for other in selected))
    expected = {
        module: [name for name in names if name.startswith(module)]
        for module in filters if module in available
    }
    return filters, expected, unmatched


def all_module_selection(env, root=ROOT, depth=None):
    """One filter per test module (or per `depth` leading path segments): a single unfiltered run
    shares one page, so a crash or leaked state in one module masks or breaks the rest."""
    names = list_wasm_tests(env, root)
    modules = sorted({test_module_filter(name, depth) for name in names})
    filters = [m for m in modules if not any(o != m and m.startswith(o) for o in modules)]
    expected = {module: [name for name in names if name.startswith(module)] for module in filters}
    return filters, expected


def selected_generator_sources(files, root=ROOT):
    requested = filters_for_files(files, root)
    generator = filters_for_files(GENERATOR_SOURCE_FILES, root)
    return any(not request or not source or request.startswith(source) or source.startswith(request)
               for request in requested for source in generator)


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


@contextmanager
def packaged_generator_harness(root=ROOT, env=None):
    """Build and serve a fresh, allow-listed generator package for one CLI run."""
    root = Path(root).resolve()
    env = dict(env or os.environ)
    with TemporaryDirectory(prefix="boardstudio-wasm-generators-") as temporary:
        assets = Path(temporary) / "assets"
        try:
            build = subprocess.run(
                ["node", str(Path(root) / "scripts/web/build-layout-generators.mjs"), str(assets)],
                cwd=root, text=True, capture_output=True,
            )
        except OSError as error:
            raise RunnerError(f"could not start layout-generator asset build: {error}") from error
        if build.returncode:
            raise RunnerError("could not build packaged layout generators:\n" +
                              "\n".join((build.stdout + build.stderr).strip().splitlines()[-30:]))

        class Handler(BaseHTTPRequestHandler):
            def do_GET(self):
                path = urlsplit(self.path).path
                if path not in GENERATOR_ASSETS:
                    self.send_error(404)
                    return
                try:
                    body = (assets / path.lstrip("/")).read_bytes()
                except OSError:
                    self.send_error(404)
                    return
                self.send_response(200)
                self.send_header("Access-Control-Allow-Origin", "*")
                self.send_header("Content-Type", "text/javascript")
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)

            def log_message(self, *_args):
                pass

        server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        server_thread = Thread(target=server.serve_forever, daemon=True)
        server_thread.start()
        address, port = server.server_address
        env[GENERATOR_MODULE_URL_ENV] = f"http://{address}:{port}{GENERATOR_ASSETS[0]}"
        try:
            yield env
        finally:
            server.shutdown()
            server.server_close()
            server_thread.join()


def run(filters, known, root=ROOT, expected_tests=None, env=None, unmatched_filters=None):
    """Returns (executed, failed_names, passing_known_names, problems)."""
    env = env if env is not None else (os.environ.copy() if os.environ.get(RUNNER_ENV) else runner_environment(root))
    outcomes, problems, logs = {}, [], []
    if not filters and not unmatched_filters:
        problems.append("no wasm tests selected; refusing an empty passing run")
    for filter_ in unmatched_filters or []:
        problems.append(f"zero tests executed for filter {filter_ or '<all tests>'}; "
                        "no matching module appeared in the wasm test list")
    for index, filter_ in enumerate(filters, 1):
        label = filter_ or "<all tests>"
        print(f"run-wasm-tests: [{index}/{len(filters)}] {label}", file=sys.stderr)
        code, found, output = run_filter(filter_, env, root)
        if (not found and (expected_tests is None or filter_ not in expected_tests)
                and filter_.count("::") > 2 and parent_filter(filter_)):
            parent = parent_filter(filter_)
            print(f"run-wasm-tests: no tests under {label}; widening to {parent}", file=sys.stderr)
            code, found, output = run_filter(parent, env, root)
            label = f"{label} (via {parent})"
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
        outcomes.update(found)
    short = lambda name: name.rsplit("::", 1)[-1]
    failed, passing_known = [], []
    for name, status in sorted(outcomes.items()):
        if short(name) in known:
            if status == "ok":
                passing_known.append(name)
        elif status == "FAILED":
            failed.append(name)
    return len(outcomes), failed, passing_known, problems, logs


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--files", nargs="+", metavar="FILE")
    group.add_argument("--all", action="store_true")
    parser.add_argument("--depth", type=int, default=None, metavar="N",
                        help="with --all: group tests by their first N module segments instead of one run per module "
                             "(fewer, faster runs; less isolation)")
    parser.add_argument("--root", default=str(ROOT))
    args = parser.parse_args(argv)
    root = Path(args.root).resolve()
    try:
        known = load_known_failures()
        custom_runner = bool(os.environ.get(RUNNER_ENV))
        env = os.environ.copy() if custom_runner else runner_environment(root)
        needs_generator = not custom_runner and (
            args.all or selected_generator_sources(args.files, root))
        browser_setup = not custom_runner
        with (desktop_webdriver_config(env, root) if browser_setup else nullcontext(env)) as env:
            with (packaged_generator_harness(root, env) if needs_generator else nullcontext(env)) as env:
                if args.files:
                    filters, expected_tests, unmatched_filters = filters_for_listed_files(args.files, env, root)
                else:
                    filters, expected_tests = all_module_selection(env, root, args.depth)
                    unmatched_filters = []
                executed, failed, passing_known, problems, logs = run(
                    filters, known, root, expected_tests, env, unmatched_filters)
    except RunnerError as error:
        print(f"run-wasm-tests: {error}", file=sys.stderr)
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
