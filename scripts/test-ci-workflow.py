#!/usr/bin/env python3
"""Hermetic checks of the CI cache policy in .github/workflows/check.yaml.

Text-based on purpose: the CI Python has no YAML parser, and the existing workflow tests
(test-bump-chrome-for-testing.py) read the file the same way.
"""
from __future__ import annotations

import os
import re
import runpy
import shutil
import subprocess
import tarfile
import tempfile
import textwrap
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
WORKFLOW = (REPO / ".github" / "workflows" / "check.yaml").read_text()
CAD_BUILD = (REPO / "cad" / "scripts" / "cad_build.py").read_text()
TOOLCHAIN = (REPO / "rust-toolchain.toml").read_text()
CACHE_ACTION = "Swatinem/rust-cache@"
MAIN_REPO = "runinit/boardstudio"


def save_if(step: str, **context: str) -> bool:
    """Evaluate a step's `save-if` the way Actions would for the given event/ref/head repo."""
    raw = re.search(r"^ +save-if: (.*)$", step, re.M).group(1)
    if raw == "false":
        return False
    expr = raw.removeprefix("${{").removesuffix("}}").strip()
    # Longest names first: `github.ref` must not eat a prefix of `github.repository`.
    for name, var in (("github.event.pull_request.head.repo.full_name", "head_repo"),
                      ("github.event.pull_request.user.login", "author"),
                      ("github.event_name", "event"), ("github.repository", "repo"),
                      ("github.actor", "actor"), ("github.ref", "ref")):
        expr = expr.replace(name, var)
    expr = expr.replace("&&", " and ").replace("||", " or ")
    assert "github." not in expr, f"unmodelled context in save-if: {raw}"
    return bool(eval(expr, {"__builtins__": {}}, context))  # noqa: S307 - our own workflow text, no builtins


def pull_request(head_repo: str, actor: str = "runinit", author: str = "runinit") -> dict[str, str]:
    return {"event": "pull_request", "ref": "refs/pull/9/merge", "head_repo": head_repo, "repo": MAIN_REPO, "actor": actor, "author": author}


def push(ref: str) -> dict[str, str]:
    return {"event": "push", "ref": ref, "head_repo": "", "repo": MAIN_REPO}


def job(name: str) -> str:
    match = re.search(rf"^  {re.escape(name)}:\n(.*?)(?=^  \S|\Z)", WORKFLOW, re.M | re.S)
    assert match, f"job {name} missing"
    return match.group(1)


def cache_steps(text: str) -> list[str]:
    parts = text.split("- uses: " + CACHE_ACTION)[1:]
    return [re.split(r"\n      - ", part, maxsplit=1)[0] for part in parts]


