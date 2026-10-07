# Handoff: running module deepening with parallel agents

For an orchestrator (Codex) running several implementation agents at once. The plan is
[map.md](map.md); each ticket is self-contained. This file says who runs what, when,
and how work merges. Tracker rules: [docs/agents/issue-tracker.md](../../agents/issue-tracker.md).
Repo rules: the root `AGENTS.md`.

## Roles

- **Orchestrator** (one): owns `dev`, claims and resolves tickets on `dev`, starts
  agents, merges branches in the order below, keeps the map's Progress current.
- **Implementation agent** (one per ticket): works only in its own worktree and
  branch, on the files its ticket names, and reports back with an `## Outcome` draft.
- **Human**: answers decision tickets (`Status: ready-for-human`) and approves PRs.

## Waves

A wave starts when every ticket it depends on is merged to `dev`. Tickets in one wave
run in parallel; the "touches" column is what keeps them apart.

| Wave | Ticket | Touches (main files) | Waits for |
| --- | --- | --- | --- |
| 1 | [ES-20](../edit-settlement/issues/20-preview-only-direct-edit-event.md) preview-only direct event | `application/src/session.rs`, `runtime.rs` test support, three preview call sites | — |
| 1 | [10](issues/10-export-leases.md) export leases | `web/crates/runtime/src/runtime.rs` export region (~3934-5850, ~6633-7231) | — |
| 1 | [11](issues/11-editor-state-tracer.md) Editor tracer | `web/src/presentation.rs` | — |
| 2 | [03](issues/03-native-test-runtime.md) native test Runtime | `web/crates/runtime/src/{lib.rs, runtime_test_stub.rs, edit_ticket.rs, in_process_support.rs}`, `part_placement.rs` tests | ES-20 |
| 2 | [06](issues/06-split-outline-lifecycle.md) outline split | `web/crates/layout/src/outline_lifecycle*` | ES-20 |
| 2 | [TCE-01](../typed-core-edits/issues/01-set-wiring-mode.md) `SetWiringMode` | `core/`, `web/crates/pcb/src/pcb_wiring/mode.rs` | ES-20 |
| 2 | [12](issues/12-editor-workspace-state.md) Editor workspace state | `web/src/presentation.rs` | 11 |
| 3 | [04](issues/04-resolution-constructors.md) resolution constructors | `session.rs`, then a sweep of every resolver in `web/` | 06, TCE-01 |
| 4 | [05](issues/05-pending-edits-module.md) `PendingEdits` + Matrix tracer | `runtime/src/{edit_ticket.rs, pending_edits.rs}`, `ui-shared`, `matrix_inspector_controller.rs` | 03, 04 |
| 5 | [07](issues/07-layout-panels-onto-pending-edits.md) Layout panels | `web/crates/layout`, `ui-shared/geometry_scripts.rs`, `web/src/presentation/` | 05, 06, 12 |
| 5 | [08](issues/08-parts-and-pcb-onto-pending-edits.md) Parts and PCB | `web/crates/parts`, `web/crates/pcb` | 05 |
| 5 | [09](issues/09-case-keymap-keycaps-library-onto-pending-edits.md) Case, Keymap, Keycaps, Library | those four crates | 05 |
| 5 | [TCE-02](../typed-core-edits/issues/02-mechanical-settings-patch.md) mechanical patch | `core/`, `web/crates/case/src/mechanical_settings*` | TCE-01 and a spec pass (`needs-info`) |
| 6 | [13](issues/13-cleanup.md) cleanup | docs, leftovers | 07, 08, 09, 10, 12 |
| human | [TCE-03](../typed-core-edits/issues/03-decide-outline-version-intents.md) outline intents | decision | TCE-01, 06 |

Why ticket 04 waits for 06 and TCE-01, though its dependency line names only ES-20: its
resolver sweep touches `outline_lifecycle.rs` and `pcb_wiring/mode.rs`. Running it after
them avoids rebasing a sweep across a file move. Ticket 07 waits for 12 for the same
reason (`web/src/presentation.rs`). TCE-02 and 09 both touch
`mechanical_settings_controller.rs`: merge 09 first.

