# 010: Packaged A→B draft-safety proof

Status: ready-for-agent
Type: build
Parent: [Draft-safe update adoption spec](../spec.md)
Blocked by: [Draft-owner registry and Geometry scripts tracer at both Update Now checkpoints](008-registry-and-geometry-tracer.md), [Diagnostic gated-persistence adapter for packaged builds](009-diagnostic-gated-persistence-adapter.md)

## What to build

The packaged tracer proof: the confirmed Geometry scripts A→B draft loss
becomes a permanent regression that passes in real packaged builds. Build A
runs with a real waiting build B; a valid unapplied Geometry scripts draft
defers Update Now at the toast entry point and, after toast dismissal, at the
header entry point. After the draft is resolved and the save settles, Update
Now loads actual build B and reopens the exact saved project at the intended
value and revision. Report at the packaged-GREEN stopping point.

## Acceptance criteria

- [ ] The packaged regression assertion from [Establish a real-editor draft-loss probe](002-establish-real-editor-draft-loss-probe.md) is RED before the tracer fix is present in the packaged build and GREEN after, with A/B build identities recorded.
- [ ] Toast entry point: trusted pointer and trusted Tab/Enter/Space defer; Cancel preserves the exact draft text and the running build stays A.
- [ ] Header entry point after toast dismissal defers the same draft the same way.
- [ ] Held-persistence legs: with the diagnostic adapter armed in A, Update Now defers as save-in-progress with no durable change; after release and acknowledgement, adoption proceeds; the same gate is consumed again after takeover in B. Fail and hold-then-fail legs defer as did-not-complete and recover via explicit retry.
- [ ] Exactly one packaged new-draft-during-takeover leg, reusing the tracer scenario's shape: a draft created while adoption waits defers Update Now again.
- [ ] Loaded-B proof: the loaded document and the served binary both identify B (not manifests or labels), the navigation is the adoption reload, and the reopened project holds the intended value at the intended revision.
- [ ] Sole-client/other-tab, offline-asset continuity and subpath-isolation legs inherited from the parent adoption acceptance pass; failed-install and no-new-build negative controls do not adopt.
- [ ] Disposable fixtures, profiles and databases are cleaned up; evidence logs are stored per the plan's evidence conventions. This ticket does not close the parent adoption umbrella.
