# Implementation and verification process

How a ticket is built and checked. Agreed with the user on 2026-10-09 after the
generator-parameters effort showed that most wall time went to repeating the full check
cycle (about ten minutes of machine time) in several checkouts. Applies to every ticket
and every agent. Tracker conventions: [issue-tracker.md](issue-tracker.md). Check steps:
`python3 scripts/check.py --list`.

## Roles

- **Implement** (Luna 6 at the lowest sufficient effort; medium for routine changes, high when warranted by evidence; before that Haiku 5.5): test first, make the change,
  run only the steps the change needs (below), write the Outcome honestly, set
  `Status: implemented (awaiting verification)`. Never `resolved`. Time box: about an hour.
- **Review** (Sol 6.1, medium by default, user revision 2026-10-11): one fresh independent session after the completed
  Luna implementation handoff. Review the final diff and source-bound raw check evidence; reuse exact writer checks when
  sufficient. Rerun targeted verification only for a concrete risk or evidence gap. Use xhigh only when actual complexity or
  escalation justifies it. Mutation-check only important claimed guarantees that need that proof; it is not an automatic
  per-ticket task. Fix **small** clearly correct defects with tests, then set `resolved` with who-reviewed-what evidence.
  Reviewer and implementer use different sessions/models; using a different app alone does not meet this requirement.
- **Planner / orchestrator** (Sol 6.1, medium, from 2026-10-10; planning research Luna 6 medium/high for bounded work; Sol 6.1 max only for material cross-domain risk, Astra medium only as a last resort; before that Claude): writes tickets, ADRs and prompts, decides the order, hands
  tickets to implementers and verifiers, merges lanes, runs the batch check and refreshes GitNexus. It does not
  verify, and it does not implement unless a task is tiny and documentation-only.
- The verifier is a reviewer, not a second implementer. A defect that needs more than a
  small, clearly correct fix (roughly a screen of code and one test) becomes a follow-up
  ticket with file:line and a test plan; the ticket is resolved only if its own acceptance
  holds, otherwise it stays `implemented (awaiting verification)` with the exact blockers.
- **Scope is fixed when the agent starts.** New decisions or requests go into tickets, not
  into messages to a running agent. This is what made one verification run over an hour.
- Low-risk docs, mechanical copy and tracking may skip a separate verifier: author checks plus root review suffice, and root
  may resolve with that evidence. Behavioural/code fixes use the one fresh Sol medium review above; execution-changing hooks
  or configuration are not mechanical copy. Preserve meaningful regression RED/GREEN and actual WASM compilation for presentation.
- Existing ticket acceptance, counterexamples and unresolved gates remain recorded. Reassess counted gates in a bounded root
  decision; this default does not silently close106 or remove its counterexamples.
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
| Rust in Core or other native crates, no web or wasm-visible change | `repo`, affected lint and focused/affected crate tests; full workspace `test` at integration |
| Core or shared types the page also compiles | add `typecheck`; add the explicit wasm `boundary_parity::` run when the transcript changes |
| Mounted web behaviour (web crates, Dioxus components, storage) | add actual WASM compilation and affected mounted/browser tests; full `browser` at integration |
| Generated demo projects or fixtures change | add `build` |
| Dependencies change | add `deny` |

- Lane development uses focused regressions and affected targets rather than the full workspace/native/browser suites. For web changes retain actual WASM compilation and affected mounted tests; for Core boundary changes retain affected parity coverage. Broaden a lane run only for a concrete cross-cutting risk or evidence gap, with the reason recorded.
- `python3 scripts/check.py quick` runs only the checks covering your changed files; use
  it while iterating.
- The **full cycle** (`repo lint typecheck build browser test`) runs **once per integration batch** of tickets that land together.
  Verifier and root reuse sufficient source-bound checks rather than repeating full suites. Effort closure alone does not require
  another full run; rerun only after changed inputs, a new risk or an evidence gap. Preserve the full-check requirement before a PR.
- `release` is never run by an agent.

Apply [laptop multi-stream resource mode](agent-workflow.md#laptop-multi-stream-resource-mode) during concurrent workstreams: four Cargo jobs, four native test threads and one check-suite launch at a time. Full integration checks keep those caps. Reviewers reuse adequate exact-source writer results; a resource-cap change alone does not invalidate them.

## One run at a time

- The user-approved scheduling policy in [agent-workflow.md](agent-workflow.md#standing-lane-authority-and-continuity) supersedes older global-every-shell/read restrictions. Read-only shell commands/searches/diffs/graph queries and approved disjoint source work in separate worktrees run freely; keep one writer per worktree and protect shared writable outputs.
- Builds/compilation, test execution, browser, timing and index refresh share ONE **GLOBAL HEAVY** slot with explicit owner/release. Never overlap resource-heavy runs; timing needs the machine alone. Execute commands sequentially within each lane. Overlapping builds can corrupt shared wasm-pack output (ticket10); separate worktrees do not waive the HEAVY slot or output ownership.
- Grant one complete bounded debugging + affected-check cycle. Carry approved RED → fix → GREEN within that same HEAVY window, without per-command/single-probe handoffs. Fresh independent review follows completed writer STOP; it reuses sufficient checks and needs a HEAVY slot only for necessary heavy verification. Complete/restore an existing probe before switching writers.
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

For the normal precommit delta, stage explicit paths, then use one complete `scope:all` result bound to the final source and
staged delta. An identical optional `scope:staged` response adds no gate. Reuse an earlier complete result only after verifying
that final paths, bytes and delta are unchanged after staging; do not infer equivalence from a clean-looking status. Record the
worktree, HEAD, paths and source hashes with the evidence. `impact` remains mandatory before symbol edits. HIGH/CRITICAL warnings
must be reported, UNKNOWN requires source confirmation, and partial or truncated analysis remains unresolved.

## Honest reporting

State in the Outcome what ran, what it printed, what did not run and why, and what the
change leaves unverified. A step that fails for an environmental reason is reported as
such, not as a pass.
