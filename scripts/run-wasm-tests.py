#!/usr/bin/env python3
"""Run the headless-Chrome `#[wasm_bindgen_test]` suite (the tests native `cargo test` never compiles).

  run-wasm-tests.py --files web/src/presentation/foo.rs ...   run only the modules those files define
  run-wasm-tests.py --all                                     run everything

Known failures (scripts/wasm-known-failures.json) are executed but do not fail the
run; a known failure that now passes is reported so the entry can be removed.

wasm-bindgen-test-runner accepts a single FILTER, so each module filter is one runner invocation.
Override the runner with BOARDSTUDIO_WASM_TEST_COMMAND (shell-split; the filter, if any, is appended).
Set BOARDSTUDIO_WASM_BINDGEN_DIR to select the locked wasm-bindgen tool pair.
"""

import argparse
from collections import Counter
from concurrent.futures import ThreadPoolExecutor
from contextlib import contextmanager, nullcontext
import importlib.util
import json
import os
from pathlib import Path
import re
import shlex
import shutil
import subprocess
import browser_tools
import wasm_bindgen_tools
import sys
from tempfile import TemporaryDirectory

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(Path(__file__).resolve().parent))
import cargo_json  # noqa: E402
from tailwind_tools import TEST_CSS_READY, check_jobs, test_css_command  # noqa: E402
KNOWN_FAILURES = Path(__file__).with_name("wasm-known-failures.json")
RUNNER_ENV = "BOARDSTUDIO_WASM_TEST_COMMAND"
# Share the profile and target directory with the browser crate tests. Test-support and
# dev-dependency features give this build a different graph from the packaged page.
PAGE_BUILD = ["cargo", "test", "--no-run", "--target", "wasm32-unknown-unknown", "--locked",
              "--profile", "wasm-check", "--no-default-features", "--features", "page,boardstudio-web-host/test-support",
              "-p", "boardstudio-web", "--bin", "boardstudio-web", "--message-format=json"]
PAGE_ARTIFACT = "BOARDSTUDIO_PAGE_TEST_ARTIFACT"
CHROME_BIN_ENV = "CHROME_BIN"
WEBDRIVER_CONFIG_ENV = "WASM_BINDGEN_TEST_WEBDRIVER_JSON"
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


def wasm_bindgen_dir(root=ROOT):
    try:
        if wasm_bindgen_tools.DIRECTORY_ENV in os.environ:
            env = wasm_bindgen_tools.configure(os.environ, root)
            return Path(env[wasm_bindgen_tools.DIRECTORY_ENV])
        version = wasm_bindgen_tools.locked_version(root)
        cache = Path(os.environ.get("XDG_CACHE_HOME") or Path.home() / ".cache")
        if patched := wasm_bindgen_tools.local_patched_pair(cache, root, version):
            return patched
    except (OSError, ValueError) as error:
        raise RunnerError(str(error)) from error
    cache /= ".wasm-pack"
    candidates = [path.parent for path in sorted(cache.glob(
        f"wasm-bindgen-*/{wasm_bindgen_tools.TEST_RUNNER}"))]
    if bindgen := shutil.which("wasm-bindgen"):
        candidates.insert(0, Path(bindgen).absolute().parent)
    for directory in candidates:
        try:
            wasm_bindgen_tools.validate_pair(directory, version)
        except (OSError, subprocess.SubprocessError, ValueError):
            continue
        return directory
    raise RunnerError(f"no wasm-bindgen pair at locked version {version} on PATH or in {cache}; "
                      f"run `cargo install wasm-bindgen-cli --locked --version {version}` "
                      "or set BOARDSTUDIO_WASM_BINDGEN_DIR to a matching pair")


def runner_environment(root=ROOT):
    env = os.environ.copy()
    env.setdefault("WASM_BINDGEN_TEST_TIMEOUT", "120")
    env.setdefault("CARGO_PROFILE_DEV_DEBUG", "false")
    try:
        env = browser_tools.browser_env(env, CHROMEDRIVER=browser_tools.locate_chromedriver(env))
    except browser_tools.MissingBrowserTool as error:
        raise RunnerError(str(error)) from error
    directory = str(wasm_bindgen_dir(root))
    env[wasm_bindgen_tools.DIRECTORY_ENV] = directory
    env["PATH"] = directory + os.pathsep + env.get("PATH", "")
    print(f"run-wasm-tests: Chrome {env['CHROME_BIN']}; driver {env['CHROMEDRIVER']}; "
          f"wasm-bindgen {directory}", file=sys.stderr)
    return env


