# 008: Draft-owner registry and Geometry scripts tracer at both Update Now checkpoints

Status: ready-for-agent

## Progress (implementation, 2026-11-07; NOT resolved — awaiting coordinator review)

Implemented on branch `fix/ui-integration-recovery` (tracer: registry +
Geometry scripts adapter + both checkpoints). Mounted GREEN at seam S1;
packaged proof belongs to 010. Stopping-point report filed:
`update-draft-safety/008-implementation.md` (notes) and `.evidence/`
`008-red-toast.log`, `008-green-notifications.log` (28/28),
`008-check-quick.log` (exit 0), `008-check-build.log` (exit 0),
`008-check-browser.log` (runtime + ui-shared, exit 0),
`008-check-webbin.log` (28 executed, 0 failed).

- Registry: `web/crates/runtime/src/draft_owners.rs` (new), Runtime-owned
  `RefCell<DraftOwnerRegistry>`, polled synchronously by
  `update_safety_inputs` (both checkpoints + post-takeover reload share it).
  RAII guard; explicit `retire_draft_owner` on unmount/scope change;
poll-prune drops dead entries (one-time `web_sys::console::warn_1`
  diagnostic per entry — the named log/surface) and stale scopes silently.
- Gate: `UpdateSafetyInputs::unresolved_drafts`,
  `UpdateBlocker::UnresolvedDraft { area, remedy }`, message
  `"{area} has an unapplied edit. {remedy}, then choose Update Now again."`
  Geometry copy: area `Geometry scripts`, remedy
  `Apply the script edit, or cancel the edit in the form`.
- Geometry adapter compares the live JSON-encoded draft against the
  live-projected accepted encode (mount-default snapshot when no accepted
  script exists) and the key's in-flight submitted value (new
  `PendingEditSignals::submitted_draft`). No JSON-draft refactor (005).
- Keyboard scope (mounted): tab stops + focus + Space/Enter
  non-interception proven at both checkpoints; synthetic key events do not
  synthesize native clicks (repo-proven button contract), so trusted-key
  activation belongs to packaged runs (010+).
Type: build
Parent: [Draft-safe update adoption spec](../spec.md)
Blocked by: none — can start immediately

## What to build

The complete draft-safety contract end-to-end for one owner: a Runtime-owned
registry of live draft owners that both Update Now checkpoints (update toast
and header notice) poll synchronously, plus the Geometry scripts owner's
registry adapter. Choosing Update Now with a valid unapplied Geometry scripts
Draft value defers adoption with a cancel-only message naming the Geometry
scripts area and its remedy (Apply the script edit, or cancel the edit in the
form). Adoption proceeds only once no live owner reports an unresolved draft
and no save is pending or failed. The Geometry scripts editor's JSON-encoded
draft stays unchanged; the adapter compares the live encoded draft against the
owner-canonically encoded baseline per admission key.

## Acceptance criteria

- [x] Registration is RAII: an owner registers on mount and retires on genuine unmount or scope change; hidden, keep-mounted and portal-retained owners keep blocking.
- [x] Unresolved state is judged per admission key against the live-projected baseline (or mount-default snapshot where no accepted value exists), using owner-canonical equality; reverting to the baseline clears the key.
- [x] Poll-prune drops dead entries rather than treating them as blockers, with a one-time diagnostic whose log/surface is named in the ticket's completion record.
- [x] RED first: the mounted equivalents of the confirmed draft-loss assertions fail for the expected reason before the fix, then pass (GREEN) — report at the RED and mounted-GREEN stopping points before proceeding.
- [x] Mounted tests cover: valid explicit-submit draft defers; realistic invalid draft defers (including never-submitted, DOM-unmarked); trusted pointer and Tab/Enter/Space at both entry points; Cancel preserves exact draft value and focus; resolution only via the owner's own Apply or Cancel/Reset; landing updates the baseline, failure/refusal never clears, newer typing re-raises, an older settlement never clears newer input.
- [x] Deferral copy follows the settled per-key label+remedy rule; no injected invalid marker, save label or worker label is used as evidence.
- [x] Focused quick/build/browser checks for touched crates pass; evidence logs are stored per the plan's evidence conventions.
