# Frontend UI wave — independent Spec review, 2026-10-02

## F6K.4b — source APPROVED; acceptance remains bounded

Reviewed implementation `32dd07c0a5179fa4c995a1da668365b4e11df2d3`, evidence `e4366bdb096f509d18127408f3a0f67f505a6764`, F5 handoff/controller and firmware projection in `f6k-firmware-positions-20261002`. Authority: issue `07-pcb-firmware-position-editor.md` SHA-256 `d86689bf058b1fb8dd6db4bbda476569c7d759aa19ab69a4ec4a25e690ace857`.

No blocking source finding. Choices/order/defaults match React; F5 supplies real plan IDs and scoped bindings. Admission checks captured UI scope, generation, accepted token/revision, executor epoch and current plan; exact outcome registration precedes SetKeyBinding submission. Core remains owner of first-layer compatibility synchronization.

Reused RESULTS.md: paired SW1 A assignment, Undo/Redo/reload, delayed persistence and aborted-write failure/recovery; author reports native/WASM/fmt/strict Clippy/release checks on merged `039cc612962821e0d8b64c26b05256d7beca5ffd`. I did not rerun those builds/browser journeys. Evidence does not establish every auxiliary-push, typed-keymap, scope-switch, compact/theme/focus acceptance case. F5.2, F8.2 and F6K.4 joins remain open.

## Numeric07 — historical source CHANGES REQUIRED; repair awaits independent approval

Reviewed `8bc16bc11803266254a1471ee6094d2f7dee5b7d`, `79e9fe666213b24f40976d953be795305b51f29a`, `232289dbf1a0d82187991ba9c03cb12e41e25c5a`. Issue07 SHA-256 `5e736b7239d0ac7f47b723c8bd3b9e47e3ed0d084e8e207e6779bd7bc1416bd9`.

- **P1:** controller lines 149–184 ignores request snapshot_token/revision. A stale callback after another-field commit remains admissible when its field baseline is unchanged; violates issue07 “Revalidate … token/revision … synchronously.”
- **P2:** lines 515–528 drops pending OutcomeSlot before terminal arrival on scope/generation change; violates “already submitted operation settles only its captured owner” while hidden.

Coordinator-authorized repair `f0328aee61b61d19cf37497ee523c5a32882de0c` addresses both. I authored it and cannot independently approve it. Red: 3 expected lifecycle failures; green: 5 lifecycle tests, 19 library + 41 page tests; strict WASM all-target Clippy/fmt/diff-check passed. Logs `/tmp/frontend-ui-wave-numeric-{lifecycle-red,lifecycle-green,wasm-clippy}-20261002.log`. No public browser acceptance claimed; pointer tools remain deferred.

## Align — historical source CHANGES REQUIRED

Reviewed `533f9a5051c1d1cd62e1dccdf1233a8285c8945e` plus shared-join.patch against issue08 SHA-256 `eb2586d81883c19d7576af8d994eabeda309045d6eb5c2f6d5537ec1149b26ea`.

**P2:** controller lines 154–166 overwrites selected_reference from an empty non-Layout projection, losing an eligible chosen reference while hidden; contradicts the root-lifetime preference contract. Author notified; corrected source review pending. Central Standards owns the separate effect-loop finding. Envelope/anchor policy matches inspected React source. Candidate paired acceptance remains open.

RF handoff: retain RF-005/RF-009. No new refactoring takeaway beyond existing captured-owner and domain-policy boundaries. No shared ledger or parent graph changed.

## Next-frontier planning receipts

Keycaps F6C.2: **planning/capability APPROVED** after source reconciliation against `app/src/ui/KeycapPanel.tsx`, existing `SetKeycapBoard`/`SetMatrixKeycaps`, and Editor-owned keycaps settings lifecycle. Revised spec SHA-256 `f25776c673a7404eefe0369f7b04c2e6490b4360dd6cfbe0c79b1fb8cdf12265`; board child `64e2f8aa800bc59ccd6e2f2182089dae57ec50384376885e98e41029f5e281eb`; matrix child `ece19c86e95d8049cb237d9284306aa2804e5de9aae4c6a94923d7c631d00844`. The revisions require browser-proven React synthetic/Dioxus native event timing and unconditional root scoped admission/exact outcome ownership. This resolves the original packet ambiguity; no runtime acceptance implied.

MatrixSetup F3.2a origin-only capability: **planning/capability APPROVED** for `/tmp/frontend-run/layout-matrix-setup-contract-20261002.md`, SHA-256 `218796ec696c5193036704075254b7779737a9fe7feaf403c1bb99c8925706c1`. Historical `ad4354ba...` incorrectly required the guide remain visible; React `Workbench.tsx:130–131,622–633,1177` instead preserves stored open/Layout state while temporarily hiding the guide during the form. Corrected packet preserves this behavior. `createWorkbenchPlacementActions.ts:29–42` proves immediate origin SetMatrix authority; eight catalogued presets and private mapping require exact source fixtures and Core/session tests. Core's 4096 bound is matrix cells (rows×columns), not total generated switch/companion count; author advised. Parts-library pointer ghost remains deferred. Existing private catalogue access must remain private; no public/member widening follows.

Both acknowledgments preserve the 62-parent graph, original parent joins, paired public browser gates and coordinator-owned RF ledger. No new RF ID proposed.
