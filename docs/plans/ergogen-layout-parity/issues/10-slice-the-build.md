# 10: Slice the build into tickets

Status: resolved
Type: task
Blocked by: [01](01-name-the-carried-parameters.md), [02](02-ergogen-anchor-and-mirror-semantics.md), [03](03-parity-fixture-pipeline.md), [04](04-layout-kernel-order-of-operations.md), [05](05-this-column-splay-representation.md), [06](06-origin-edits-hold-keys-still.md), [07](07-matrix-anchors.md), [08](08-uniform-pitch-consumers.md), [09](09-inspector-and-toolbar-prototype.md), [11](11-mirrored-halves-rules.md), [12](12-relations-and-highlight.md), [13](13-format-reset-and-recovery.md), [14](14-starters-and-demos.md)
Map: [map.md](../map.md) · Decision: [ADR-0034](../../../adr/0036-matrices-follow-ergogen-point-layout.md)

## Question

When every decision ticket and every graduated fog ticket is resolved, write the spec
(`../spec.md`) and slice the build into dependent TDD tickets in this plan. Cover:

- the format reset;
- the parity fixtures;
- the Core kernel and edits;
- mirrored halves;
- anchors;
- consumers;
- UI;
- starters.

Each ticket carries acceptance checks and preserves the extended features in the inventory.
This ticket also closes the map.

## Answer

Resolved on 2026-10-23. The accepted decisions are consolidated in [spec.md](../spec.md),
with their later qualifications taking precedence. Aggregate anchors are explicitly
deferred by the user's additional scope answer; ADR-0034 and the map now qualify the
first-build parity claim instead of promising complete Ergogen points coverage.

The implementation is split into 13 dependent TDD build tickets, each with outcome,
first RED/GREEN tracer, subsequent behavioral acceptance, source seams and checks:

- [15](15-generate-parity-fixtures.md): pinned upstream fixtures and replacement inputs.
- [16](16-build-layout-kernel.md): staged pure kernel and comparison seam.
- [17](17-cut-over-format-and-model.md): coordinated model/caller cutover and format gates.
- [18](18-core-placement-edits.md): named placement, inverses and exact compensation.
- [19](19-anchor-layout-groups.md): anchor graph, Layout groups and Core edits.
- [20](20-reflect-linked-layouts.md): group reflection and actual-frame inverse edits.
- [21](21-exception-lifecycle.md): Local/Omitted, Rejoin, confirmation and removal.
- [22](22-update-layout-consumers.md): setup/Align/context/size/downstream consumers.
- [23](23-inspector-toolbar-and-handles.md): real Inspector, toolbar and handle behavior.
- [24](24-relations-and-highlights.md): Core-backed explanations and reference highlighting.
- [25](25-project-format-recovery.md): safe saved-project/import/switch recovery.
- [26](26-rebuild-examples-and-fixtures.md): exact examples and fixture integration audit.
- [27](27-end-to-end-acceptance.md): complete cross-crate acceptance and release gate.

Planning is closed; product work is not. Use one unshipped integration lane and isolated
test stores until the full cutover is accepted. The live schema change is atomic across
its callers, not a serde-only commit with compatibility shims. A staged kernel or private
lifecycle test passing does not establish application support. The prototype stays on
its throwaway branch; its untracked working copy remains untouched.

The current source/test map and supported command families were checked, including
the Core package name, explicit integration harness, browser subset selectors and
compiler-driven field inventory. Planning validation covers doc links, ADR index,
whitespace, the dependency DAG and contract coverage: 549 Markdown files, 34 ADRs,
27 linked issues with no dependency cycle and 15 as the sole build frontier. Read-only
Luna Fast/high consistency and same-board-schema reviews found no material issues.
These are planning checks; no Cargo, browser/provider build or application runtime
acceptance is claimed by this ticket.

## Comments

### 2026-10-23 — consolidation checks

All explicit decision prerequisites are resolved. Source-derived planning is in
progress; no production implementation or build has started.

- The consolidated spec must include `columnLayout.rotation` from ticket 05 and
  explicitly place Follow after the anchor-relative Origin/Rotation candidate, with
  ticket 11's transported filter on linked partners. These are assembly of settled
  decisions, not new policy questions.
- The remaining Aggregate anchors fog conflicts with an unconditional claim to
  implement every Ergogen points concept in ADR-0034. Obtain an explicit scope choice
  before declaring the spec/map complete; do not silently defer or invent aggregate
  storage, degenerate-case rules or UI.
- A pure kernel/types slice can be staged without changing the persisted model. The
  actual schema cutover is wider than serde: matrix parallel-vector readers, singular
  Layout ownership readers, scenes, edits, mirrors, storage and example/fixture paths
  must be coordinated. Do not publish a partially supported version-7 writer or use a
  compatibility migration to hide incomplete callers.
- Existing public test seams and supported commands were inventoried. Native package
  names differ from browser `--only` selectors. `check.py test` has no native package
  selector; use the existing Cargo interface for focused native RED/GREEN filters.

No Cargo/browser/provider checks were run during this inventory. The existing
untracked prototype working copy remains untouched.

### 2026-10-23 — aggregate scope accepted

The user answered “y”: defer average/intersection aggregate anchors from this first
build, explicitly qualify ADR-0034 and the spec's parity claim, and require a separate
future design rather than inventing aggregate storage/evaluation/UI in this slice.
This closes the only genuinely undecided scope branch found by the coherence check.
