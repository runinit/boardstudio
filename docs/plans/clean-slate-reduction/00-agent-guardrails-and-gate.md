# Track 0: Non-Negotiable Agent Guardrails & Mechanical Reduction Gate

This document defines the **hard operating rules** for every orchestrator, implementer, and reviewer participating in the Clean-Slate Codebase Reduction Program. Any PR, branch, or subagent handoff that violates **any** rule below MUST be rejected immediately — even if all tests pass.

---

## The 10 Iron Laws

### Law 1 — Hard Negative-LOC Gate on BOTH Production AND Test Rust
Every track and sub-track has a **Hard LOC Budget** measured across the **entire repository** (`git diff <base> HEAD`) using [check_reduction.py](check_reduction.py):
- **Net Production Rust LOC** MUST be `<= track_max_prod_delta` (strictly negative).
- **Net Test Rust LOC** MUST be `<= track_max_test_delta` (strictly negative).
- **Net Scripts LOC** MUST be `<= 0` (except Track 5, where it MUST be `<= -15,000`).
- **Net Other LOC** (`.evidence/`, markdown inventories, JSON fixtures) MUST be `<= +200`.

**"Tests pass" does NOT override a failed LOC gate.** If a branch finishes net-positive (or misses its negative budget), it is **incomplete** and cannot be merged.

### Law 2 — Delete First; Never Wrap, Layer, or Keep Dual Paths
When replacing a subsystem, widget, controller, or format:
- **Delete the superseded implementation in the SAME commit/track.**
- **FORBIDDEN:** "temporary adapters", "legacy fallback paths", "format-7 compatibility layers", "staged conversion UI", "original-file recovery stores", "dual-mode feature flags", or "keeping old files until a later wave".
- BoardStudio is **pre-release (not live)**. There are **zero** production user files to migrate. Update in-repo example/fixture files in place and reject older schema versions with a 1-line error.

