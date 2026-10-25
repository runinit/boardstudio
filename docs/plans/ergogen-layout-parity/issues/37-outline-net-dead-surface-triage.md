# 37: Outline and Net dead-surface triage

Status: claimed
Type: build
Blocked by: None (all dependencies resolved; the live outline and net surfaces are established, so each dead variant can be judged against them)
Parent: [map.md](../map.md)
Map: [map.md](../map.md)

## Goal

Triage every Core edit variant with no live surface caller, and for each one
either wire it to a real Inspector or canvas surface or delete it, so no
dead outline or net variant remains in the Core contract.

## First RED / GREEN slice

Fail a mounted test: the triage table in this ticket names every dead
variant in the SelectOutline family, SetNet, and the superseded mechanical
and outline operations, with a wire-or-delete verdict for each, and follow-up
work proves the table is empty by the ticket's close.

## Subsequent acceptance

- [ ] Each SelectOutline-family variant is either reachable from a real
      surface with acceptance proof or deleted with dependents updated.
- [ ] SetNet is either reachable from a real surface with acceptance proof or
      deleted with dependents updated.
- [ ] Each superseded mechanical and outline operation is either reachable
      from a real surface with acceptance proof or deleted with dependents
      updated.
- [ ] No dead Core variant remains: enumeration of the Core edit contract
      shows every surviving variant has a live caller or an explicit test.

## Checks

Triage table plus mounted browser tests for each wired surface (or deletion
proof with dependents updated); affected quick/browser checks.

## Triage (2026-10-25, worktree `t3-18fae8db`, base `d772c70a2`)

Wiring-audit pointer was unavailable in-session (no audit doc found under
`docs/`; `matrix-layout-dependents-inventory` and the ergogen comparison do
not name these variants), so the table below is enumerated directly from the
Core contract (`core/src/model.rs::EditOperation`) against live callers in
`web/crates/` + `web/src/` (`grep -rln`). `core/src` rows are Core-internal
mirroring/lifecycle/test dependents, not live surfaces.

| Variant | Live surface caller? | Core-internal dependents | Verdict |
| --- | --- | --- | --- |
| `SelectOutline` | None in `web/` | `core/src/lib.rs`, `core/src/outline_versions.rs` | Delete (with dependents) — live outline surface uses `ApplyOutlineEdit` (`layout/src/outline/adapter.rs`, `hook.rs`, `inspector.rs`) |
| `RenameOutline` | None in `web/` | `core/src/lib.rs`, `core/src/outline_versions.rs` | Delete (with dependents) — version rename lives in `OutlineSettingEdit::RenameVersion` via `ApplyOutlineEdit` |
| `RemoveOutline` | None in `web/` | `core/src/lib.rs`, `core/src/outline_versions.rs` | Delete (with dependents) — no live remove-version surface found |
| `CopyOutline` | None in `web/` | `core/src/lib.rs`, `core/src/outline_versions.rs` | Delete (with dependents) — no live copy surface found |
| `SetNet` | None in `web/` (`pcb_wiring` does not submit `SetNet`) | `core/src/lib.rs`, `core/src/mirrored_half/inverse/hosted.rs` | Delete (with dependents) — no live net Inspector/canvas path found |
| `SetMechanical` | None in `web/crates/`; stale `web/src` refs only (`layout_component_inspector.rs`, `presentation.rs`) | `core/src/lib.rs`, `mirrored_half/reconcile/prepare.rs`, `mirrored_half/lifecycle.rs` | Delete (with dependents) — superseded by `ApplyMechanicalEdit` (`case/src/mechanical_settings_controller.rs`) |
| `SetOutline` | None in `web/crates/`; stale `web/src` refs only (same two files) | `core/src/lib.rs` | Delete (with dependents) — superseded by `ApplyOutlineEdit` (`layout/src/outline/`) |

Live counterparts confirmed: `ApplyOutlineEdit` (adapter/hook/inspector +
`outline_lifecycle_tests.rs`) and `ApplyMechanicalEdit`
(`mechanical_settings_controller.rs`, `mechanical_pair.rs`, session tests).
`SelectOutline`-family + `SetNet` + superseded `SetMechanical`/`SetOutline`
are the only dead families in this ticket's scope; the full 50-variant
contract was not re-audited here.

## Progress

- Triage table above is the RED artifact: it names every dead variant in the
  ticket's scope with a delete verdict and the live counterpart for each.
  No wire-to-surface candidate was found — every family is superseded.
- Deletion (model.rs variant removal + `lib.rs` arms + `mirrored_half`
  inverse/reconcile/lifecycle + `outline_versions.rs` dependents + baseline
  test-name updates) requires model.rs type changes and is therefore STOPPED
  per task rules ("if so, stop and report"). No code changed; ticket stays
  claimed for parent review/approval before any deletion lands.
- New test names: none (no code changed; no baseline edits).
