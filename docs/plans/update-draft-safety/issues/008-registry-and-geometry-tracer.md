# 008: Draft-owner registry and Geometry scripts tracer at both Update Now checkpoints

Status: ready-for-agent
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

- [ ] Registration is RAII: an owner registers on mount and retires on genuine unmount or scope change; hidden, keep-mounted and portal-retained owners keep blocking.
- [ ] Unresolved state is judged per admission key against the live-projected baseline (or mount-default snapshot where no accepted value exists), using owner-canonical equality; reverting to the baseline clears the key.
- [ ] Poll-prune drops dead entries rather than treating them as blockers, with a one-time diagnostic whose log/surface is named in the ticket's completion record.
- [ ] RED first: the mounted equivalents of the confirmed draft-loss assertions fail for the expected reason before the fix, then pass (GREEN) — report at the RED and mounted-GREEN stopping points before proceeding.
- [ ] Mounted tests cover: valid explicit-submit draft defers; realistic invalid draft defers (including never-submitted, DOM-unmarked); trusted pointer and Tab/Enter/Space at both entry points; Cancel preserves exact draft value and focus; resolution only via the owner's own Apply or Cancel/Reset; landing updates the baseline, failure/refusal never clears, newer typing re-raises, an older settlement never clears newer input.
- [ ] Deferral copy follows the settled per-key label+remedy rule; no injected invalid marker, save label or worker label is used as evidence.
- [ ] Focused quick/build/browser checks for touched crates pass; evidence logs are stored per the plan's evidence conventions.