class CachePolicyTests(unittest.TestCase):
    def test_rust_cache_save_matrix(self):
        # GitHub scopes a pull_request cache to refs/pull/N/merge, so a same-repo pull request may save
        # (its own later pushes restore it); trusted branches keep saving; nothing else may.
        for name in ("native", "web"):
            [step] = cache_steps(job(name))
            for label, ctx, expected in (
                ("push main", push("refs/heads/main"), True),
                ("push dev", push("refs/heads/dev"), True),
                ("same-repo pull request", pull_request(MAIN_REPO), True),
                ("dependabot pull request", pull_request(MAIN_REPO, "dependabot[bot]"), False),
                ("dependabot PR triggered by a human", pull_request(MAIN_REPO, author="dependabot[bot]"), False),
                ("fork pull request", pull_request("someone/boardstudio"), False),
                ("push to another branch", push("refs/heads/stack/02-split-board"), False),
                ("dispatch on a feature branch", {**push("refs/heads/feature"), "event": "workflow_dispatch"}, False),
            ):
                self.assertEqual(save_if(step, **ctx), expected, f"{name}: {label}")

    def test_release_build_never_saves(self):
        [step] = cache_steps(job("release-build"))
        for ctx in (push("refs/heads/main"), {**push("refs/heads/main"), "event": "workflow_dispatch"}, pull_request(MAIN_REPO)):
            self.assertFalse(save_if(step, **ctx), "release-build restores the web cache; only `web` saves it")

    def test_cache_on_failure_stays_off(self):
        # post-if is `success() || CACHE_ON_FAILURE == 'true'`, which also runs on cancellation. With
        # cancel-in-progress, a superseded first run would save a partial target under a key that every
        # later run exact-hits (and therefore never overwrites).
        for name in ("native", "web", "release-build"):
            for step in cache_steps(job(name)):
                self.assertNotRegex(step, r"cache-on-failure:\s*['\"]?true", name)

    def test_no_trigger_can_write_the_default_branch_cache_scope(self):
        triggers = set(re.findall(r"^  (\w+):", WORKFLOW.split("\npermissions:\n")[0].split("\non:\n", 1)[1], re.M))
        self.assertEqual(triggers, {"pull_request", "push", "workflow_dispatch"},
                         "pull_request_target/workflow_run run with base-scope cache access")
        top = WORKFLOW.split("\nconcurrency:\n")[0].split("\npermissions:\n", 1)[1]
        self.assertEqual(top.strip(), "contents: read")

    def test_toolchain_is_installed_before_the_cache_hashes_rustc(self):
        # RUSTUP_HOME starts empty on every runner; `rustup target add` only worked there through a
        # deprecated auto-install (rustup#4836).
        self.assertNotIn("rustup target add", WORKFLOW)
        for name in ("native", "web", "release-build"):
            text = job(name)
            self.assertIn("      - run: rustup toolchain install\n", text, name)
            self.assertLess(text.index("- run: rustup toolchain install"), text.index("- uses: " + CACHE_ACTION), name)

    def test_tool_caches_are_version_keyed_and_release_only_restores(self):
        for name in ("web", "release-build"):
            for key in re.findall(r"^ +key: (.*)$", job(name), re.M):
                self.assertNotIn("github.", key, "tool cache keys are content versions, not refs")
        self.assertNotIn("actions/cache@", job("release-build"))
        # The wasm-pack cache key must follow the wasm-pack version web-tools installs.
        [key_version] = re.findall(r"key: wasm-pack-(\d+\.\d+\.\d+)-", job("web"))
        self.assertIn(f"wasm-pack/releases/download/v{key_version}/", (REPO / ".github" / "actions" / "web-tools" / "action.yml").read_text())

    def test_cache_key_sees_only_the_pinned_toolchain(self):
        env = WORKFLOW.split("\njobs:\n")[0].split("\nenv:\n", 1)[1]
        self.assertRegex(env, re.compile(r"^  RUSTUP_HOME: /home/runner/\.\S+$", re.M))
        self.assertIn('channel = "', TOOLCHAIN, "RUSTUP_HOME holds only what rust-toolchain.toml installs")
        for name in ("native", "web", "release-build"):
            [step] = cache_steps(job(name))
            self.assertRegex(step, re.compile(r"^ +env-vars: ImageOS$", re.M), "image family must stay in the key")

    def test_web_and_release_build_share_one_key(self):
        [step] = cache_steps(job("web"))
        self.assertIn("shared-key: web\n", step)
        self.assertIn("shared-key: web\n", cache_steps(job("release-build"))[0])

    def test_single_web_job_gates_release(self):
        web = job("web")
        self.assertNotIn("matrix.", web)
        self.assertNotIn("name: web-", web)
        self.assertIn("run: python3 scripts/check.py build browser\n        env:\n"
                      "          BOARDSTUDIO_WASM_BINDGEN_DIR: ${{ runner.temp }}/wasm-bindgen-patched\n"
                      '          CARGO_PROFILE_DEV_DEBUG: "false"', web)
        self.assertIn("name: browser-failure\n", web)
        self.assertIn("name: drag-browser-receipts\n", web)
        self.assertIn("needs: [native, web]", job("release-build"))
        self.assertNotIn("web-result", WORKFLOW)

    def test_native_caches_every_cad_target_dir(self):
        [step] = cache_steps(job("native"))
        cached = {(REPO / workspace / target).resolve() for workspace, target in
                  re.findall(r"^ +(\S+) -> (\S+)$", step, re.M)}
        targets = re.search(r'^NATIVE_TEST_TARGETS = \((.*)\)$', CAD_BUILD, re.M)[1]
        built = {REPO / "cad" / target for target in re.findall(r'"([^"]*target)"', targets)}
        self.assertIn(REPO / "cad/target", built)
        self.assertLessEqual(built, cached, "cad_build.py builds into a target dir the CI cache skips")

    def test_web_build_jobs_cache_cad_wasm_from_the_merged_workspace(self):
        [wasm_target] = re.findall(r'^WASM_BUILD_TARGET = "([^"]+)"$', CAD_BUILD, re.M)
        for name in ("web", "release-build"):
            [step] = cache_steps(job(name))
            [target] = re.findall(r"^ +cad -> (\S+)$", step, re.M)
            self.assertEqual((REPO / "cad" / target).resolve(), REPO / "cad" / wasm_target)
            self.assertNotIn("cad/wasm ->", step)


