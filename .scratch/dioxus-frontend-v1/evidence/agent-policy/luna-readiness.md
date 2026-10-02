# Luna implementation readiness audit

Audit target: baseline `7b7b004e`, executable `f44a3d1b`, project plan baseline `c827c4e6`; source materials are `.scratch/dioxus-frontend-v1/{PLAN.md,EXECUTION.md,tasks.json}` and `issues/02-projects-shared-ui.md`, `issues/03-layout.md`. The planned graph has 62 stable task IDs; the following are dispatch subpackets beneath INT.1/F2.1/F2.3/F3.1, not changes to that graph.

## Readiness

Yes, the behavior specifications are sufficient for Luna to implement bounded private UI packets. They provide user-visible acceptance behaviors, React reference outcomes, error/cancel paths, responsive/keyboard requirements, non-mutation constraints, and explicit authority boundaries. F2.1 specifies existing `BrowserStore::list_documents`, `load_document`, active-project restore and `open_fixture`; F2.3 is presentation-only; F3.1 identifies Session selection/navigation/read-model ownership and explicitly says the missing work is the Dioxus tree projection/disclosure/adapter. `ReadModel` already has selection state and `Event` has `SelectParts`/`Navigate`; storage exposes listing/loading. The first wave has no demonstrated need to invent APIs.

The planning records are not yet sufficient as literal agent handoffs: add packet metadata at dispatch (do not edit the 62-task graph): exact packet ID under its parent task; immutable source baseline/commit; author-owned private file(s) and explicitly coordinator-owned files; exact public scenario/fixture/action oracle and required evidence; applicable checks; read-model/event inputs and callback outputs; refactoring-register handoff. `EXECUTION.md` already requires most handoff evidence and a refactoring observation at every workflow handoff. Findings register update is required for each packet: record evidence-backed finding(s), or explicitly state “No new refactoring takeaway observed” with reviewed scope.

## Dispatch packet breakdown

| Parent | Bounded packet / outcome | Actual dependencies and ownership | Luna tier |
|---|---|---|---|
| INT.1 | INT.1a: inspect current composition/session types and publish a private read-only shell contract (inputs, callbacks, scope/draft identity) sufficient for the three child modules; no feature UI. | Existing `Runtime`, `ReadModel`, `Event`; coordinator owns `presentation.rs`, `runtime.rs`, module wiring, global CSS/build wiring. Publish written contract before parallel feature edits. No public API. | Medium |
| INT.1 | INT.1b: install the private module mount points and preserve existing F1/F3a behavior; prove all three slots compose against the same runtime/session. | Depends INT.1a. Coordinator-only shared shell/module wiring. No general framework or API visibility change. | Medium |
| F2.1 | F2.1a: project saved-document summaries and demo cards/previews/counts/current marker; render loading/empty/populated and bad-preview fallback. | Depends INT.1 contract. Existing `store.list_documents`, accepted snapshot identity and fixture data; feature-private library component. Coordinator routes/mounts it. Browsing must not submit events or alter revision/history/selection. | Medium |
| F2.1 | F2.1b: search, clear/no-match and list failure/retry states, including stable state on repeated list load. | Depends F2.1a projection. Same list boundary; private UI state only. Public browser oracle includes no-match/clear and retry. | Low |
| F2.1 | F2.1c: open saved project and demo-as-editable-copy actions, loading/supersession feedback. | Depends F2.1a; existing `open_saved`/`open_fixture` actions; callbacks through coordinator-published contract. Verify active identity changes only on accepted open and stale opens follow Runtime policy. Does not take on F2.2 create/delete/archive. | Low |
| F2.3 | F2.3a: private panel preference model for mode/width and defaults/storage fallback; desktop pinned/auto-hide/collapsed state. | Depends INT.1 contract. Presentation-only; reference widths Objects 200–420px, Inspect 280–480px and fit with minimum canvas. Feature-owned controller/component; coordinator alone integrates shell/CSS. | Medium |
| F2.3 | F2.3b: desktop pointer/keyboard bounded resizing, capture/cancel and persisted width/mode. | Depends F2.3a; no domain service. Public oracle includes limits, keyboard increment, simultaneous panel/canvas constraint, reload persistence and storage-unavailable fallback. | Medium |
| F2.3 | F2.3c: compact breakpoint, drawer/scrim/close/Escape and focus return/hidden-focus exclusion. | Depends F2.3a and shell mount dimensions from INT.1b; coordinator integrates CSS/layout. Verify panel behavior leaves revision/history/camera unchanged. | Medium |
| F3.1 | F3.1a: project active-board `ReadModel`/document into board→matrix→row/column→key/component and outline/version/bridge tree nodes; independent disclosure state. | Depends INT.1. Existing canonical accepted document/scene/read model; pure private projection and local disclosure. No new engine hierarchy API. Feature owns private tree module. | Medium |
| F3.1 | F3.1b: tree-to-session selection adapter, replace/toggle/range interactions and canvas synchronization. | Depends F3.1a and INT callback contract; use existing `Event::SelectParts`/selection modes and canonical selection state. Rectangular key range is UI projection from current matrix membership/order; never mutate the document. If current read model does not expose enough stable ordering/membership, that is a concrete read-model gap to report to coordinator, not permission to widen visibility. | High |
| F3.1 | F3.1c: board/project scope transitions cancel gestures and clear only invalid selection; compact keyboard/focus behavior. | Depends F3.1a/b and existing `Event::Navigate` plus Runtime open/scope lifecycle. Compose public contract; verify switching board preserves still-valid scoped state and opening panels/visibility does not revise document. | Medium |