def build_page_test(root, env):
    fd = browser_tools._inherited_build_lock()
    if env.get(TEST_CSS_READY) != "1" and (root / "web/tailwind.css").is_file():
        subprocess.run(test_css_command(), cwd=root / "web", env=env, check=True, stdout=sys.stderr,
                       pass_fds=() if fd is None else (fd,))
    build_env = wasm_bindgen_tools.test_build_env(env, root)
    try:
        result = subprocess.run(PAGE_BUILD, cwd=root, env=build_env, text=True,
                                stdout=subprocess.PIPE, pass_fds=() if fd is None else (fd,))
    except OSError as error:
        raise RunnerError(f"could not build the page test: {error}") from error
    artifacts = [root / executable for message in cargo_json.messages(result.stdout, sys.stderr)
                 if (executable := cargo_json.wasm_executable(message, kind="bin", name="boardstudio-web", test=True))
                 and Path(message["manifest_path"]) == root / "web/Cargo.toml"]
    if result.returncode:
        raise RunnerError(f"page test build exited {result.returncode}")
    if len(artifacts) != 1 or not artifacts[0].is_file():
        raise RunnerError("expected one existing page test wasm artifact from Cargo")
    return artifacts[0]


def run_page_runner(args, env, root):
    override = os.environ.get(RUNNER_ENV)
    if override:
        command = shlex.split(override)
        cwd = root
    else:
        command = [wasm_bindgen_tools.TEST_RUNNER, env[PAGE_ARTIFACT]]
        # wasm-pack launched from the page package. The runner serves its relative assets here.
        cwd = Path(root) / "web"
        env = dict(env, RAYON_NUM_THREADS="1")
    fd = browser_tools._inherited_build_lock()
    return subprocess.run([*command, *args], cwd=cwd, env=env, text=True, capture_output=True,
                          pass_fds=() if fd is None else (fd,))


@contextmanager
def desktop_webdriver_config(env, root=ROOT):
    env = env.copy()
    supplied = env.get(WEBDRIVER_CONFIG_ENV)
    root = Path(root)
    path = Path(supplied) if supplied else next(
        (path for path in (root / "web/webdriver.json", root / "webdriver.json") if path.is_file()), None)
    try:
        config = json.loads(path.read_text()) if path else {}
        options = config.setdefault("goog:chromeOptions", {})
        chrome = env[CHROME_BIN_ENV]
        if supplied and options.get("binary") and Path(options["binary"]).resolve() != Path(chrome).resolve():
            raise RunnerError("caller WebDriver browser conflicts with selected Chrome")
        options["binary"] = chrome
        args = options.setdefault("args", [])
        if not supplied and not any(arg.startswith("--window-size=") for arg in args):
            args.append("--window-size=1280,900")
        config.setdefault("pageLoadStrategy", "eager")
    except (OSError, ValueError, AttributeError, TypeError) as error:
        raise RunnerError(f"invalid WebDriver config: {error}") from error
    with TemporaryDirectory(prefix="bs-wasm-", dir=browser_tools.shallow_browser_tmpdir()) as temporary:
        config_path = Path(temporary) / "webdriver.json"
        config_path.write_text(json.dumps(config))
        env[WEBDRIVER_CONFIG_ENV] = str(config_path)
        yield env


def list_wasm_tests(env, root):
    result = run_page_runner(["--list"], env, root)
    output = result.stdout + result.stderr
    names = re.findall(r"^(\S+): test$", output, re.M)
    if result.returncode or not names:
        raise RunnerError(f"could not list WASM tests (exit {result.returncode}):\n{output}")
    if len(set(names)) != len(names):
        raise RunnerError("duplicate test names in WASM listing")
    return sorted(names)


def select_tests(names, files, depth, isolated, root):
    direct = set(names)
    if files:
        direct.clear()
        paths = path_attr_modules(root)
        for file in (path.replace("\\", "/") for path in files):
            prefix = module_filter(file, paths)
            selected = {name for name in names if name.startswith(prefix)}
            if not selected:
                raise RunnerError(f"no listed WASM tests for {file}")
            direct.update(selected)
    if any(not prefix.endswith("::") for prefix in isolated):
        raise RunnerError("isolated module filters must end in '::'")
    for prefix in isolated:
        if any(other != prefix and other.startswith(prefix) for other in isolated):
            raise RunnerError(f"overlapping isolated module filter {prefix}")
        if not files and not any(name.startswith(prefix) for name in names):
            raise RunnerError(f"no listed WASM tests for {prefix}")
    groups = {}
    for name in sorted(direct):
        parts = name.split("::")[:-1]
        prefix = "".join(f"{part}::" for part in parts[:depth]) or name
        prefix = next((isolate for isolate in isolated if name.startswith(isolate)), prefix)
        groups.setdefault(prefix, []).append(name)
    if files and depth:
        # Match the full gate's group boundaries.
        for prefix in groups:
            groups[prefix] = [name for name in names if name.startswith(prefix)
                              and not any(other != prefix and name.startswith(other)
                                          for other in groups if other in isolated)]
    return groups


