# Lane registry and coordination

Root owns this registry. Read the central copy at
`/home/chris/01_Projects/ts-boardstudio2/.worktrees/tce-matrix/docs/plans/typed-core-edits/lane-registry.md`
before planning a dispatch, changing files, requesting checks or handing work to a verifier. A lane's branch copy
may lag. Confirm current worktree status and index freshness in the granted command window; a snapshot is not a lease.

[Parallel-streams decision](parallel-streams-review.md) is approved: B Core work remains serial; isolated A/F streams
may receive separate edit leases after exact audits and the original 105 verification session ends. A Core-only acceptance
gap does not block audited independent work. Approval does not grant unlisted files or commands.

## Coordinators

The user requires a persistent chat for every lane. Each coordinator owns its lane's planning, bounded tickets,
Luna implementation and fresh Sol 6.1 xhigh verification sessions. Use the Luna-first effort table in
[agent-workflow.md](../../agents/agent-workflow.md#models-and-who-does-what-the-same-in-codex-and-claude)
for new sessions; active sessions keep their settings and scope.
The coordinator does not implement or verify work it delegates. Root owns integration, maps, shared file leases,
command scheduling, batch checks and every GitNexus refresh. Reuse the listed chats rather than starting duplicate owners.

Root: **Orchestration Chat**, `01a1272d-9690-7342-a509-877cc412ec02`.

| Lane | Coordinator chat / ID | Worktree and branch | Current work / gates |
| --- | --- | --- | --- |
| A browser tests | Coordinate browser test lane A / `01a1276a-18ec-76d0-83ed-82d289372b07` | `/home/chris/01_Projects/ts-boardstudio2/.worktrees/lane-web-sleeps`, `codex/lane-web-sleeps`, base `f2d180c6`; own target/CAD cache | Child128 claimed at `cf148ed6`; full30-test list and five-test pre-edit baseline passed. Luna high may edit leased tests without shell; next check window requested separately. 38 still waits for complete37. |
| B mirror Core | Coordinate mirrored-half reconciliation / `01a12740-01ce-7f21-ba6b-39982e260d5c` | `/home/chris/01_Projects/ts-boardstudio2/.worktrees/lane-mirror-core`, `lane/mirror-core` | Fresh 105 verification at `3a7ba67b8` found companion/stabiliser reuse gap. Bounded Luna high revision claimed at `2aac1855`; test draft ready, red command now granted. Production mutation waits meaningful red and root grant; no106. |
| C check speed | Coordinate check speed lane C / `01a1276a-2f0a-7d01-b42e-9a16249e8fbc` | Not assigned | HOLD. 11 needs the machine alone; 12 follows A38; 13 is a human decision. |
| D test cleanup | Coordinate test cleanup lane D / `01a1276a-40fa-77d0-935d-c68fefdb47ca` | Not assigned | HOLD. 35 after mirror/recipe integration; 24 closes after cleanup. Root alone regenerates goldens. |
| E web adapters | Coordinate mirrored half web lane E / `01a1276a-5631-7b73-a8d3-9d864b586f5f` | Not assigned | HOLD. 21, 41, 22 follow their B dependencies. Companion placement excludes independent parts. |
| F boards | Coordinate BoardStudio board adaptations / `01a1273f-b992-7133-b8ed-1eae6bbc90da` | `/home/chris/.codex/worktrees/sofle-adaptations/ts-boardstudio2`, `codex/sofle-adaptations` | START selected-BOM planning review, then exact component plans118–127 and qualification/asset leases. Existing catalogue first; native recipes retain dependencies. Root write/read window required. |

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

Current queue: A128 initial setup is released at `cf148ed6`, before baseline/list compilation or source edits.
F119 claim is committed at `a5bc4359`; its drawing download/render window is released with independently viewed manufacturer pixels.
A128 baseline/list is released: exactly30 names, five leased tests pass. Root's tiny source read supplied F the v2 SW26 pose
from `content/layouts/sofle-layouts.json:178` (96.7,133.41,300 degrees). B105 red exited101 solely for four missing
`companion_assembly` calls, without incidental fixture errors, and its command window is released. Root grants B bounded
non-shell production extraction in its two leased Rust source files; this is structural missing-helper evidence, not proof of
a behavior bug. A/F may edit disjoint leased tests without shell. Root commits this tracking update, then F119 focused
checks and A128 probes/checks request subsequent command slots; B targeted green follows when ready.
All heavy checks need explicit grants. B's Core-only acceptance
gap stays in B and does not block A/F. Root publishes explicit handovers; exact readiness leases remain required.
C/D/E implementation remains gated. Non-shell disjoint edits may overlap ordinary checks in another checkout.
105's source lease is `core/src/mirrored_half.rs`, narrowly necessary existing helpers in `core/src/matrix/layout.rs`,
creation tests in `core/tests/mirrored_half.rs`, and permitted private `core/src/mirrored_half/` child modules.
Ticket 105's own tracking file is writable by its implementer/verifier. Other paths require root assignment first.
The current105 revision is narrower: only `core/src/mirrored_half.rs`, `core/src/mirrored_half/derive.rs`,
`core/tests/mirrored_half.rs` and105 tracking. No additional child file, layout/model/lib/API/golden/runtime activation edit.
Extract private companion/tweak/stabiliser value derivation; preserve existing Part helper evidence and31/39 contracts.
Reserved A128 lease: `web/crates/keycaps/src/keycaps_settings.rs`, restricted to `queued_settings_tests` and local
`mounted_legend`; plus `docs/plans/typed-core-edits/issues/128-keycaps-queued-settings-waits.md`. No production/shared-helper,
Core, catalogue, model or golden edits. Audit found LOW impact for `mounted_legend` (four test callers) and UNKNOWN for a
WASM test entrypoint, confirmed in source; this is not complete graph proof of independence. Keep five test names/assertions,
all30 keycaps test names and an explicit bounded negative-idle proof for owner retirement. Parent37/38 gates stay intact.
Root permits test-local `Probe`, `host` and their initializer sites within that same module for causal retirement evidence.
This does not approve hook-order/version assumptions or a render-counter proof; implementation and fresh verification must
establish causal sufficiency. No production/shared helper is added to the lease, and baseline/list precedes source edits.
F's non-shell documentation lease covers `docs/plans/typed-core-edits/audit/sofle-adaptation-hardware-candidates.md`
and `docs/plans/typed-core-edits/issues/118-sofle-adaptation-hardware-contract.md`; Luna medium has drafted them.
F's audit confirmed existing `footprints/src/generators/ceoloide/switch_mx.rs` stays read-only. F may draft
`docs/plans/typed-core-edits/issues/119-mx-hotswap-socket-qualification.md` without shell. F119's granted source lease is only
new `footprints/tests/mx_socket_qualification.rs` plus119 tracking: Luna medium starts with its explicit setup window,
then non-shell test writing may overlap A's ordinary checks in a separate checkout. No geometry edit is authorized.
Official drawing comparison remains to be implemented. Electrical/firmware/stack qualification remains open.
Maps, this registry, CONTEXT, ADRs and generated goldens remain root-owned. `scripts/check.py` remains C-owned,
with edits requiring C's explicit active lease. F owns its report/component plans only after a root write window.

During this batch the root Stop-hook registration is preserved under the suspended filename recorded in the decision.
Automatic indexing is prevented; root refreshes manually in foreground command windows and freezes the indexed checkout.
Every handover waits for the actual analyzer exit and confirms no refresh remains. Re-enable automatic registration only
after leases end, preserving intervening registration changes. File edit leases never authorize changes to another check's
checkout or inputs; every shell command, including reads and commits, uses the single global window.

## Integration and graph snapshot

2026-10-10: integration at `cc3bdf8f`; full default check passed source batch `12a6784d` in 813.3 s
(70 presentation tests, zero failures; 15 CAD browser checks). Later changes are orchestration documentation.
B is at `3a7ba67b8`; `boardstudio-lane-mirror-core` was current at `1edddb94` before verification, now behind its evidence commit.
Integration uses `boardstudio-tce-matrix`; root refreshes after this documentation commit.
F has synced source `64510cb0` and candidate report `b9e93fab`; its bound index is `boardstudio-ts-boardstudio2`,
behind these documentation commits. F source research must state this limitation and confirm unresolved graph answers.
Each actual dispatch rechecks HEAD/status/freshness; coordinator prompts do not certify future state.

Sofle hardware selection/review and eventual component implementation stay in F. Root retains shared scheduling/merges.
F's delegated-selection report amendment and118 draft await safe inspection/checks/commit in a command window.
The entire 19 parent remains open until final child activation and complete acceptance; 114 still needs constraint decisions.
Ticket 37 remains partial, and 38 is blocked. These distinctions survive every handoff.
