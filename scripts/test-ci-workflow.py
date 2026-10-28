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
TRUSTED_SAVE = "${{ github.ref == 'refs/heads/main' || github.ref == 'refs/heads/dev' }}"


def job(name: str) -> str:
    match = re.search(rf"^  {re.escape(name)}:\n(.*?)(?=^  \S|\Z)", WORKFLOW, re.M | re.S)
    assert match, f"job {name} missing"
    return match.group(1)


def cache_steps(text: str) -> list[str]:
    parts = text.split("- uses: " + CACHE_ACTION)[1:]
    return [re.split(r"\n      - ", part, maxsplit=1)[0] for part in parts]


class CachePolicyTests(unittest.TestCase):
    def test_rust_cache_saves_only_from_trusted_branches(self):
        steps = [step for name in ("native", "web", "release-build") for step in cache_steps(job(name))]
        self.assertEqual(len(steps), 3)
        for step in steps:
            save_if = re.search(r"^ +save-if: (.*)$", step, re.M)
            self.assertIsNotNone(save_if, step)
            self.assertIn(save_if.group(1), (TRUSTED_SAVE, "false"),
                          "a pull request must never write a cache a trusted branch can read")

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
        built = {f"cad/{d}" for d in re.findall(r'CAD_ROOT / "([^"]+)/target"', CAD_BUILD)}
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
