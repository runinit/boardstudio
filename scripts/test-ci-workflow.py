#!/usr/bin/env python3
"""Hermetic checks of the CI cache policy in .github/workflows/check.yaml.

Text-based on purpose: the CI Python has no YAML parser, and the existing workflow tests
(test-bump-chrome-for-testing.py) read the file the same way.
"""
from __future__ import annotations

import re
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
        keys = [re.search(r"shared-key: (\S+)", cache_steps(job(name))[0]) for name in ("web", "release-build")]
        self.assertTrue(all(keys))
        self.assertEqual(keys[0].group(1), keys[1].group(1))

    def test_native_caches_every_cad_target_dir(self):
        [step] = cache_steps(job("native"))
        cached = set(re.findall(r"^ +(\S+) -> target$", step, re.M))
        [targets] = re.findall(r'NATIVE_TEST_TARGETS = \((.*?)\)', CAD_BUILD, re.S)
        built = {f"cad/{d}" for d in re.findall(r'"([^"]+)/target"', targets)}
        self.assertIn("cad/monstertruck-qualification", built)
        self.assertLessEqual(built, cached, "cad_build.py builds into a target dir the CI cache skips")


class ConcurrencyTests(unittest.TestCase):
    block = WORKFLOW.split("\nenv:\n")[0].split("\nconcurrency:\n", 1)[1]

    def test_only_pull_request_runs_are_cancelled(self):
        self.assertIn("cancel-in-progress: ${{ github.event_name == 'pull_request' }}", self.block)

    def test_each_pull_request_has_its_own_group(self):
        # A stack's PRs must not cancel each other; release runs keep the `pages` group.
        self.assertIn("format('validation-pr-{0}', github.event.pull_request.number)", self.block)
        self.assertRegex(self.block, r"&& 'pages' \|\|")
        self.assertIn("format('validation-{0}', github.run_id)", self.block)


if __name__ == "__main__":
    unittest.main()
