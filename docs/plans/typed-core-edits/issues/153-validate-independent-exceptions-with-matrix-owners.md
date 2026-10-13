# 153: Validate independent exceptions alongside exact Matrix owners

Status: resolved
Type: build
Blocked by: [152](152-reconcile-authored-matrix-cells.md)
Parent: [19](19-reconcile-mirrored-halves.md)
Related: [111](111-integrate-mirrored-half-reconciliation.md), [150](150-prepare-matrixless-half-exceptions.md)

## Source-proven preparation gap

111's independent RST Part Local transition on a matrix-backed pair refuses
“Lifecycle ownership does not match actual target membership”. The intent adapter correctly
selects None for an independent item, but the pair-wide lifecycle context validates every
Matrix-owned child against one optional selected Matrix pair. One selected matrix also cannot
stand in for other matrix pairs. Evidence: main `.evidence/ticket153/context-audit.md`.

## Scope and existing contract

Prepare independent Part/EmbeddedCircuit exceptions on pairs with one or several Matrix
correspondences. Validate every association against its exact durable parent/real membership;
keep the requested operation's selected Matrix context separate from unrelated owners.
Reuse existing lifecycle/exception/projection kernels. Do not pick an arbitrary matrix, fabricate
IDs, weaken owner/board/pair validation, invent policy or widen helper visibility.
Own `core/src/mirrored_half/lifecycle.rs`, `lifecycle/tests.rs` and only necessary private context
changes in `reconcile.rs`. Keep operation vocabulary and production activation in 111. Root
assigns further paths before edits; shared source changes remain serial after 152.

## Acceptance

- [x] Runtime private regression first exposes the independent-item context refusal.
- [x] Independent Part and applicable circuit Local/Omit/Rejoin work beside one and multiple
      Matrix pairs, preserving current-source derivation, exact identity and explicit children.
- [x] Matrix-owned selections use their exact owner; unrelated matrices/pairs and local data
      remain unchanged. No arbitrary first-matrix selection or fake context is used.
- [x] Stale/ambiguous/wrong-board/incorrect actual membership refuses atomically; existing
      matrixless and matrix-backed lifecycle/membership tests remain green.
- [x] Independent Standards/Spec review, sensitivity mutations and required checks pass.

Root owns full checks/baseline/integration. 111 stays unmerged until this preparation is verified.

## Answer (2026-10-13)

Private lifecycle preparation now authenticates each Matrix companion against its exact durable
Matrix owner and actual source/mirror memberships, independently of the selected operation context.
Independent Part/circuit Local/Omit/Rejoin works alongside one or multiple Matrix pairs; the selected
Matrix child uses its exact association. Other Matrix state and explicit Local children are preserved.
Wrong owner, board, ambiguous identity and missing actual membership remain strict refusals.
No production activation, operation vocabulary, schema or helper visibility changed.

Four runtime-private regression failures were captured before the fix; the refusal control passed
as expected. Final focused reconciliation coverage is 221 tests, including two direct-helper
sensitivity controls. Independent Standards and Spec reviews accept the initial source and test-only
delta. All six final refined compiled mutations were killed and every file restored exactly. Earlier
survivors and evidence corrections remain recorded, rather than being counted as kills.

Root `repo test lint --update-native-baseline` passes with all 1,833 native tests executed; the baseline
adds exactly seven names. Evidence is in main `.evidence/ticket153/`, including immutable review
snapshots, captured final greens, mutation histories and full-check logs. Local integration and
post-merge checks follow; 111 remains an unmerged activation draft until 154 also completes.