Wave 1 and wave 2 tickets 10, 11 and 12 never block the critical path
(ES-20 → 03/04 → 05 → 07/08/09); give the critical path agents first.

## Claiming and resolving

The orchestrator, not the agents, writes ticket status on `dev`, so claims never race:

1. Set `Status: claimed` in the ticket and commit to `dev` ("Claim ticket NN: …").
2. Create the agent's worktree from that commit (below).
3. On merge, append the agent's `## Outcome` (commits, checks run, follow-ups), set
   `Status: resolved`, add one line under the map's Progress, commit to `dev`.

## Worktrees and branches

- One worktree per ticket, under `.worktrees/<ticket-slug>` (gitignored) or outside the
  repo. Branch name `deepening/<NN>-<slug>` (`edit-settlement/20-…`, `typed-core-edits/01-…`
  for the other efforts).
- The old `.claude/worktrees/*` are stale copies of earlier work; ignore them.
- Before reporting back, the agent rebases on `dev` and reruns its verification.
- Merge with `--no-ff` in wave order; within a wave, critical-path tickets first.

## Rules every agent follows

Give each agent its ticket path plus this list.

- Read `AGENTS.md`, `CONTEXT.md`, the ticket, its `Decision:` links and the ADRs it
  cites before editing. Line numbers in tickets are at `915d305c0`, orientation only.
- Stay inside the files your ticket names. If a fix needs another ticket's files,
  stop and report it instead.
- Shared files (`application/src/session.rs`, `web/crates/runtime/src/runtime.rs`,
  `web/src/presentation.rs`, `core/src/model.rs`, `core/src/lib.rs`): change them in a
  small commit of their own, first, so other branches rebase cleanly.
- Skills: `tdd` for build tickets (failing test at the deepened module's interface
  first; delete the shallow tests it replaces); `codebase-design` for interface and seam
  choices; `code-review` on the branch before reporting; `grilling` and
  `domain-modeling` only when the orchestrator relays a decision to the user.
- Git: stage explicit paths only; never `git add -A`, `git add .` or `git stash`.
- GitNexus: `impact` (upstream) before changing a shared symbol, `detect_changes`
  before each commit, `rename` for renames (the `gitnexus-*` skills). An empty or
  `UNKNOWN` result is unanswered, not safe: confirm with `rg`. The index is shared by
  all worktrees and lags their edits; the orchestrator reindexes `dev` after each merge
  (`node .gitnexus/run.cjs analyze --index-only`).
- Context7 for crate and browser API docs; `memsearch:memory-recall` for why an earlier
  ticket chose something. Ticket text and ADRs win over memory.
- Verification: the ticket's commands, then `python3 scripts/check.py` steps `lint`,
  `typecheck`, `test`, and `browser` for presentation changes. `browser` and the WASM
  build need headless Chromium and Podman; if the sandbox blocks them, say so in the
  report rather than skipping silently.
- Report: commits, checks run with results, anything skipped and why, follow-ups.
  The orchestrator turns it into the ticket's `## Outcome`.

## Agent prompt template

```text
You are implementing ticket <path> in the BoardStudio repo, in worktree <dir> on
branch <branch>, branched from dev at <sha>. Read AGENTS.md and
docs/plans/module-deepening/handoff.md ("Rules every agent follows") first, then the
ticket and every document it links as a decision. Implement the acceptance criteria
with the tdd skill, run GitNexus impact before shared-symbol edits and detect_changes
before each commit, and code-review the branch. Run the ticket's verification, rebase
on dev, rerun verification, and report commits, checks and follow-ups. Do not edit ticket status or the map.
```

## Human checkpoints

- TCE-03 needs the user before any outline intent build.
- TCE-02 needs a spec pass after TCE-01 merges (the orchestrator may draft it, the
  user approves).
- Any agent report that proposes changing a decision (`## Answer`, ADR) goes to the
  user, not straight into a commit.
