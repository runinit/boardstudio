# 02: Consolidate OwnedEdits<K> into ui-shared

Status: resolved
Type: build
Blocked by: None (can start immediately)

**What to build:** Replace the duplicated Case and Keymap controller `OwnedEdits<K>` helpers with a single shared owner-scoped edit-observation helper in `ui-shared` (alongside `PendingEditSignals`), so both workspaces share one implementation for owner-lifetime retirement, field Signal rebinding across owner changes, pending-request tracking/pruning, and reading bound field drafts.

## Acceptance criteria

- [x] A single `OwnedEdits<K>` implementation lives in `ui-shared` and is used by both the Case and Keymap workspaces; the local duplicate module files in Case and Keymap are deleted.
- [x] Owner-change retirement (`owner_changed` / `follow_owner`), mounted field binding and unbinding (`bind_field` / `unbind_field`), bound draft inspection (`bound_draft`), and pending/terminal tracking (`remember`, `is_pending`, `pending`, `has_terminal`, `prune`) preserve their existing behavior across both workspaces.
- [x] Unit tests cover owner replacement (retiring prior requests while rebinding mounted field Signals on the replacement helper) and `bound_draft` lookup, and `python3 scripts/check.py quick` passes.

## Comments

- 2026-10-19: Claimed for shared-checkout implementation. Scope: ui-shared, Case/Keymap imports and duplicate removal, and this ticket only. Parent owns all builds and final combined review; no Cargo, wasm-pack, or check.py builds will run in this worker.
- Implementation: moved the Keymap helper to `ui-shared::owned_edits`, retaining Case's `BoundField` alias and exposing the existing `bound_draft` method across crates. Updated all four controller imports and removed both duplicate module declarations/files. No production behavior or dependencies changed.
- Prepared three native tests at the approved public-method seams: owner replacement retires observations without cancelling Session edits and rebinds draft/failure Signals; bound draft lookup follows live values/rebindings and forgets unmounted fields; pending tracking preserves latest-key order and prunes terminal requests.
- Verification handoff: tests were written before the shared module was moved/exported, but RED/GREEN execution is deferred to the parent's single-target build coordinator. Run `cargo test -p boardstudio-web-ui-shared --lib owned_edits::tests`, then `cargo check --target wasm32-unknown-unknown -p boardstudio-web-case -p boardstudio-web-keymap`, and finally the acceptance command `python3 scripts/check.py quick` (redirect long output into `.evidence/`). Keep this ticket claimed until execution and combined review pass.
- Non-build evidence: `rustfmt --check` on the new helper/tests and scoped `git diff --check`; logs in `.evidence/cleanup-02-format.log` and `.evidence/cleanup-02-diff-check.log` (empty output means success).

## Verification evidence

- `cargo test -p boardstudio-web-ui-shared --locked --lib owned_edits::tests`: 3 passed; `.evidence/cleanup/owned-edits-tests.log`.
- `scripts/mutation-check.py`: owner-retirement, mounted-binding and bound-draft mutants all killed (3 killed, 0 survived, 0 compile failures); `.evidence/cleanup/owned-edits-mutations.log`.
- WASM Case/Keymap `cargo check --locked` passed; `.evidence/cleanup/owned-edits-typecheck.log`.
- Combined review: no blocking standards violations or spec findings. Recorded the three executed native test names in the retention baseline.

## Answer

Both workspaces now use `ui-shared::owned_edits::OwnedEdits`; the duplicate modules are deleted without changing settlement policy. The required quick check passed with `NODE_PATH=/home/chris/.cache/boardstudio-web-tools/node/node_modules BOARDSTUDIO_CHECK_JOBS=1` (416.4s): 2,024 native tests, all crate browser launches and all 77 page tests passed. Receipt: `.evidence/cleanup/quick-retry.json.md`.

The full check passed build, native tests and provider/drag smoke checks, but its parallel isolated setup-guide browser launch timed out before reporting a result. That same launch and all remaining browser/page checks passed in the serialized quick retry. Full-run receipt: `.evidence/cleanup/full.json.md`.
