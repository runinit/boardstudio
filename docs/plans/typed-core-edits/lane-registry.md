# Lane registry and coordination

Root owns this registry. Read the central copy at
`/home/chris/01_Projects/ts-boardstudio2/.worktrees/tce-matrix/docs/plans/typed-core-edits/lane-registry.md`
before planning a dispatch, changing files, requesting checks or handing work to a verifier. A lane's branch copy
may lag. Confirm current worktree status and index freshness in the granted command window; a snapshot is not a lease.

## Coordinators

The user requires a persistent chat for every lane. Each coordinator owns its lane's planning, bounded tickets,
Luna 6 high implementation and fresh Sol 6.1 xhigh verification sessions. Planning research uses Sol 6.1 max.
The coordinator does not implement or verify work it delegates. Root owns integration, maps, shared file leases,
command scheduling, batch checks and every GitNexus refresh. Reuse the listed chats rather than starting duplicate owners.

Root: **Orchestration Chat**, `01a1272d-9690-7342-a509-877cc412ec02`.

| Lane | Coordinator chat / ID | Worktree and branch | Current work / gates |
| --- | --- | --- | --- |
| A browser tests | Coordinate browser test lane A / `01a1276a-18ec-76d0-83ed-82d289372b07` | Not assigned; root will create/sync a dedicated worktree | HOLD. 37's delivered slice verified at `fb2fe7bb7`; remaining acceptance stays open. 38 waits for complete 37. |
| B mirror Core | Coordinate mirrored-half reconciliation / `01a12740-01ce-7f21-ba6b-39982e260d5c` | `/home/chris/01_Projects/ts-boardstudio2/.worktrees/lane-mirror-core`, `lane/mirror-core` | 31/32/39 integrated. Reserve 105 only, then fresh verifier. 19 child chain runs alone; no automatic release of 106. |
| C check speed | Coordinate check speed lane C / `01a1276a-2f0a-7d01-b42e-9a16249e8fbc` | Not assigned | HOLD. 11 needs the machine alone; 12 follows A38; 13 is a human decision. |
| D test cleanup | Coordinate test cleanup lane D / `01a1276a-40fa-77d0-935d-c68fefdb47ca` | Not assigned | HOLD. 35 after mirror/recipe integration; 24 closes after cleanup. Root alone regenerates goldens. |
| E web adapters | Coordinate mirrored half web lane E / `01a1276a-5631-7b73-a8d3-9d864b586f5f` | Not assigned | HOLD. 21, 41, 22 follow their B dependencies. Companion placement excludes independent parts. |
| F boards | Coordinate BoardStudio board adaptations / `01a1273f-b992-7133-b8ed-1eae6bbc90da` | `/home/chris/.codex/worktrees/sofle-adaptations/ts-boardstudio2`, `codex/sofle-adaptations` | Research/review only. Best-fit Sofle BOM selection and missing-footprint onboarding delegated by the user; existing catalogue first. IDs 118–127 reserved. No implementation/check lease yet. |

F's earlier `.worktrees/lane-boards-review` branch is committed and integrated; it is frozen, not another active writer.
Other existing worktrees and old ticket chats are not active lane leases. Preserve them and confirm ownership before reuse.

## Dispatch and conflict escalation

1. Coordinator reads this central registry and the ticket dependencies, then proposes the absolute worktree/base commit,
   exact paths, shared symbols/APIs, generated outputs and required commands. Root grants a bounded file and command lease.
   A new lane gets its persistent chat and isolated worktree before implementation; a blocked lane may plan while holding.
2. In the command window, confirm path/branch/status, inspect overlapping edits and the latest integration changes.
   Run GitNexus impact against the uniquely named index before symbol edits. Compare callers/processes with other lanes'
   leases, including shared Core dispatch/model/reflection, catalogue assembly/recipe and browser adapter seams.
3. Escalate an overlapping file, shared API/behavior change, stale base that invalidates a prerequisite, unexpected dirty
   tree or unassigned generated output to root **before editing the contested scope**. Include paths/symbols, base/HEAD,
   index freshness/risk, affected lanes, proposed owner/order and whether checks are running. Root resolves the lease and
   ordering and informs affected coordinators; work outside the contested scope may continue only within its existing lease.
4. Before commit, run complete bound `detect_changes` and compare its actual changed files/processes to the lease.
   HIGH/CRITICAL impact must be reported before edits; UNKNOWN/empty/stale results need source confirmation.
   Partial/truncated analysis is unresolved. An index can report capped call/process coverage even when the delta is complete;
   an absent caller/flow is not proof of safety. Only root refreshes and communicates the new indexed base.
5. Implementer reports `implemented (awaiting verification)`, exact commit/paths, checks, limitations and changed shared seams.
   Coordinator starts a fresh verifier only after the implementer has stopped and the next command window is granted.
   Verifier independently reproduces acceptance/mutations; resolve only the complete ticket scope. Return a handoff with
   commit IDs, clean/dirty status, evidence, open decisions and released leases. Root merges and updates this registry.

Coordinator awareness comes from the shared registry plus root's handoffs, not from independent guesses about other chats.
Root broadcasts a changed lease, dependency, integration base, index refresh or conflict decision to affected coordinators.
Coordinators keep their own lane detail in ticket Outcomes; only root edits this shared registry and maps.

## File and command leases

One foreground shell command runs globally at a time. A lease grants a bounded sequence of commands, all sequential;
it never grants concurrent builds, browsers or timing work. Request a window from root and report explicit release after
the last command. Root waits for release before granting the next owner. Web research or planning without shell commands
may continue. Read-only shell commands also use the window. See [parallel-lanes.md](parallel-lanes.md) for shared file owners.

Current reservation: **B105 implementation, then its fresh verifier**, activated only by root's explicit START handoff
after registry bookkeeping completes. F's prior bounded source-read window is released. A/C/D/E have no command lease.
105's source lease is `core/src/mirrored_half.rs`, narrowly necessary existing helpers in `core/src/matrix/layout.rs`,
creation tests in `core/tests/mirrored_half.rs`, and permitted private `core/src/mirrored_half/` child modules.
Ticket 105's own tracking file is writable by its implementer/verifier. Other paths require root assignment first.
Maps, this registry, CONTEXT, ADRs and generated goldens remain root-owned. `scripts/check.py` remains C-owned,
with edits requiring C's explicit active lease. F owns its report/component plans only after a root write window.

## Integration and graph snapshot

2026-10-10: integration at `bc4dbc7d`; full default check passed source batch `12a6784d` in 813.3 s
(70 presentation tests, zero failures; 15 CAD browser checks). Later changes are orchestration documentation.
B is synchronized at `3570e8d9`; `boardstudio-lane-mirror-core` was confirmed current before dispatch.
Integration uses `boardstudio-tce-matrix`; root refreshes after this documentation commit.
F has synced source `64510cb0` and candidate report `b9e93fab`; its bound index is `boardstudio-ts-boardstudio2`,
behind these documentation commits. F source research must state this limitation and confirm unresolved graph answers.
Each actual dispatch rechecks HEAD/status/freshness; coordinator prompts do not certify future state.

Sofle hardware selection/review and eventual component implementation stay in F. Root retains shared scheduling/merges.
The candidate report predates the delegated-selection instruction; F will update its selection gates in its next write window.
The entire 19 parent remains open until final child activation and complete acceptance; 114 still needs constraint decisions.
Ticket 37 remains partial, and 38 is blocked. These distinctions survive every handoff.
