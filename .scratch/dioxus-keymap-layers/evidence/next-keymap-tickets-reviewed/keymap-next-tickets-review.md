# Independent Astra review — next Keymap tickets

**Decision: revise the bounded items below before publication.** The three vertical scopes, existing Core command boundaries and canonical dependency graph are otherwise sound. No repository edits, Cargo or browser execution; source risks below are not claimed as reproduced bugs.

Reviewed integration HEAD `49b792eb11d94b8535f00fac46005f04c1e12c1a`; pinned React `5a472a9426e6e38993361da402cd4ec730feb369`.

Exact draft SHA-256:

- 03 binding: `c1e7f3475189c621269e5a365ac0629ab6d8f6c907c0f05889b69a66b581ff49`
- 04 macros: `32fd27cf71f7881643104540f43dae32766fc5b320f51a4b822fd47917da3481`
- 05 encoder/firmware: `56eebde238584e016c305af949cc627a592579c047fe6f47a2d8b3903fa2ba30`

## Findings

1. **Draft04 silently expands the reference macro controls.** `app/src/ui/KeymapPanel.tsx:49–62` renders tap/press/release/wait choice, then a plain Keycode field for each non-wait step. Editing that field submits a key-press binding; it does not expose KeyBindingEditor's non-macro behavior, layer, modifier or sticky selectors. Core permits more non-macro binding values, but that is not evidence those frontend controls already exist. Replace the requirement to reuse the *whole* F6K.2 editor with reuse of an appropriate private keycode/draft control while retaining the reference visible surface and preserved untouched accepted values. Include wait-switch default100ms and new non-wait step default tap/A. “Ordered steps” means preserved sequence plus existing add/remove/edit; do not infer new drag/reorder buttons from the text. No MoveStep command exists or is needed. Any wider UI would need an explicit intentional-change decision, not an invented parity claim.

2. **Draft05 needs complete push-gate provenance.** The leaf component accepts optional pushKeyId and uses `?? encoder.id + '/push'` with `!== null`, so an *injected undefined input at that boundary* can expose a synthetic control. However the actual pinned producer `app/src/main.tsx:129` maps `pushKeyId: item.pressKeyId ?? null`; module encoders in `createKeymapWorkspace.tsx:78` explicitly use null. The full public call path therefore already agrees with the canonical reported-push-only requirement. Retain the strict reported-ID gate, but describe undefined as a defensive boundary risk, not an observed reference/spec regression. No real red was run and no baseline defect is established. Core independently rejects unsupported `/push` IDs via electrical `press_key_id` membership (`core/src/keymap.rs:336–365`).

3. **Make Medium routing explicitly conditional for both03 and04.** Current `web/src/presentation/keymap/panel.rs` is read-only; its callbacks only select layer/key. There is no mounted Keymap edit controller/outcome callback in current integration. Runtime's operation-outcome observer is useful infrastructure, not proof that layer/key/field/editor-generation feedback is already supplied. Draft03 already permits escalation; keep that condition and default actual dispatch to Luna High until the exact accepted-source/Scope/operation-outcome seam is reviewed and mounted. Add the same condition to04: indexed step add/remove and late feedback must have a proven lifetime/source-generation policy before Medium is sufficient. Do not persist invented step IDs or widen APIs.05's High author is appropriate.

4. **Clarify Enter versus reference blur semantics in03/04.** Pinned KeyBindingEditor and MacroEditor commit text/numeric inputs on blur and contain no Enter handler. If the accepted Dioxus field contract intentionally makes Enter trigger that same blur/commit path, name that established interaction decision and keep the paired reference expectation distinct; otherwise preserve blur behavior. Deduplication is correct, but newly added Enter behavior must not be presented as already-observed React parity.

## Confirmed boundaries and gates

The62-parent graph is preserved.03 startsF6K.1, no extra acceptance join;04 startsF6K.1/F6K.2 with INT.2 acceptance;05 startsF6K.1/F6K.2 with F5.2/F8.2 acceptance. F3.1 stays F6K.1's separate selection join; it is not an added start barrier. Fixture authoring does not close real F5/F8/INT.2 integration.

Supported binding defaults, stable reference IDs, typed-over-legacy precedence and transparent/none remain accurate. Macro validation limits match Core. Preserve rejection/recovery when deleting a referenced macro: RemoveMacro does not rewrite bindings, and whole-map validation can reject a dangling reference. Core validation must remain authoritative; do not copy its parser or widen its crate-private keycode validator.

Encoder rotation, legacy PCB SetKeyBinding controls and qualified ZMK generation are coherent frontend slices through existing operations/providers. F5 owns selected-board electrical data and Wiring composition; root owns Runtime/export delivery; feature authors own disjoint private modules. The firmware path must keep all request/result/delivery freshness checks and actual generated archive evidence; visibility of Export is not readiness.

RF: no new refactoring takeaway observed. Existing RF-009 reconciliation remains; these findings do not justify an architecture rewrite.
