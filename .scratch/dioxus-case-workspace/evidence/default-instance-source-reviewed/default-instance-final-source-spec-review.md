# Independent final source Spec review — default-instance repair

Reviewed working six-file delta against HEAD `49b792eb11d94b8535f00fac46005f04c1e12c1a`, approved `/tmp/frontend-run/default-instance-selection-contract-review.md`, repair draft04 and React selection producers. No edits, Cargo or browser execution during this review.

**One bounded contract gap remains; the core repair otherwise matches the approved source policy.**

**P2 — explicit choice of the already effective fallback lacks a public event route.** `presentation.rs:1143–1159` correctly remembers explicit `Some(id)` before the executor's equality no-op. However, `presentation/objects.rs:141–149` only emits that request from native select `onchange`. Rechoosing the already selected option does not change its value. The approved proposal explicitly requires “an explicit click on a fallback turns it into a real preference.” Example: choose A2 on boardA; visit boardB, which automatically displays B1; explicitly choose that displayed B1; return toA. React's Case assembly row onClick invokes `onSelectCaseInstance` even for the active instance (`app/src/ui/useWorkbenchTree.ts:63–80`), so the preference becomes B1 and A falls back to A1. The candidate control may retain A2 instead. This is a concrete source-level reachability risk, **not a claimed public reproduction**. Before clearing this contract point, provide the actual public path/evidence that records an unchanged selection or implement the smallest explicit-activation route; do not treat focus/opening the menu as a choice. Root/author were notified while source remained frozen.

All other inspected requirements pass at source level:

- Resolver uses validated explicit preference then first matching instance in document order; no mechanical, flip, name or topology filtering. Epoch/document matching prevents same-ID reopen leakage. Automatic fallback never replaces remembered preference, allowing board return/removal/Undo restoration.
- Existing guarded navigation retains full Scope/generation/membership checks and scoped pointer/gesture/context/anchor cleanup. Case configured-board navigation uses target-board preference/fallback. Selection does not edit document, configuration, revision, history or camera.
- Automatic reconciliation gates Ready, saved-current, no preview, no Session gesture and no local Drag; fresh Scope/token/revision/generation/preferences are checked before dispatch. Pointer end/cancel, Escape and workspace cleanup retry signal covers local pan completion, where Session gesture notifications alone are insufficient. Existing pointer acquisition and ownership guards are retained.
- Pending Case rendering and live Case settings/generation/body admission prevent unresolved canonical-scope actions. Existing scoped body feedback, defaults and persistence logic are preserved. Layout/Keymap/Keycaps still consume canonical geometry.
- Added APIs are private binary composition/helpers, not widened existing library/Session/Core/provider members. Native-only main.rs test wiring is authorized. The four tests cover helper policy plus real Session opening/save admission; they do not prove actual UI navigation, pointer timing, stale effects or the same-value-choice path.

Fresh production original import→Case, explicit-choice/order/reopen/Undo, busy/gesture/camera/history neutrality and delayed feedback checks remain required. Original red and normalized-main diagnostic green are distinct from repaired acceptance. Parent graph/joins remain unchanged. RF: no new architecture finding; retain existing RF-006/009 accounting.

Exact reviewed SHA256:
- presentation.rs `0545037b9e8ec96599a3f4b9d00e88e4c7bdf237831666cff1399d2c7b818e57`
- instance_selection.rs `28dd03ba57c403940ae3ab9d9e7c11c039c5ce3d933d2c9587f3a72c6a07a2b9`
- objects.rs `0aa614633b3050fa301fc3b2d8521f021471c543cf3a9c0ad484908317ea739f`
- case_controller.rs `bc5ce15ce817090f46801c69057fd698fda741cef520a694475db44e852f7601`
- cad_presentation.rs `16151541526ef3f193737bddd1a0a9775a781c4d28d5114d9a3f033b31d0a580`
- main.rs `67d11f57b415964a1b55e26ecb53c6142fe29a624374fb11997db2fb8016e85e`

## P2 rereview — cleared at source level

Reviewed `/tmp/frontend-run/default-instance-current-activation-repair.md` and exact changed Objects/CSS after the first review. **P2 is resolved; no remaining material Spec source findings.** Only Case renders the matching instances as native `type=button` controls in a labelled group. `aria-pressed` comes from actual Session selection, and every click calls the guarded `Some(id)` wrapper even when already pressed. Native Enter/Space activation shares that route. No focus/menu-open side effects, synthesized change events or preference mutation during rendering were added. Other workspaces retain the existing native selector and approved canonical-option correction; no-instance boards remain unchanged. CSS is limited to these controls, uses existing light/dark tokens and provides visible focus and compact target sizing.

Replacement objects.rs SHA256 `39954b6247b2f0529bd26a3206ad199afea060597242da2feb35a4011176d9d5`; added CSS review SHA256 `0e4781c4886d33c600f37c3623e80f30883eb5b07b5d4323249fec79146255e3`. Other five source hashes above were rechecked unchanged.

This clears the implementation for fresh public verification, not behavioral acceptance. Required same-current-fallback regression must use actual pointer or Enter/Space activation: A2 explicit→B fallbackB1→activateB1→returnA=A1; the passive-visit control must preserveA2. Do not substitute programmatically dispatched select onchange. Original import→Case and all previously listed scope, lifecycle, gesture, persistence and history gates remain open. No source edits or Cargo/browser runs in this rereview.