def run_group(prefix, expected, names, known, env, root):
    skipped = [name for name in names if prefix in name and name not in expected]
    if any(skip in name for skip in skipped for name in expected):
        return {}, [f"{prefix}: ambiguous substring exclusions"]
    result = run_page_runner([prefix, *(arg for skip in skipped for arg in ("--skip", skip))], env, root)
    output = result.stdout + "\n" + result.stderr
    terminal = TEST_LINE.findall(output)
    outcomes = {name: ("passed" if status == "ok" else "failed") for name, status in terminal}
    outcomes.update({name: "incomplete" for name in INVOKED_LINE.findall(output) if name not in outcomes})
    counts = Counter(name for name, _ in terminal)
    problems = []
    if missing := set(expected) - counts.keys():
        problems.append(f"{prefix}: missing terminal results for {', '.join(sorted(missing))}")
    if unexpected := outcomes.keys() - set(expected):
        problems.append(f"{prefix}: unexpected tests {', '.join(sorted(unexpected))}")
    if duplicates := sorted(name for name, count in counts.items() if count > 1):
        problems.append(f"{prefix}: duplicate results for {', '.join(duplicates)}")
    failed = {name for name, status in outcomes.items() if status == "failed"}
    if unexpected_failures := {name for name in failed if name.rsplit("::", 1)[-1] not in known}:
        problems.append(f"{prefix}: failed {', '.join(sorted(unexpected_failures))}")
    if result.returncode and not failed:
        problems.append(f"{prefix}: runner exited {result.returncode}")
    if problems:
        print(output, file=sys.stderr)
    return outcomes, problems


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--all", action="store_true")
    group.add_argument("--files", nargs="+")
    group.add_argument("--build-only", action="store_true")
    parser.add_argument("--depth", type=int)
    parser.add_argument("--isolate", action="append", default=[])
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--test-binary", type=Path)
    parser.add_argument("--result-json", type=Path)
    args = parser.parse_args(argv)
    if args.depth is not None and args.depth <= 0:
        parser.error("--depth must be positive")
    if args.test_binary and (args.build_only or not args.test_binary.is_file()):
        parser.error("--test-binary requires an existing file and --all or --files")

    root = args.root.resolve()
    names, groups, outcomes, problems = [], {}, {}, []
    exit_code = 0
    try:
        browser_tools._inherited_build_lock()
        if args.build_only:
            print(build_page_test(root, os.environ))
            return 0
        custom = bool(os.environ.get(RUNNER_ENV))
        env = wasm_bindgen_tools.configure(os.environ.copy(), root) if custom else runner_environment(root)
        if not custom:
            env[PAGE_ARTIFACT] = str(args.test_binary.resolve() if args.test_binary else build_page_test(root, env))
            env["WASM_BINDGEN_TEST_ONLY_WEB"] = "1"
        known = {entry["test"] for entry in json.loads(KNOWN_FAILURES.read_text())}
        with (nullcontext(env) if custom else desktop_webdriver_config(env, root)) as env:
            names = list_wasm_tests(env, root)
            groups = select_tests(names, args.files, args.depth, sorted(set(args.isolate)), root)
            workers = 1 if os.environ.get("GITHUB_ACTIONS") == "true" else check_jobs()
            with ThreadPoolExecutor(max_workers=min(workers, len(groups))) as pool:
                futures = [pool.submit(run_group, prefix, expected, names, known, env, root)
                           for prefix, expected in sorted(groups.items(), key=lambda item: -len(item[1]))]
                for future in futures:
                    observed, errors = future.result()
                    problems.extend(errors)
                    if duplicate := outcomes.keys() & observed.keys():
                        problems.append(f"tests ran in multiple groups: {', '.join(sorted(duplicate))}")
                    outcomes.update(observed)
        exit_code = int(bool(problems))
    except (OSError, ValueError, RuntimeError, subprocess.CalledProcessError) as error:
        problems.append(str(error))
        exit_code = 2
    for problem in problems:
        print(f"run-wasm-tests: {problem}", file=sys.stderr)
    for name in sorted(name for name, status in outcomes.items()
                       if status == "passed" and name.rsplit("::", 1)[-1] in known):
        print(f"run-wasm-tests: known failure now passes: {name}")
    print(f"run-wasm-tests: {len(outcomes)} observed across {len(groups)} groups")
    if args.result_json:
        args.result_json.parent.mkdir(parents=True, exist_ok=True)
        args.result_json.write_text(json.dumps({
            "listed_tests": names, "expected_tests": sorted({name for group in groups.values() for name in group}),
            "terminal_outcomes": [{"test": name, "status": status} for name, status in sorted(outcomes.items())],
            "problems": problems, "complete": exit_code == 0,
        }, indent=2) + "\n")
    return exit_code


if __name__ == "__main__":
    raise SystemExit(main())
