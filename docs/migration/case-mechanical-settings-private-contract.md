# Case mechanical settings private UI contract

This note describes the private Dioxus presentation seam in `web/src/presentation/mechanical_settings.rs`. The module renders only the F7.4a construction/settings slice and emits typed intents. It does not own accepted project state, persistence, geometry, mechanical normalization, closure-mount initialization, or generated PCB clearances.

## Parent inputs and events

`MechanicalSettingsProps` receives a bounded presentation projection:

- `identity`: editor lifetime, monotonically advanced Case scope generation, full application `Scope`, captured snapshot token/revision, active board ID, and configuration board ID. A generation change remounts the field subtree even when navigation returns to an equal `Scope`. The numeric draft key intentionally excludes snapshot token/revision so an unrelated saved revision does not discard unchanged field text.
- `values`: optional displayed values for the matching current configuration. It contains only represented controls, never a mutable `MechanicalConfiguration` or `ProjectDoc`.
- `profiles` and `layers`: immutable `Rc<[...]>` rows already selected/resolved by the parent. The parent should memoize these bounded projections by their source identity so a Runtime repaint passes a cheap shared handle rather than rebuilding rows. Finding rows contain only ID, severity and message; they do not retain unused target IDs.
- `findings`: an immutable `Rc<[...]>` projection containing only displayed ID, severity and message fields.
- `mismatch`, `editable`, `disabled_reason`, and `feedback`: current parent admission/display state. A mismatch is read-only and routes through `on_show_configured_board`.

`on_request` emits `MechanicalSettingsRequest { identity, request_id, field_id, patch }`. `request_id` is monotonic for the mounted component. Root must validate the captured identity and its own Runtime/instance-selection/readiness/preview/gesture/saved-revision guards at admission and immediately before applying a patch. Configure (`Enable`) is distinct from update/disable and can be offered when no configuration is projected; root must validate a current board and compute its defaults at admitted Configure time. Update and Disable require a configuration that belongs to the active board. A mismatched configuration must never fall through to Configure.

`feedback` is a bounded immutable list with one current row per request identity. Each `MechanicalSettingsFeedback` echoes the full captured identity, request ID, and field ID with Pending/Saved/Failed plus optional message. Replace a request's Pending row with its terminal row; do not append duplicate Pending and terminal rows for the same identity because a field consumes the first match. Root owns terminal operation classification. Every emitted request must receive an exact response: an admission-rejected stale/busy request receives Failed immediately, without replacing or settling the already admitted operation's response. Retaining the active Pending response beside such a rejection lets both fields consume their own result. In particular, expected revision numbers alone do not establish Saved. A successful result can advance the current accepted snapshot while retaining the captured request identity for the field acknowledgement.

`on_select_layer`, `on_show_finding`, and `on_show_configured_board` are presentation/navigation events. Layer selection is not a Session part selection. Findings and board navigation do not emit configuration patches.

## Field draft behavior

Dimension inputs retain their local text across unrelated token/revision movement. They reset on Escape, a matching Saved acknowledgement, or a changed accepted value for that field. A matching failure leaves the text correctable and displays the source error. The child matches feedback against the full submitted request plus field owner, so an earlier acknowledgement cannot settle later typing. Numeric blur submits; Enter triggers the native input blur and that blur owns the one submission; Escape restores the accepted value. Parent admission remains authoritative and must disable or reject controls while a write is pending without replacing its admitted request slot.

## Source-backed behavior and open integration gates

The controls mirror the represented construction choices and dimensions in `app/src/ui/MechanicalAssemblyPanel.tsx`. They intentionally emit field intents only. Root/controller work must separately preserve the source's plate/profile/foam coupling and normalize represented part-process methods, materials, and thicknesses on every patch. The private UI does not implement those transformations.

The corrected source review also found behavior outside the currently accepted Rust helpers: non-gasket Configure/updates can initialize closure mounts from fresh contour suggestions, and `withClosureClearance` updates generated PCB parts/definitions and board membership. This component neither fabricates nor approximates those changes. The ownership and atomic-transaction policy for those effects remains an explicit integration gate; this UI contract does not claim full React parity or close F7.4/INT.2.

No public API, shared catalogue, schema, dependency, CAD algorithm, or Runtime operation is added by this component.
