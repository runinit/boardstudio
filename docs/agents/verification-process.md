# Implementation and verification process

How a ticket is built and checked. Agreed with the user on 2026-10-09 after the
generator-parameters effort showed that most wall time went to repeating the full check
cycle (about ten minutes of machine time) in several checkouts. Applies to every ticket
and every agent. Tracker conventions: [issue-tracker.md](issue-tracker.md). Check steps:
`python3 scripts/check.py --list`.

## Roles

- **Implement** (Haiku 5.5 unless the ticket says otherwise): test first, make the change,
  run only the steps the change needs (below), write the Outcome honestly, set
  `Status: implemented (awaiting verification)`. Never `resolved`.
- **Verify and integrate** (Sonnet 5.5, high effort): do not trust the implementer's
  report. Reproduce its claims, mutation-check the main assertions (disable one site at a
  time, confirm a test fails, restore), review the code, fix defects with tests, then set
  `resolved` with who-verified-what evidence and update the map.
- Planning and decision work (Opus, medium) is read-only; the user answers decisions.
- Say which model did what in the Outcome. Never claim a result you did not observe.

## Run the steps the change needs, not the whole check

| Change | Steps |
| --- | --- |
| Docs, tickets, ADRs only | `repo` |
| Rust in Core or other native crates, no web or wasm-visible change | `repo`, `lint`, `test` |
| Anything the page compiles (web crates, shared types, fixtures, the boundary transcript) | add `typecheck`, `browser` |
| Generated demo projects or fixtures change | add `build` |
| Dependencies change | add `deny` |

- `python3 scripts/check.py quick` runs only the checks covering your changed files; use
  it while iterating.
- The **full cycle** (`repo lint typecheck build browser test`) runs **once per batch**
  of tickets, and once more in the integration or verification ticket that closes an
  effort, not once per ticket. A batch is the tickets that land together.
- `release` is never run by an agent.

## One run at a time

- Run steps **sequentially, in the foreground**, and report only when they have finished.
  Do not start a second run while one is going; they fight over the build directory and
  lock, and the results are meaningless.
- Do not arm background monitors for a command you can simply wait for.
- Never mutate sources in one checkout and test in another when both share a `target`
  directory: restoring a file by copy can leave an artifact the other checkout accepts.
  Mutate only where you test, and touch the sources after restoring.

## Checkouts and sandbox

- Work in the ticket's worktree. Native `test` runs there too; a second "verify" checkout
  is a fallback only while the nested-worktree CAD problem exists (tracked in
  [check-speed](../plans/check-speed/map.md)). If you must use one, keep a single one with
  its own `target`, a real `cad/.cache` and no stale copies.
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
