# 009: Diagnostic gated-persistence adapter for packaged builds

Status: ready-for-agent
Type: build
Parent: [Draft-safe update adoption spec](../spec.md)
Blocked by: none — can start immediately (parallel with 008)

## What to build

The diagnostic-only gated-persistence adapter specified by
[Locate a deterministic packaged persistence hold](003-locate-packaged-persistence-hold.md):
a build-gated way for packaged verification to hold a packaged save genuinely
unanswered, then release it, fail it, or release it into failure, against the
real packaged store. It never ships as product behaviour. Exact diagnostic
A/B build flags, build-identity stamping and cleanup attestation are delivered
as part of this ticket.

## Acceptance criteria

- [ ] Arm hold: the save is entered with the intended name/revision, reports Saving, receives no acknowledgement and makes no durable change until released.
- [ ] Release completes the save and reports Saved at the intended revision; the reopened value is correct.
- [ ] Arm fail reports Failed, defers adoption as did-not-complete, and an explicit retry succeeds.
- [ ] Hold-then-fail is a single armed mode: releasing the hold enters failure atomically (not two re-armed gates).
- [ ] Persistence is judged by the real store's completion/abort acknowledgements, not request-success callbacks, save labels or worker labels.
- [ ] The adapter is diagnostic-only and build-gated; a normal packaged build contains no armed gate. Diagnostic build identity is distinguishable and attested, and disposable fixtures/databases are cleaned up with the cleanup attested in the completion record.
- [ ] Focused checks for touched crates pass; evidence logs are stored per the plan's evidence conventions.
