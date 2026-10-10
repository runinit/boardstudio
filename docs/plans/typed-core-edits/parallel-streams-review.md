# Parallel streams decision and rollout

Status: scheduling approved by the user on 2026-10-10; per-stream readiness and exact leases still required.
Reviewed baseline: integration `ad1c2804`. Approval: "Does 19 have to run alone? If not.. spin up the other streams."
This decision does not change active ticket105's scope or waive independent verification.

## Recommendation

Start with three implementation streams: B's serial mirror work, A's isolated remaining browser-test work and F's
qualified component work. Keep the six persistent coordinators, with C/D/E preparing their later work. Separate a
file/edit lease from the global command window. Builds, browser runs, commits and read-only shell commands remain
sequential; timing measurements require the machine alone.

The original human handoff said **19 runs ALONE**. The user approved narrowing this blanket restriction to actual Core
file/API ownership and activation/timing windows. Disjoint A/F edits may overlap B preparation, while preserving B's internal
order. Root grants exact audited leases after readiness gates; approval of scheduling is not blanket path ownership.
Existing acceptance, independent verification, generated-output ownership and current 105 scope stay binding.

## Initial streams

| Stream | Work that can overlap after the gates | Required gate before implementation |
| --- | --- | --- |
| B: mirror Core | Continue 105, then the existing 19 child chain serially | Finish 105's unchanged Luna implementation and fresh Sol verification; root releases each next child. |
| A: browser tests | Bounded remaining 37 test-local conversions, potentially parts/keycaps first | Audit exact test/helper paths, production dependencies and mutations; exclude B and E/F shared seams. Root grants an isolated worktree and exact paths. |
| F: components | Existing-part qualification and demonstrably missing component assets | F selects/reviews the BOM, inventories existing entries, defines pin/pad/drill/side/model evidence and exact asset/test paths. Shared registration/controller/firmware work gets one owner. |
| C/D/E: preparation | C measurement protocol; D literal/coverage inventory; E adapter/control contracts; early 114/40 decisions in B | Source reads use scheduled windows. Plans do not release blocked implementation or alter dependencies. |

A's broad scope in [37](issues/37-sleeps-layout-library-ui-page.md) is an inventory, not a lease. Give shared
wait/render/idle helpers one owner; distinguish suite-local conversions, negative-idle proof and the carried PCB recovery
regression. Any necessary production change escalates before editing. Preserve 37's open acceptance and
[38](issues/38-merge-duplicate-browser-tests.md)'s dependency on complete 37.

F owns hardware selection and its fresh planning review in its existing chat/worktree. Qualification does not inherently
depend on 19/20, but final paired fit and native board integration do. Reserved IDs 118–127 are not finalized tickets or
implementation permission. A selected alternate controller can require electrical profiles, firmware and export support;
do not describe that as merely adding a footprint. Native Sofle recipe work retains 20+44 prerequisites.

