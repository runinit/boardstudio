# Implementation and verification process

How a ticket is built and checked. Agreed with the user on 2026-10-09 after the
generator-parameters effort showed that most wall time went to repeating the full check
cycle (about ten minutes of machine time) in several checkouts. Applies to every ticket
and every agent. Tracker conventions: [issue-tracker.md](issue-tracker.md). Check steps:
`python3 scripts/check.py --list`.

## Roles

- **Implement** (Haiku 5.5 unless the ticket says otherwise): test first, make the change,
  run only the steps the change needs (below), write the Outcome honestly, set
  `Status: implemented (awaiting verification)`. Never `resolved`. Time box: about an hour.
- **Verify and integrate** (Sonnet 5.5, high effort): do not trust the implementer's
  report. Reproduce its claims, mutation-check the main assertions (disable one site at a
  time, confirm a test fails, restore), review the code, fix **small** defects with tests,
  then set `resolved` with who-verified-what evidence and update the map. Time box: about
  30 minutes of work.
- The verifier is a reviewer, not a second implementer. A defect that needs more than a
  small, clearly correct fix (roughly a screen of code and one test) becomes a follow-up
  ticket with file:line and a test plan; the ticket is resolved only if its own acceptance
  holds, otherwise it stays `implemented (awaiting verification)` with the exact blockers.
- **Scope is fixed when the agent starts.** New decisions or requests go into tickets, not
  into messages to a running agent. This is what made one verification run over an hour.
- Small tickets (docs, one file, no behaviour) may be implemented and verified in a single
  Sonnet pass.
- Planning and decision work (Opus, medium) is read-only; the user answers decisions.
- Say which model did what in the Outcome. Never claim a result you did not observe.

## Ticket size

One concern per ticket, small enough to implement in about an hour and review in about 30
minutes. If a ticket needs several unrelated behaviours, split it before starting.

## Run the steps the change needs, not the whole check

| Change | Steps |
| --- | --- |
| Docs, tickets, ADRs only | `repo` |
| Rust in Core or other native crates, no web or wasm-visible change | `repo`, `lint`, `test` |
| Core or shared types the page also compiles | add `typecheck`; add the explicit wasm `boundary_parity::` run when the transcript changes |
| Mounted web behaviour (web crates, Dioxus components, storage) | add `browser` |
| Generated demo projects or fixtures change | add `build` |
| Dependencies change | add `deny` |

- The full `browser` step (about 200 seconds) runs for web changes, and otherwise **once per
  batch**, not per Core ticket. `typecheck` (about 15 seconds) plus the wasm boundary parity
  run (about 35 seconds) cover Core changes the page compiles.
- `python3 scripts/check.py quick` runs only the checks covering your changed files; use
  it while iterating.
- The **full cycle** (`repo lint typecheck build browser test`) runs **once per batch**
  of tickets, and once more in the integration or verification ticket that closes an
  effort, not once per ticket. A batch is the tickets that land together.
- `release` is never run by an agent.

## One run at a time

- Run steps **sequentially, in the foreground**, and report only when they have finished.
  Do not start a second run while one is going; they fight over the build directory and
  lock, and the results are meaningless. Two overlapping `build`s fail in `wasm-pack` with
  `Installing wasm-bindgen ... invalid type: sequence, expected a string` because both write the
  same `--out-dir`; the error disappears when the runs are sequential (ticket 10).
- Do not arm background monitors for a command you can simply wait for.
- Never mutate sources in one checkout and test in another when both share a `target`
  directory: restoring a file by copy can leave an artifact the other checkout accepts.
  Mutate only where you test, and touch the sources after restoring.

## Checkouts and sandbox

- Work in the ticket's worktree. A nested worktree (under `.worktrees/`) runs every check step,
  including native `test` and `build`; no second "verify" checkout is needed (see
  [ticket 10](../plans/check-speed/issues/10-nested-worktrees.md)). Its `cad/.cache`,
  `cad/wasm/pkg` and `cad/wasm/target` must be real directories, not symlinks into another
  checkout: `build` refuses a symlink because the CAD container cannot resolve it.
- The sandbox must be lifted (`dangerouslyDisableSandbox`) for `wasm-pack`, Chrome and
  podman (the CAD container), and for nothing else.
- Stage explicit paths; never `git stash` or `git add -A`; no push, merge or PR without the
  user; end commits with the required `Co-Authored-By` trailer.

## Hygiene

- After browser runs, remove stale Chromium profiles older than 30 minutes
  (`/tmp/org.chromium.Chromium.scoped_dir.*`) and any scratch checkouts you created.
  `/tmp` is a tmpfs and filled to 98% once.
- GitNexus: run `impact` before editing a symbol and `detect-changes` before committing.
  A stale index or capped result is not a clean check; say so.

## Honest reporting

State in the Outcome what ran, what it printed, what did not run and why, and what the
change leaves unverified. A step that fails for an environmental reason is reported as
such, not as a pass.
