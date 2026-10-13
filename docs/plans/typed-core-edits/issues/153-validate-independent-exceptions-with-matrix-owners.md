# 153: Validate independent exceptions alongside exact Matrix owners

Status: ready-for-agent
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

- [ ] Runtime private regression first exposes the independent-item context refusal.
- [ ] Independent Part and applicable circuit Local/Omit/Rejoin work beside one and multiple
      Matrix pairs, preserving current-source derivation, exact identity and explicit children.
- [ ] Matrix-owned selections use their exact owner; unrelated matrices/pairs and local data
      remain unchanged. No arbitrary first-matrix selection or fake context is used.
- [ ] Stale/ambiguous/wrong-board/incorrect actual membership refuses atomically; existing
      matrixless and matrix-backed lifecycle/membership tests remain green.
- [ ] Independent Standards/Spec review, sensitivity mutations and required checks pass.

Root owns full checks/baseline/integration. 111 stays unmerged until this preparation is verified.