D can prepare a separate literal-only maintenance slice after auditing the files named in
[35](issues/35-test-maintenance-cuts.md). Producer-dependent assertions wait for their producers. The user already approved
the audit recommendations in [24](issues/24-test-audit.md#phase-2-user-approved-the-audits-recommendations-2026-10-10);
do not request that approval again for the accepted list. R1 recipe-reflection tests stay until 23. Root alone regenerates
the boundary/demo goldens; transcript/baseline coverage changes and final 35/24 acceptance retain their checks.

## Later Core branching, conditional on evidence

Keep the existing linear child dependencies for now. After verified 105, B may assess whether its natural private seam
supports separate 106 matrix/part, 107 hosted/electrical and 108 outline preparation. Before proposing a dependency change,
review contracts for supplied correspondence identities, allocation, owner exceptions, deletion closure, changed IDs,
collisions and atomic refusal. Assign disjoint domain/test files; one B integration owner retains shared module wiring,
`core/src/mirrored_half.rs`, `core/src/mechanical/halves.rs`, association helpers and test registration.
No public visibility widening or abstraction exists solely to create parallelism.

112/113 inverse translation may separate only after canonical operation/association contracts stabilize. Keep
115 transaction, 116 Open repair, 117 preview/history and 111 final activation ordered where they share
`core/src/lib.rs` and engine ordering. Parent 19 resolves only on complete original acceptance, not on preparatory tickets.

## Remove decision bottlenecks early

- **114:** prepare the constraint design before its predecessor finishes. Cover the operation inventory, identity ownership,
  linked/local endpoints and rotated-axis representability. Unsupported cases and their effect on parent acceptance must be
  resolved; the ticket remains `needs-info` meanwhile.
- **40:** settle pair-default mode versus item tweaks, future-inheritor zero tweak, mixed pair/item payloads and transitions
  away from mode 3 without silent loss. E can prepare control/test contracts; its documented implementation gates remain.
- **C12:** its header requires check-speed 11 and typed-core 34; lane docs additionally require 38. Establish whether 38 is a
  workload dependency. Either add/explain that header prerequisite or use a frozen workload and audited file lease. This
  review does not waive either current gate or authorize timing before 11.

## Queue and conflict protocol

The [central registry](lane-registry.md) remains authoritative. After approval, each proposed edit lease records the
coordinator, worktree/base, exact paths, shared symbols/interfaces, dependencies and excluded shared/generated files.
Root grants a separate bounded command window with command purpose, checks and explicit release condition.

An agent with an approved edit lease may reason and edit disjoint files through non-shell tools while another lane runs an
ordinary check. It must not edit that check's checkout or inputs. Shell reads, mutations, commits, builds, browser runs and
verification probes use the single global foreground command window. No release is inferred from inactivity. Timing windows
pause other agent activity, browser sessions, graph work and refreshes on the machine.

Before editing, compare the lease with current branch changes and bound GitNexus callers/processes. Before committing,
compare complete `detect_changes` with the lease. Escalate overlaps, API changes, stale prerequisites, unexpected dirt and
unassigned output to root before contested edits. Root broadcasts the resolution and owns all index refreshes/merges.
File separation alone is not proof of behavioral independence. UNKNOWN/stale/empty or capped graph coverage needs source
confirmation; partial/truncated analysis is unresolved.

Index ownership alone does not serialize the detached Stop-hook refresh described in
[gitnexus-index.md](../../agents/gitnexus-index.md#background-refresh-behaviour). Before activation, root must make automatic
refreshes obey the same exclusive command window: either establish foreground-only refresh for the scheduled batch with
the automatic path prevented from launching, or add and verify a lease-aware guard. Keep the window until the actual
analyzer process exits; a detached launcher returning is not release. Confirm no refresh remains before granting another
command or timing window. Freeze the indexed checkout and its inputs during refresh. This is an activation gate, not a
claim that the existing hook already honors leases; any required tooling/configuration work gets an explicit owner.

Use the [workflow's Luna-first effort policy](../../agents/agent-workflow.md#models-and-who-does-what-the-same-in-codex-and-claude)
and fresh independent Sol acceptance verification. Run affected checks per ticket and one full default check per integration
batch; preserve any ticket-specific full before/after gates. Avoid duplicate investigations and full checks per small slice.

## Evidence and activation checklist

Luna medium inventoried frozen plan dependencies. A Sol 6.1 max planner read the relevant workflow/tickets and made a bounded
graph-first query against `boardstudio-tce-matrix`; it established symbol locations, not complete independence. Persistent
A–F coordinators supplied provisional lane analyses. No new component choice or A/F path lease is certified by this review.
No active 105 extraction source was inspected or changed. No implementation tests were run for this planning review.

Activation checklist:

1. Independent Sol 6.1 xhigh review completed; both material findings closed.
2. User approved narrowing the original19 exclusivity rule and starting independent streams.
3. Complete A/F's focused file/API/qualification audits, assign worktrees and exact leases, and update parent 19/future child
   execution clauses plus map/registry and `parallel-lanes.md` atomically. Update that page's lane descriptions/current-release
   paragraph so its old 19-alone/F-planning rules do not contradict the approved release. Preserve active scopes; no retrospective
   widening of 105. Finish its original implementation and fresh verification session before adding concurrent code streams.
   That session ended at `3a7ba67b8` with incomplete acceptance: companion/stabiliser derivation still needs extraction.
   Keep that revision in B; it is not a dependency of audited independent A/F work. 106 still requires resolved105.
4. Foreground-only refresh established for this batch: root preserved its registration as
   `.git/gitnexus-orchestrator.suspended-01a1272d-9690-7342-a509-877cc412ec02.json`; the only installed Codex GitNexus Stop
   command is the registration-gated wrapper. A Stop-event probe returned silently with no registration, and process
   inspection found no analyzer/refresh. Keep manual refreshes in root foreground windows. Restore registration only
   after all parallel command/edit leases end and no running process remains; preserve any intervening registration.
5. Release at most B+A+F initially. Reassess after one verified integration batch; add a fourth code stream only when leases
   and the check queue show capacity. C/D/E preparation can continue throughout without premature code activation.

Independent planning review: fresh Sol 6.1 xhigh identified two P2 gaps: detached automatic indexing bypassed the proposed
window, and activation omitted contradictory `parallel-lanes.md` instructions. The reviewer confirmed both closed by the
corrections and approved the proposal as pending the user's decision and activation gates; no code or hardware was verified.
