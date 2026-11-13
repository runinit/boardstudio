# 008: Parity-gated dock retirement, proof preservation and shipped docs

Status: awaiting-decision
Type: build
Blocked by: [007](007-responsive-accessibility-acceptance.md)
Parent: [map.md](../map.md)
Owner: coordinator for shared routing/panel retirement, docs and final full gate

## Boundary

Only after all six workspace adapters and 007 pass, remove superseded shell use of DockLayout,
ObjectsPanel/InspectorPanel, PanelSettings/PanelMode, resize/pin/rail, dock-width persistence,
narrow Sheet/tabs, old Layout pill/snap-in-dock routing and dead CSS. Inventory live users
before deleting any shared primitive: unrelated dialogs/Sheet/Sidenav consumers stay.
No interim duplicate shell becomes a permanent fallback. Retain commands by remapping them,
not deleting them; obsolete stored dock values stop influencing UI, unrelated preferences stay.

Inventory every removed field/type by compiler errors (`cargo check`, then affected wasm/lint
checks) and Gortex impact/references. Map every retired/renamed test to the surviving case
carrying its assertions and original mutation. Baseline name changes ride the causing commit.

Rewrite DESIGN.md shell/dock/strip sections from shipped reality, remove its next-shell
transition notice, keep GLOSSARY's historical dock distinction and update architecture docs
where needed. Link ADR-0041. **Do not edit either frozen README or either HTML snapshot**, even
to change their status lines. No extra domain capability removal or legacy script retirement.

## Acceptance criteria

- [ ] Six inventory checklists and acceptance evidence complete before deletion; no old host path.
- [ ] Test retirement/name mapping reviewed; no weakened oracle, guard count or unexplained removal.
- [ ] Legacy preference upgrade behaves as specified, no broad local-storage clearing.
- [ ] Documentation describes shipped state without contradictory competing shell instructions.
- [ ] One combined Spec+Standards review, doc gate and full integrated gate on stable candidate.

## Proof map

- No old path — switch all six workspaces at desktop/narrow, panels closed/open → inspect DOM
  and operate utilities → no shell Sidenav/rail/Sheet/tab/resize handle; every domain destination
  still reachable · mounted · planned
- Upgrade — seed obsolete dock values plus valid Theme and unrelated preference → load and
  switch workspaces → no phantom width/pin, no read-dependent old shell, other values retained · mounted · planned
- Proof retention — list retired tests and original failures → run surviving case/mutation →
  same independent assertion fails; rename/name baseline is exact and documented · mounted · planned
- Final gate — reviewed stable candidate with no pending implementation deltas → run full
  `python3 scripts/check.py` → all required steps pass, receipt states executed vs blocked · packaged · planned

## Verification

Use [verification](../verification.md). Coordinator runs `cargo check` for native field
inventory and `check.py lint` for wasm branches, then `python3 scripts/check.py` as the final
full gate; it includes build and browser work. Do not replace it with repo/lint/test only.
Upgrade proof-gap mutation must fail no-phantom-width/retained-Theme assertions. Report any
blocked build verbatim after attempting it; never override owned build flags.
