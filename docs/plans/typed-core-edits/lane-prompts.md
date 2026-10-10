# Lane prompts

Paste-ready prompts for [parallel-lanes.md](parallel-lanes.md). Each names its worktree, branch and tickets; the
ticket files hold the detail. The orchestrator sets the lane up first (see "Setting up a lane" in that page).
Replace `<agent>` in commit trailers with the app that makes the commit (for example `Codex`, `Claude Sonnet 5.5`).

## Common rules (every lane prompt starts with these)

> Read `AGENTS.md`, `docs/agents/verification-process.md` and `docs/plans/typed-core-edits/parallel-lanes.md`. Work
> only in your lane worktree on your lane branch. Claim a ticket by setting `Status: claimed` and committing that
> change. Implement test-first; run only the check steps the process table names, one command at a time in the
> foreground, with the sandbox lifted only for `wasm-pack`, Chrome and the CAD container. Stage explicit paths; no
> `git stash`, no `git add -A`, no push, no merge. Do not edit `docs/plans/*/map.md`, `CONTEXT.md`, `docs/adr/*` or the
> generated goldens (`core/tests/fixtures/boundary/transcript.json`, `core/tests/fixtures/demo-projects-baseline.json`)
> unless your ticket names them; propose text in your ticket Outcome instead. Do not refresh GitNexus; use `rg` for
> callers and say the index may be stale. Never run `scripts/check.py release`. Time box about two hours. When done,
> set the ticket `Status: implemented (awaiting verification)` and write an honest `## Outcome`: commands with
> timings, what you did not run, decisions, deviations. Report exactly what you observed.

## Lane B: mirror core (worktree `.worktrees/lane-mirror-core`, branch `lane/mirror-core`)

### B1: ticket 31, companion placement modes

> Common rules above. Ticket: `docs/plans/typed-core-edits/issues/31-companion-placement-modes.md`. Also read
> ADR-0010 (all amendments), tickets 29, 30 and 18 (Verification), `core/src/mirrored_half.rs`,
> `core/src/mechanical/halves.rs`, `core/src/model.rs` (pair and `halves` types). Check steps: `repo`, `lint`,
> native `test`, `typecheck`, and the explicit wasm run `wasm-pack test --headless --chrome web/crates/host
> --locked --features page --lib -- boundary_parity::` only if the transcript changes (it is a generated golden:
> re-bless with `BOUNDARY_BLESS=1` only for your own additive operation transcript entries and say so).

### B2: ticket 32, optional labels

> Common rules above. Ticket: `docs/plans/typed-core-edits/issues/32-mirrored-half-labels.md`. Start after B1 is
> merged into your lane (same branch). Check steps as B1. Keep the ticket-26 equivalence test in
> `web/crates/catalogue` passing; shrink its pinned difference list only for the relabelling rows when labels are passed.

### B3: ticket 19, reconcile on every edit (run alone in the lane, after B1 and B2 are resolved)

> Common rules above. Ticket: `docs/plans/typed-core-edits/issues/19-reconcile-mirrored-halves.md`, including its
> "Contracts from other tickets". It edits the CRITICAL-impact apply path: list the callers with `rg` before you
> change it, keep `core/tests/matrix_performance.rs` passing and record the added per-edit cost. Check steps: `repo`,
> `lint`, `test`, `typecheck`, wasm boundary parity.

## Lane A: web test speed (worktree `.worktrees/lane-web-sleeps`, branch `lane/web-sleeps`; set up after ticket 36 is resolved)

### A1: ticket 37, remaining fixed sleeps (includes the work carried from 36)

> Common rules above. Ticket: `docs/plans/typed-core-edits/issues/37-sleeps-layout-library-ui-page.md` including its
> "Carried from 36" section. Read tickets 34 and 36 (Outcome and Verification) first: the lessons there are binding
> (the predicate must be the precondition of the NEXT assertion; negative assertions need a sentinel or a note; every gate
> is bounded), and use `web/crates/ui-shared/src/test_wait.rs` (`wait_until`, `wait_until_step`, `wait_until_future`).
> Check steps: `repo`, `lint`, `typecheck`, `browser` three times in a row at the end, run alone on the machine if you can
> (note `uptime` load before each timed run). Diff the crate `--list` outputs before and after for every crate you touch.
> Mutation-check three waits per suite family. Remove Chromium profiles older than 30 minutes when you finish.

### A2: ticket 38, merge exact duplicate browser tests (after A1 is verified)

> Common rules above. Ticket: `docs/plans/typed-core-edits/issues/38-merge-duplicate-browser-tests.md`. Re-read both
> bodies first; merge only exact duplicates; list what each merge preserves. Check steps as A1.

## Lane F: shipped boards review research (worktree `.worktrees/lane-boards-review`, branch `lane/boards-review`; no builds)

> Common rules above. Ticket: `docs/plans/typed-core-edits/issues/25-shipped-boards-review.md`. This is PHASE 1,
> read-only research: produce `docs/plans/typed-core-edits/audit/shipped-boards-review.md` with, for each of the 20
> shipped projects (Sofle v2, RGB and Choc FIRST, then the seven measured splits, then the unibody boards, the
> REVIEUNG41 archive and the VIK review): the part list and footprints, library source, side and `reversible` settings,
> positions against the measured source JSON under `content/layouts`, mounting holes and keep-outs, the generated
> bay/reset/TRRS placeholders, the recorded fit findings (tickets generator-parameters 02 and 06, typed-core-edits 04
> Outcome), and the rule differences from ADR-0010 (headings: independent parts take `180 - r`; switches `-r`; the
> companion modes of ticket 31). Rank findings by severity (blocks a build, wrong physically, cosmetic) with file or
> fixture references. Ask the user for hardware decisions; do not guess. Do NOT change recipes, fixtures, footprints
> or code. No builds are needed; do not run `cargo`. Run `python3 scripts/check.py repo` before committing the doc.
> Finish with the ticket `Status: ready-for-human` (the user reviews the ranked list before fixes start) and a short
> Outcome linking the review.

## Verifier prompts (a different app than the implementer)

> Common rules above, plus: you are a REVIEWER, time box about 45 minutes. Do not trust the Outcome; reproduce its
> claims; mutation-check the main assertions (disable one site at a time, confirm a test fails, restore, touch the
> sources); fix only small clearly correct defects (about a screen plus a test); anything bigger becomes a follow-up in the
> ticket's `## Verification` section. Set the ticket `resolved` only if its own acceptance holds, with who-verified-what
> evidence, and report findings, commits, commands with timings and unverified items.
