# PCB Wiring source handoff

## Source slice and ownership

The private Wiring Inspector and Editor-lifetime query are implemented in source commit `626668b7d911ca5d73f2233f9197a13d6db67afe`; a reviewed follow-up is in progress. Current working-file hashes are:

- `web/src/presentation/pcb_wiring.rs`: `d0430d2fc38384629b0c2ca4bfe76e803283ad308114411dee24a1ebce414b22`
- `web/src/presentation/pcb_wiring/controller.rs`: `3edcbb27595cb4a19a0d77a0dd0dffce0feb790ce85ddd6faff26220adcde8db`
- `web/src/runtime.rs`: `4b108e5370aa41989a1fe393b8eb0e6ff1807d365a71a099a7f059aaa0f5c688`

Root owns `presentation.rs`, feature registration, CSS, compiler/build and browser verification. The leaf owns only immutable accepted-source projection. `PcbWiringInspectorProps` is `{ source: PcbWiringSource, resolution: PcbWiringResolution, on_resolve: EventHandler<()>, on_edit_board_wiring: EventHandler<()> }`. `PcbWiringSource::new` takes the accepted snapshot, a board `Scope` with `instance_id: None`, first active selected part ID (if any), and the current `Runtime::electrical_preview_executor_epoch()`.

## Mount join

In the Editor parent, register the private module and call `use_pcb_wiring_controller(runtime.clone(), version)` unconditionally, before conditional returns. The hook refreshes its event callback through Dioxus `use_callback`; do not replace it with per-render `EventHandler::new`. For each accepted render, get the current Runtime scope, normalize its `instance_id` to `None`, and build the source with the accepted snapshot, ordered Session selection's first real ID, and current private executor epoch. Mount `PcbWiringInspector` in the existing right Inspector when the accepted workspace and selection route to PCB Wiring. Wire `on_resolve` directly to the mount's guarded retry handler. The switch's “Edit board wiring” action must use the existing selection adapter to clear Session selection/context and render the same board-level Inspector; it must not mutate `ProjectDoc` or create a second selection store. Retain the parent controller across selected-part and workspace changes so the board plan does not refetch for a switch click.

The controller tracks accepted document/board identity, Session epoch and Core executor epoch. The Runtime request additionally verifies accepted source, exact worker `Rc`, executor epoch, request/reply ID and plan board/revision/instance around the await. This private owner does not prove the root callback's admission guards; root must retain the existing scope/generation validation on selection clear and Inspector actions.

## CSS and visible join

The leaf uses `m1-pcb-wiring`, `m1-pcb-switch-wiring`, `m1-pcb-wiring-breadcrumb`, `m1-pcb-wiring-controller`, `m1-pcb-wiring-mode`, `m1-pcb-wiring-pin-summary`, `m1-pcb-wiring-readiness`, `m1-pcb-wiring-section`, `m1-pcb-wiring-assignment`, and `m1-pcb-wiring-empty`. Style with existing theme variables. The panel content should own vertical scrolling inside the existing Inspector, while the breadcrumb, controller/mode summary, disabled-with-no-candidate Resolve action, diagnostics with per-finding severity under an alert region, assignments, and switch terminal list remain readable at the supported narrow Inspector width.

## Verification and open joins

`rustfmt --edition 2024` and `git diff --check` passed for the owned source. No Cargo/native/WASM compiler, browser, screen-reader, or paired React interaction was run by this source author. Root owns those checks. Required public follow-up covers first-ready/accepted-board refresh, no-candidate diagnostics/action state, real switch named-terminal context, executor restart/reopen stale-result suppression, narrow Inspector scrolling, and confirmation that returning to board Wiring clears only the existing Session selection/context. F5.2/F5.3, INT.2 and parent acceptance remain open. No performance pass is claimed; accepted generator scene allocation measurement remains a separate unmeasured RF-001 item.

## Refactoring finding handoff

No new RF ID is warranted. Retain the bounded observation under RF-001, RF-006 and RF-009. The new `Runtime::resolve_electrical_preview` repeats the existing mechanical resolver's local accepted-source, exact worker-`Rc` and executor-epoch admission checks around an async query. This bounded helper is needed for this feature; do not add a generic query framework here. After parity, evaluate one shared, tested Runtime read-query lifecycle boundary with explicit scope/source identity, worker epoch, cancellation/stale-reply policy and no duplicate Session or document authority. RF-006 records the board-level electrical `Scope(instance_id=None)` separately from physical Case/mechanical instance projection; that normalization must not leak into instance-scoped mechanical operations. RF-009 retains exact source/build/fixture accounting and the open paired acceptance gate.
