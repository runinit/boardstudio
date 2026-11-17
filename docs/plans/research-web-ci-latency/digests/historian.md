# Historian digest (role taken inline)

Worked back from `46d758d` with `git blame -L` on the cited lines and `git show -s` on each commit. Ten commits followed; the budget of ten is spent. Pull requests were not looked up. Six of them (`12f37299b`, `7f787b9cb`, `62a274623`, `bf6ba7288`, `403e13a69`, `f82c10263`) carry an AI co-author trailer. Each message is its author's claim about intent, checked here only against the lines it blames to, not against the full diff.

| Commit | Date | Subject | Lines it owns at `46d758d` |
| --- | --- | --- | --- |
| `12f37299b` | 2026-10-09 | "Speed up the check: no test debug info, fast build, parallel launches and a quick mode" | `scripts/run-wasm-tests.py:209-212`, part of `scripts/check.py:279-284` |
| `7f787b9cb` | 2026-10-28 | "ci: let same-repo pull requests save their own rust-cache scope" | `.github/workflows/check.yaml:44-46` |
| `f9340574c` | 2026-10-28 | "tooling: default the drag smoke to the Rust harness, retire the Node path" | `scripts/check.py:515`, `scripts/check.py:636` |
| `62a274623` | 2026-10-30 | "Compile every browser crate launch in one prebuild" | `scripts/check.py:96-104` |
| `7102a22d0` | 2026-10-30 | "Overlap page and provider builds (ticket 05)" | `scripts/build-web.py:74-98` |
| `f8a0b8fbd` | 2026-10-30 | "Handle interrupted parallel checks" | `scripts/check.py:584-597`, `681-691`, `759-779` |
| `9ba41e4a6` | 2026-10-30 | "WIP: checkpoint browser build and Node-free tooling work" | added `scripts/build-patched-wasm-bindgen.py` and `scripts/wasm_bindgen_tools.py` |
| `bf6ba7288` | 2026-10-31 | "Stop browser launches waiting for the full page load" | `scripts/run-wasm-tests.py:512-514` |
| `403e13a69` | 2026-10-31 | "Run page filters concurrently with ordered reports" | `scripts/run-wasm-tests.py:548-557` |
| `f82c10263` | 2026-10-31 | "Set CPU-aware default check jobs" | most of `scripts/check.py:279-284` |

## Rationale found, quoted

- `bf6ba7288` (load-bearing): "wasm-bindgen-test-runner reads no test output until navigation returns and allows that 300 s, so a page slow to reach load stalled passing launches silently. Eager navigation returns at DOMContentLoaded, leaving the wait under the test timeout."
- `7f787b9cb` (load-bearing): "cache-on-failure stays off: its post-if (`success() || CACHE_ON_FAILURE == 'true'`) also runs when cancel-in-progress cancels a superseded run, and a partial target saved under the immutable key would be exact-hit, and never overwritten, by every later run." Also: "GitHub writes a pull_request cache to refs/pull/N/merge, which only later pushes to that pull request can restore; main, dev and sibling pull requests cannot read it."
- `62a274623`: "Launches each resolved different first-party features, so they recompiled Core and the web crates while queued on the build lock ... one cargo test --no-run now serves the 12 crates." It reports "Cold check.py 1031.2 s to 986.6 s, browser 491.2 s to 446.6 s", measured on a machine the message does not name.
- `7102a22d0`: "Provider lanes plus a dx lane for fast single-route builds. Cold build step 282s -> 197s; page wasm hash unchanged."

## Decisions the history shows

- Eager page-load strategy over the default, to bound a silent navigation wait (`bf6ba7288`).
- No cache save on failure, over saving partial targets, because a cancelled run would save too (`7f787b9cb`).
- A pull request saves its own cache scope, over main/dev-only saves, so stacked pull requests stop starting cold (`7f787b9cb`).
- One prebuild for all crate launches, over per-launch compiles queued on the build lock (`62a274623`).

## Not found

- No message body explains `403e13a69` (three workers, ordered reports), `f82c10263` (the `min(6, cpu)` default), `f8a0b8fbd` (signal handling) or `9ba41e4a6` (the patched `tiny_http` pair). Their reasons are not in the history followed.
- No commit followed mentions hosted-runner timings near 300 s per launch.