Luna high is appropriate for the selection adapter only after packet inputs are confirmed, since it combines hierarchy semantics, anchor/range projection and session synchronization. Astra should review that contract before implementation if the current public read model cannot prove membership/order, or if a proposal changes shared scene/read-model APIs. Do not hold F2.1/F2.3 on that question.

## Missing packet fields vs genuine design gaps

Missing at handoff granularity (not spec/design gaps): exact source baseline; owned source paths and private module names; packet-specific done oracle; fixture and interaction sequence; author/verifier/reviewer identity; output artifact/evidence path; affected check list; handoff/refactoring entry. The source/spec behavior is already sufficient to fill these with inspection at dispatch. Stable 62-task graph and its aggregate acceptance remain intact.

Genuine narrow integration questions, not broad design blockers:

- INT.1 contract must declare the concrete Rust types available to children and callback signatures for navigate/select/open; it cannot be finalized by prose alone. Existing shared composition ownership is clear.
- F2.1 summary/preview/count projection must establish which fields are already available from stored `ProjectDoc` and demo fixtures. This is implementation inspection; no service gap is established by the specs.
- F2.3 shell geometry/breakpoint and insertion point must match the existing shell. React behavior is known; only coordinator wiring is shared ownership.
- F3.1 must confirm current `ReadModel` and accepted document/scene expose the stable active-board matrix membership/order needed for rectangular range selection and outline/version tree labels. If so, private projection is enough. If not, state the exact missing value and smallest candidate adapter to coordinator; no API widening implied.

## Astra review-before-implementation risks

1. F3.1 selection mapping: have Astra review the proposed read-model-to-tree mapping and range anchor semantics before implementation. This governs edit-target safety and board scoping; verify same-board/matrix membership and disabled/hidden cell exclusion without duplicating core authority.
2. INT.1 shared shell contract: Astra reviews once before parallel dispatch, chiefly to check private ownership, single Runtime/session authority, stale-result scope, and no shared-file race. This is a short boundary review, not a milestone gate.
3. F2.3 keyboard/focus and pointer-capture state machine deserves Astra review after Luna’s packet patch, before integration, because errors can leave hidden content focusable or drawer focus stranded; keep the reference acceptance matrix unchanged.
4. Any packet that discovers a genuinely missing service/public contract, proposes visibility/API/scene schema changes, alters persistence semantics, or changes acceptance/performance/AT gates must stop at the concrete narrow proposal for Astra review. Ordinary private wrappers/composition using current public providers continue.

Other work can use Luna low/medium: search/error states, card composition, local panel state, and fixture-backed rendering. Keep existing frontend parity, public-browser evidence, paired React comparisons, keyboard/focus/axe and relevant build/check gates. F2.1/F2.3/F3.1 each dispatch and accept independently after INT.1; no full-F2/full-F3/full-milestone blanket lock. Record one findings-register disposition per handoff.
