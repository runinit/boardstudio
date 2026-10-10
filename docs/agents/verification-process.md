# Implementation and verification process

How a ticket is built and checked. Agreed with the user on 2026-10-09 after the
generator-parameters effort showed that most wall time went to repeating the full check
cycle (about ten minutes of machine time) in several checkouts. Applies to every ticket
and every agent. Tracker conventions: [issue-tracker.md](issue-tracker.md). Check steps:
`python3 scripts/check.py --list`.

## Roles

- **Implement** (Luna 6, high, fast, from 2026-10-10; before that Haiku 5.5): test first, make the change,
  run only the steps the change needs (below), write the Outcome honestly, set
  `Status: implemented (awaiting verification)`. Never `resolved`. Time box: about an hour.
- **Verify and integrate** (Sol 6.1, xhigh, fast, from 2026-10-10; before that Sonnet 5.5 at high effort): do not trust the
  implementer's report. Reproduce its claims, mutation-check the main assertions (disable one site at a time,
  confirm a test fails, restore), review the code, fix **small** defects with tests, then set `resolved` with
  who-verified-what evidence. Time box: about 30 to 45 minutes of work. The verifier is a fresh session with a different
  model from the implementer (Luna implements, Sol verifies); using a different app alone does not meet this requirement.
- **Planner / orchestrator** (Sol 6.1, medium, fast, from 2026-10-10; planning research Sol 6.1 max fast, Astra medium as a last resort; before that Claude): writes tickets, ADRs and prompts, decides the order, hands
  tickets to implementers and verifiers, merges lanes, runs the batch check and refreshes GitNexus. It does not
  verify, and it does not implement unless a task is tiny and documentation-only.
- The verifier is a reviewer, not a second implementer. A defect that needs more than a
  small, clearly correct fix (roughly a screen of code and one test) becomes a follow-up
  ticket with file:line and a test plan; the ticket is resolved only if its own acceptance
  holds, otherwise it stays `implemented (awaiting verification)` with the exact blockers.
- **Scope is fixed when the agent starts.** New decisions or requests go into tickets, not
  into messages to a running agent. This is what made one verification run over an hour.
- Small tickets (docs, one file, no behaviour) may be implemented and verified in a single
  Sol pass. This exception does not cover code, hooks or configuration that changes execution.
- Planning and decision work is read-only; the user answers decisions. All role models are in [agent-workflow.md](agent-workflow.md).
- Say which model did what in the Outcome. Never claim a result you did not observe.

## Ticket size

One concern per ticket, small enough to implement in about an hour and review in about 30
minutes. If a ticket needs several unrelated behaviours, split it before starting.

## Run the steps the change needs, not the whole check

| Change | Steps |
| --- | --- |
| Docs, tickets, ADRs only | `repo` |
| Python scripts or check tooling | `repo`, `tooling`, plus a changed script's tests if not yet in the tooling list |
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
  `docs/plans/check-speed/issues/10-nested-worktrees.md`). Its `cad/.cache`,
  `cad/wasm/pkg` and `cad/wasm/target` must be real directories, not symlinks into another
  checkout: `build` refuses a symlink because the CAD container cannot resolve it.
- `wasm-pack`, Chrome and podman need a runtime that permits their processes and filesystem access. In Claude the per-command
  option is `dangerouslyDisableSandbox`. In Codex follow the active session's permission policy: with `danger-full-access` and
  approval policy `never`, run normally and omit `sandbox_permissions`. In a restricted session, report a denied check and use
  the permitted escalation mechanism if available. A blocked run remains unverified.
- Stage explicit paths; never `git stash` or `git add -A`. Lane agents do not merge; the orchestrator merges verified lanes
  into the integration branch. Pushes, PRs and merges to `dev` need the user's authorization. End commits with the trailer
  naming the agent that actually made them.

## Hygiene

- After browser runs, remove stale Chromium profiles older than 30 minutes
  (`/tmp/org.chromium.Chromium.scoped_dir.*`) and any scratch checkouts you created.
  `/tmp` is a tmpfs and filled to 98% once.
- GitNexus: run `impact` before editing a symbol and `detect-changes` before committing.
  A stale index or capped result is not a clean check; say so. Agents never re-index: the
  orchestrating session does, once, at batch boundaries ([gitnexus-index.md](gitnexus-index.md)).

## Honest reporting

State in the Outcome what ran, what it printed, what did not run and why, and what the
change leaves unverified. A step that fails for an environmental reason is reported as
such, not as a pass.