class ConcurrencyTests(unittest.TestCase):
    block = WORKFLOW.split("\nenv:\n")[0].split("\nconcurrency:\n", 1)[1]

    def test_only_pull_request_runs_are_cancelled(self):
        self.assertIn("cancel-in-progress: ${{ github.event_name == 'pull_request' }}", self.block)

    def test_each_pull_request_has_its_own_group(self):
        # A stack's PRs must not cancel each other; release runs keep the `pages` group.
        self.assertIn("format('validation-pr-{0}', github.event.pull_request.number)", self.block)
        self.assertRegex(self.block, r"&& 'pages' \|\|")
        self.assertIn("format('validation-{0}', github.run_id)", self.block)


class PatchedPairWorkflowTests(unittest.TestCase):
    suites = {"web": "build browser"}
    cache_path = "${{ runner.temp }}/wasm-bindgen-patched"
    builder_path = "scripts/build-patched-wasm-bindgen.py"
    bins = ("wasm-bindgen", "wasm-bindgen-test-runner")
    compatible_lock = '[[package]]\nname = "wasm-bindgen"\nversion = "0.2.129"\n'

    def setUp(self):
        self.steps = re.split(r"^      - ", job("web"), flags=re.M)[1:]
        self.key = self.step("Validate workspace and identify patched wasm-bindgen pair")
        self.restore = self.step("Cache patched wasm-bindgen pair")
        self.build = self.step("Build patched wasm-bindgen pair on cache miss")
        self.verify = self.step("Verify patched wasm-bindgen pair")
        self.consumers = {
            "web": self.step("Build and run browser tests"),
        }
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)
        self.pair = self.root / "wasm-bindgen-patched"
        self.lock = self.root / "Cargo.lock"
        self.lock.write_text(self.compatible_lock + '[[package]]\nname = "unrelated"\nversion = "1.0.0"\n')
        self.output = self.root / "output"
        self.trace = self.root / "trace"
        self.trace.touch()
        self.env = {
            **os.environ, "PROJECT_SCRIPTS": str(REPO / "scripts"), "RUNNER_TEMP": str(self.root), "GITHUB_OUTPUT": str(self.output),
            "CARGO_HOME": str(self.root / "cargo-home"), "RUSTC": str(self.root / "rustc"),
            "PATH": str(self.root) + os.pathsep + os.environ["PATH"], "TRACE": str(self.trace),
        }
        # Fixture subprocesses use fake Cargo and do not inherit the real build-lock descriptor.
        self.env.pop("BOARDSTUDIO_BUILD_LOCK_FD", None)
        self.write_executable(self.root / "rustc", 'echo "fixture rustc $RUSTUP_TOOLCHAIN"\n')
        self.write_executable(self.root / "cargo", textwrap.dedent('''\
            set -eu
            echo build >> "$TRACE"
            mkdir -p "$CARGO_TARGET_DIR/release"
            for name in wasm-bindgen wasm-bindgen-test-runner; do
              cat > "$CARGO_TARGET_DIR/release/$name" <<EOF
            #!/bin/sh
            echo "\\$0" >> "\\$TRACE"
            echo "$name 0.2.129"
            EOF
              chmod +x "$CARGO_TARGET_DIR/release/$name"
            done
            '''))
        (self.root / ".cargo").mkdir()
        (self.root / "scripts").mkdir()
        shutil.copyfile(REPO / "rust-toolchain.toml", self.root / "rust-toolchain.toml")
        shutil.copyfile(REPO / ".cargo/config.toml", self.root / ".cargo/config.toml")
        for helper in ("wasm_bindgen_tools.py", "browser_tools.py"):
            shutil.copyfile(REPO / "scripts" / helper, self.root / "scripts" / helper)
        # Run the real builder in the fixture checkout, with locally generated pinned archives.
        builder = runpy.run_path(str(REPO / self.builder_path))
        source = (REPO / self.builder_path).read_text()
        for name, files in (
            ("wasm-bindgen-cli-0.2.129", {"Cargo.lock": builder["TINY_LOCK"]}),
            ("tiny_http-0.12.0", {"src/util/task_pool.rs": builder["OLD"]}),
        ):
            archive = self.make_archive(name, files)
            entry = f'"{name}": "{builder["CRATES"][name]}"'
            self.assertEqual(source.count(entry), 1)
            source = source.replace(entry, f'"{name}": "{builder["digest"](archive)}"')
        (self.root / self.builder_path).write_text(source)
        (self.root / "scripts/check.py").write_text(textwrap.dedent('''\
            import os
            import shutil
            import sys
            from pathlib import Path
            from unittest.mock import patch
            sys.path.insert(0, os.environ['PROJECT_SCRIPTS'])
            import check
            pair = Path(os.environ["BOARDSTUDIO_WASM_BINDGEN_DIR"])
            assert pair == Path(os.environ["RUNNER_TEMP"]) / "wasm-bindgen-patched"
            assert os.environ["CARGO_PROFILE_DEV_DEBUG"] == "false"
            def observe_launch(name, commands, env, *args, **kwargs):
                for binary in ('wasm-bindgen', 'wasm-bindgen-test-runner'):
                    assert shutil.which(binary, path=env['PATH']) == str(pair / binary)
                return 0.0, True
            suite = sys.argv[-1]
            with patch.dict(os.environ, {'CHROME_BIN': '/bin/true', 'CHROMEDRIVER': '/bin/true'}), \
                 patch.object(check, 'run_commands', observe_launch):
                assert check._run_plan([(suite, check.STEPS[suite][1])]) == 0
            with open(os.environ["TRACE"], "a") as trace:
                trace.write("consumer " + " ".join(sys.argv[1:]) + " " + str(pair) + "\\n")
            '''))

    @staticmethod
    def write_executable(path, body):
        path.write_text("#!/bin/sh\n" + body)
        path.chmod(0o755)

    def make_archive(self, name, files):
        source = self.root / name
        for path, content in files.items():
            (source / path).parent.mkdir(parents=True, exist_ok=True)
            (source / path).write_text(content)
        archive = Path(self.env["CARGO_HOME"]) / "registry/cache/fixture" / (name + ".crate")
        archive.parent.mkdir(parents=True, exist_ok=True)
        with tarfile.open(archive, "w:gz") as tar:
            tar.add(source, arcname=name)
        return archive

    def step(self, name):
        [step] = [step for step in self.steps if step.startswith(f"name: {name}\n")]
        return step

    def run_step(self, step, **env):
        command = re.search(r"^        run: (.*)\n((?:^          .*\n)*)", step, re.M)
        shell = textwrap.dedent(command[2]) if command[1] == "|" else command[1]
        return subprocess.run(["bash", "-e", "-c", shell], cwd=self.root, env={**self.env, **env},
                              text=True, capture_output=True)

    def assert_step_passes(self, step, **env):
        result = self.run_step(step, **env)
        self.assertEqual(result.returncode, 0, result.stderr)

    def run_web_steps(self, suite, cache):
        self.output.unlink(missing_ok=True)
        for step in self.steps:
            if step == self.key:
                result = self.run_step(step)
                if result.returncode:
                    return result
                key = self.output.read_text().strip().removeprefix("key=")
                self.assertRegex(key, r"^patched-wasm-bindgen-v1-[0-9a-f]{64}$")
            elif step == self.restore:
                hit = key in cache
                if hit:
                    shutil.copytree(cache[key], self.pair)
            elif step == self.build and not hit:
                self.assert_step_passes(step)
            elif step == self.verify:
                result = self.run_step(step)
                if result.returncode:
                    return result
            elif step == self.consumers[suite]:
                self.assert_step_passes(step, BOARDSTUDIO_WASM_BINDGEN_DIR=str(self.pair),
                                        CARGO_PROFILE_DEV_DEBUG="false")
        if not hit:
            cache[key] = self.root / key
            shutil.copytree(self.pair, cache[key])
        return result

    def test_workflow_orders_and_connects_patched_pair_steps(self):
        for suite, step in self.consumers.items():
            ordered = [self.steps.index(part) for part in (self.key, self.restore, self.build, self.verify, step)]
            self.assertEqual(ordered, sorted(ordered))
            self.assertNotIn("matrix.suite", step)
            self.assertIn(f"        run: python3 scripts/check.py {self.suites[suite]}\n", step)
            self.assertIn(f"          BOARDSTUDIO_WASM_BINDGEN_DIR: {self.cache_path}\n", step)
            self.assertIn('          CARGO_PROFILE_DEV_DEBUG: "false"\n', step)
        setup = self.key + self.restore + self.build + self.verify
        self.assertEqual(setup.count("\n        if:"), 1)
        self.assertNotIn("continue-on-error:", setup)
        self.assertLess(job("web").index("run: rustup toolchain install"), job("web").index(self.key))
        self.assertIn("        id: patched-pair-key\n", self.key)
        self.assertIn("        id: patched-pair\n", self.restore)
        self.assertIn("        uses: actions/cache@", self.restore)
        self.assertIn("          key: ${{ steps.patched-pair-key.outputs.key }}\n", self.restore)
        self.assertIn(f"          path: {self.cache_path}\n", self.restore)
        self.assertNotIn("restore-keys:", self.restore)
        self.assertIn("        if: steps.patched-pair.outputs.cache-hit != 'true'\n", self.build)
        for step, mode in ((self.build, "--output"), (self.verify, "--verify")):
            self.assertIn(f'        run: python3 {self.builder_path} {mode} "$RUNNER_TEMP/wasm-bindgen-patched"\n', step)

    def test_web_builds_on_miss_and_reuses_pair_on_hit(self):
        original = self.lock.read_text()
        self.lock.write_text(original)
        self.trace.write_text("")
        cache = {}
        self.assertEqual(self.run_web_steps("web", cache).returncode, 0)
        [key] = cache
        self.assertEqual(set(path.name for path in self.pair.iterdir()), set(self.bins))
        shutil.rmtree(self.pair)
        self.lock.write_text(original.replace("1.0.0", "2.0.0"))
        self.assertEqual(self.run_web_steps("web", cache).returncode, 0)
        self.assertEqual(list(cache), [key])
        trace = self.trace.read_text().splitlines()
        self.assertEqual(trace.count("build"), 1)
        verified = [str(self.pair / name) for name in self.bins]
        self.assertEqual(trace[-10:], [*verified, *verified, f"consumer {self.suites['web']} {self.pair}",
                                       *verified, *verified, f"consumer {self.suites['web']} {self.pair}"])
        shutil.rmtree(self.pair)
        shutil.rmtree(cache[key])

    def test_workflow_key_tracks_compiler_override_and_rustflags(self):
        def emitted_key():
            self.output.unlink(missing_ok=True)
            self.assert_step_passes(self.key)
            return self.output.read_text().strip()

        self.env.pop("RUSTFLAGS", None)
        self.env.pop("CARGO_BUILD_RUSTC", None)
        self.env.pop("RUSTC")
        original = emitted_key()
        self.env["RUSTFLAGS"] = "-C opt-level=1"
        self.assertNotEqual(emitted_key(), original)
        del self.env["RUSTFLAGS"]
        alternate = self.root / "alternate-rustc"
        self.env["CARGO_BUILD_RUSTC"] = str(alternate)
        self.write_executable(alternate, 'echo "alternate compiler A"\n')
        first = emitted_key()
        self.write_executable(alternate, 'echo "alternate compiler B"\n')
        self.assertNotEqual(emitted_key(), first)

    def test_incompatible_lock_blocks_prospective_hit_before_restore(self):
        cache = {}
        self.assertEqual(self.run_web_steps("web", cache).returncode, 0)
        shutil.rmtree(self.pair)
        self.trace.write_text("")
        for case, lock in (
            ("zero packages", "version = 4\n"),
            ("duplicate pinned packages", self.compatible_lock * 2),
            ("incompatible version", self.compatible_lock.replace("129", "128")),
        ):
            for suite in self.suites:
                with self.subTest(case=case, suite=suite):
                    self.lock.write_text(lock)
                    result = self.run_web_steps(suite, cache)
                    self.assertNotEqual(result.returncode, 0)
                    self.assertIn("exactly one wasm-bindgen", result.stderr)
                    self.assertFalse(self.output.exists())
                    self.assertFalse(self.pair.exists())
                    self.assertEqual(self.trace.read_text(), "")

    def test_bad_cached_binary_blocks_either_consumer(self):
        cache = {}
        self.assertEqual(self.run_web_steps("web", cache).returncode, 0)
        [cached_pair] = cache.values()
        shutil.rmtree(self.pair)
        for suite in self.suites:
            for name in self.bins:
                tool = cached_pair / name
                original = tool.read_text()
                for fault, contents in (("missing", None), ("execution failure", "exit 9\n"),
                                        ("unexpected name", "echo wrong-name 0.2.129\n"),
                                        ("unexpected version", f"echo {name} 0.2.128\n")):
                    with self.subTest(suite=suite, binary=name, fault=fault):
                        self.trace.write_text("")
                        if contents is None:
                            tool.unlink()
                        else:
                            self.write_executable(tool, contents)
                        result = self.run_web_steps(suite, cache)
                        self.assertNotEqual(result.returncode, 0)
                        self.assertIn(name, result.stderr)
                        trace = self.trace.read_text()
                        self.assertNotIn("consumer ", trace)
                        self.assertNotIn("build", trace)
                        tool.write_text(original)
                        tool.chmod(0o755)
                        shutil.rmtree(self.pair)


if __name__ == "__main__":
    unittest.main()