### Law 3 — Legacy Narrative Tests Are NOT Untouchable Oracles
The `dev` codebase contains **248,907 LOC of Rust tests** (46% of all Rust code), including tens of thousands of lines of copy-pasted ticket/regression narratives tightly coupled to deleted internal implementations.
- **When you delete a legacy subsystem, you MUST delete its legacy narrative tests in the same track.**
- Replace them with **compact behavioral / property-based invariant tests** that test the public contract of the new, simpler architecture.
- **Hard Test-to-Prod Ratio Cap:** New/rewritten code in any track MUST keep **Test LOC ≤ 60% of Prod LOC** (and strictly below the track's Test LOC ceiling). Never write 3,000 lines of new narrative tests for 500 lines of new production code.

### Law 4 — No Silent Scope Rewrites or "Bug-for-Bug Preservation" Detours
Agents (especially autonomous orchestrators) tend to invent defensive requirements ("preserve every historical edge case", "keep false-parent decoy IDs", "keep exact legacy changed-ID arrays", "keep staged recovery").
- **FORBIDDEN:** Expanding requirements beyond what is written in the track's markdown plan.
- **Escalation Rule (The 100-LOC Circuit Breaker):** If preserving a legacy behavior or obscure edge case would require **more than 100 lines of production code** or force keeping a legacy module from the Mandatory Deletion Manifest, **STOP immediately** and ask the user in ≤ 3 bullet points:
  1. What the legacy quirk is.
  2. Why keeping it costs >100 LOC.
  3. Recommendation to drop/simplify it.
  **Never** silently write 500–5,000 lines of machinery to preserve an unrequested legacy quirk.

### Law 5 — Mandatory Deletion Manifests Are Pass/Fail
Every track plan contains a `## Mandatory Deletion Manifest` listing exact files/directories that MUST no longer exist in `git ls-files` at the end of the track, plus `## Forbidden Symbols` that MUST have **0 matches** in tracked `.rs` files.
- [check_reduction.py](check_reduction.py) verifies both mechanically.
- Renaming a file or moving its contents into another crate/module to bypass the manifest is a **critical violation** (and will be caught by the whole-repo LOC gate anyway).

### Law 6 — No Cross-Crate Code Shuffling Counted as Reduction
Moving 10,000 lines from `core/crates/mirrored-half` to `core/crates/engine` is **0 lines reduced**.
- Always report **whole-repository** `check_reduction.py` numbers (`Prod Rust`, `Test Rust`, `Scripts`, `Other`) relative to the wave's base commit (`dev` or prior merged wave).
- **FORBIDDEN:** Reporting net reductions relative to an intermediate WIP commit you created earlier in the same branch (e.g., building 3,000 lines of v7 migration code, deleting it, and reporting "−3,000 net lines").

### Law 7 — Zero Exploratory Scripts or Massive `.evidence/` Logs Committed
- Exploratory benchmarks, A/B trials, one-off measurement scripts, and temporary HTML/JS probes belong in `/tmp` and MUST be deleted before committing.
- **FORBIDDEN:** Committing new `scripts/run-*-trial.py`, `scripts/measure-*.py`, or `>100`-line `.evidence/**/*.md` inventory files.
- Keep `.evidence/` strictly for concise, ephemeral check logs as documented in [AGENTS.md](../../../AGENTS.md).

### Law 8 — No Unrequested Renderer or Architecture Detours
- **Keep SVG** for 2D Layout/PCB/Keymap/Keycaps canvases (already measured at 4.55 µs hit query and 16.67 ms frame pacing). **FORBIDDEN:** Canvas2D or WebGL2 2D rewrite trials.
- **Keep Rust + Dioxus + `rust-ui.com` primitives.** Do not introduce JS frameworks, new B-Rep kernels, or unapproved crate dependencies (check [deny.toml](../../../deny.toml) before adding any crate).

### Law 9 — Performance & Bundle Non-Regression Floor
Every completed wave MUST verify (at integration, not inside parallel lanes) that compared to `dev` (`5aae86ff4`):
- Page WASM bytes `<= dev` (27.05 MB) and total shipped WASM bytes strictly **smaller** after Track 2 retires `core-worker.wasm` (−11.6 MB!).
- Full `check.py` wall time `<= dev` (dropping toward `≤ 60–75s` by Track 5).
- Cold startup and drag-to-save latency `<= dev`.

### Law 10 — Strict Orchestration & Worktree Hygiene
- **Max 4 parallel subagent worktrees** at any time; zero overlapping file ownership between active lanes.
- **No full `check.py`, release builds, or Chrome suites inside parallel worktrees** — run only fast targeted `cargo check` / `cargo test -p <crate>` under the shared lock (`~/.cache/tmp/boardstudio-build.lock`, `BOARDSTUDIO_BUILD_LOCK_FD=9`, `TMPDIR="$HOME/.cache/tmp"`).
- **Immediately `git worktree remove`** every merged or abandoned delegate worktree; never leave stale worktrees accumulating gigabytes of `target/` directories.
- **Review the actual `git diff --stat` and `check_reduction.py` output** before merging any subagent branch.

---

## Using the Mechanical Reduction Gate (`check_reduction.py`)

Every track plan provides an exact invocation of [check_reduction.py](check_reduction.py). Example:

```bash
python3 docs/plans/clean-slate-reduction/check_reduction.py \
  --base 5aae86ff4 \
  --head HEAD \
  --max-prod-delta -15000 \
  --max-test-delta -25000 \
  --max-script-delta 0 \
  --max-other-delta 200 \
  --must-delete core/crates/mirrored-half/src/mirrored_half/reconcile \
  --must-delete core/crates/mirrored-half/src/mirrored_half/inverse \
  --forbid-symbol reconcile_pairs \
  --forbid-symbol canonical_request
```

Exit codes:
- `0` — ALL LOC budgets, deletion manifests, and forbidden-symbol checks PASSED.
- `1` — One or more guardrails FAILED (prints exact violations). **Do not merge or hand back until exit code is `0`.**

---

## Reviewer Rejection Checklist (Run Before Every Merge)

1. Did `check_reduction.py` exit `0` against the true wave base (`dev` or prior merged wave, NOT a mid-branch commit)?
2. Are ALL files in the track's Mandatory Deletion Manifest absent from `git ls-files`?
3. Are ALL Forbidden Symbols absent from `git grep` across `*.rs`?
4. Did any production file grow by wrapping an old file instead of replacing it? Check `git diff --stat <base>..HEAD` for suspicious large additions.
5. Did the test count/LOC shrink proportionally instead of piling new tests on top of old ones?
6. Were zero one-off trial/benchmark scripts or giant `.evidence/` markdown files added?
7. Did `cargo fmt --all --check` and the required `scripts/check.py` gates pass?
