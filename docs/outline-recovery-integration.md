# Outline recovery integration

Recovered the outline implementation preserved during the September 30 cleanup
and reconciled it with `dev` at `d11497f0`. The original outline worktree remains
unchanged. The recovery source includes Stage 1 outline repair, protected gaps,
fixed versions, bridge references, located findings, export blocking, and the
subsequent shared snapping and inference-guide changes. Later outline stages in
`docs/design/board-outline-plan.md` remain future work.

## Preservation and reconciliation

- Snapshot: `.scratch/outline-recovery/20260930T175121Z` in the main checkout;
  54 modified and 48 untracked files, based on `6abf3eb9`. File hashes and modes
  were verified against the archive and the source was rechecked unchanged.
- Recovery commit: `698706c2`, retained on `codex/outline-recovery-preserved`.
- Integration used a separate checkout based on current dev. The initial shared
  worktree index writes were denied; standalone checkouts allowed progress until
  permissions changed. The original checkouts were preserved.
- Keep current dev's keycaps, keymap, firmware, onboarding, case workspace,
  preview lifetime, and workflow-specific export controllers. Extend
  `useOutlineEditor` with recovered versions and snapping instead of restoring
  the older inline Workbench implementation. Regenerate contracts from the
  combined Rust model, retaining keycap and outline edits.
- Pass active board outline readiness through the existing mechanical export
  workflow. Keep current dev's project export coordinator unchanged.
- Browser regression reproduced duplicate Snap controls while drawing. Unmount
  the inactive toolbar so only the drawing menu owns those controls. The retained
  snapping regression then passes. Configure the draft-position scenario with
  a 1 mm grid and geometry snapping disabled; retain its exact pixel assertion.

## Verification

- Unmodified current-dev Rust core baseline passed before integration.
- Combined Rust core: 324 tests passed.
- App: 468 tests passed across 81 files. The initial cold native archive-driver
  build caused one existing 5-second timeout; its isolated retry and then the
  complete app suite passed without changing the timeout.
- Generated contracts, runtime contract imports, app and CAD typechecking,
  repository checks, and native/WASM boundary parity passed. Parity covered
  17 core and 9 archive requests after rebuilding core WASM; the first parity
  attempt used copied stale WASM and was retained as a failed preparation step.
- Production core WASM and app builds passed. Renderer source is unchanged;
  existing renderer artifacts were reused. All 23 native CAD tests and 49 CAD
  JavaScript tests passed; CAD WASM rebuilt successfully. The initial direct
  CAD JavaScript run lacked its required native preparation driver; rebuilding
  the driver and running the package test sequence resolved those failures.
- All 16 outline browser scenarios passed after the reproduced menu fix.
- All 12 keycaps, saved-project, electrical handoff, and case-preparation browser
  scenarios passed, including real artifact exports and persistence.
- Agent-browser review opened an editable REVIUNG41 demo on its own origin,
  edited a perimeter coordinate, and confirmed an active fixed version with
  Generated still available and no browser errors. User storage was untouched.
- Logs and a screenshot are retained under
  `docs/design/evidence/board-outlines/recovery/`.

The full repository `pnpm check` umbrella and performance acceptance/soak suites
were not run for this recovery. These scoped checks do not close pending Phase 3
performance work, establish physical fit, or complete later outline stages.
