# 08: Dead dependencies

Status: resolved
Type: research
Parent: [CI wall time map](../map.md)

## Question

Which dependencies can go without losing protection, itemized with per-item evidence?

## Pointers

- Scale: 18 workspace members plus `cad/wasm`, 427 locked packages.
- Leads, not conclusions: the unused `renderer/vendor/cgmath-0.18.0` fallback (patched to
  `cgmath-next`), full-catalogue loading, repetitive mechanical-settings field definitions.
- Method (no new tooling required): `cargo metadata` build-graph analysis across all features
  and cfgs — an item qualifies only if no feature/cfg combination references it.
- Removals graduate as follow-up tasks with per-item justification and a green full gate;
  lockfile churn lands after 03 scores the warm baseline and re-measures after.
- Deliverable: the itemized list with evidence per item, so removals need no further research.

## Answer

16 removable dependency edges across 8 crates, all with per-item evidence in
`docs/research/2026-10-29-dead-deps.md` on branch `research/dead-deps` (pushed, no PR) — but
predicted wall-time saving is **~zero**: `cargo tree -i` proves every candidate package stays
reachable through another workspace edge (e.g. runtime pulls `sha2` unconditionally), so no
lockfile package and no compile unit leaves any graph. Value is hygiene/audit-surface, not the
wall. Lead corrections: the `cgmath-0.18.0` fallback is already gone; full-catalogue loading and
repetitive mechanical-settings fields are real costs but not Cargo dependencies (re-scoped as
follow-ups); documented a `rename`-with-dash grep trap (`dioxus-primitives` is used) and a
kept-list (`three-d`/`lyon`/`i_overlay`, `libm`, core `js-sys`) so nothing gets re-researched
blindly.
