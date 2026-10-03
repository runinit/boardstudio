# Post-port refactoring takeaways

**Living register, started 2026-10-02 at migration baseline `c827c4e6`.** The user requested that architectural, design, theoretical and general software-quality issues discovered during the Dioxus rewrite be preserved for a major refactoring phase after the port.

The [machine-readable register](../../.scratch/dioxus-frontend-v1/refactor-findings.json) holds stable IDs, evidence, impact, affected workflows, current mitigations, later proposals and validation questions. This document is its readable index. Findings are observations and proposals, not approved redesigns. A missing screen alone is not an architectural defect; uncertainty is stated explicitly.

## Required practice during the rewrite

Every workflow handoff and independent review must update an existing RF entry, add an evidence-backed entry, or state **“No new refactoring takeaway observed”** with the scope reviewed. Capture small local design problems as well as recurring architectural or conceptual issues. Include source/revision, the concrete example, why it matters, and what would confirm or disprove the proposed explanation.

Keep correctness required for parity in the current slice. Defer broad cleanup and redesign until after the port; a deferred entry never waives a bug, public-contract decision, accessibility requirement or release gate. Record temporary bridges and their intended retirement conditions. Preserve resolved entries with the fixing commit and evidence so the later retrospective includes what worked.

At frontend v1, F9 produces a refactoring handoff grouped by architectural boundary, recurring cause, user impact and uncertainty. Prioritize the future refactor using the accumulated evidence; do not adopt every suggested abstraction automatically.

## Current port checkpoint

The authoritative current delivery state is [current_progress](dioxus-frontend-v1-run.json). Recent source-specific handoffs retain the outline test-harness/provider-root friction, PCB module default/focused finding visibility mismatch and shared Keycaps worker/viewer capability gap in the machine register. Every parent criterion remains separate from bounded journey evidence.

## Historical port checkpoint — 2026-10-02

The current served build is 34730/d320, with paired same-archive Case 90/90. PCB07+08 now has a reviewed single-header/ordered-section source join at 94480901; its initial duplicate-header and section-order regression is retained under RF-001. The guarded page-only packaging helper is integrated at 4d08ac9b; its reproduced root-config and extra-provider-file guard defects and remaining real-build qualification are retained under RF-009. The full helper-matched baseline is building from e3566700, while all parent gates stay open.

The source-specific machine register now retains the paired34727 bridge-camera blank-canvas defect, confirmed production IndexedDB error/abort callback-lifetime regression, and shared Inspector clipping. Required fixes remain in the port; neither parent acceptance nor the future refactor absorbs these blockers. The original repeated Matrix save's abort cause remains unknown despite12 fresh successful creates/history/reloads. Reviewers are keeping the callback defect and its trigger distinct.

Temporary storage pressure also exposed an execution reliability cost: inactive generated build targets filled tmpfs during parallel browser/build work. The coordinator preserved two inactive generated target trees on disk with original-path symlinks after checking process references and inactivity; source, Git history and failed browser state were retained. This adds operational evidence to RF-009 without claiming a product architecture defect.

The board Wiring pin/lock child keeps its action adapter in the private `pcb_wiring/pins.rs` feature module and routes the new control through the existing Editor owner, accepted Session snapshot and `ReplaceDocument` edit seam. This is another concrete instance of RF-001's shared root wiring cost and current mitigation; it does not establish that the hotspot is resolved. Its public import/reload failure remains unattributed. The private predicate probe reuses the existing board-scope versus UI-scope distinction in RF-006 and exposes no debug surface. **No new refactoring takeaway was observed** beyond RF-001/RF-006/RF-009. See the [Issue 11 contract](../../.scratch/dioxus-pcb-view/issues/11-board-wiring-pin-lock-controls.md) and [private import probe](../../.scratch/dioxus-pcb-view/evidence/pcb-apply-mode-disabled-20261002/private-probe-20261003.md).

The F3.4d perimeter canvas slice keeps point drafts inside the private Layout outline owner and uses the existing Session/Core preview and commit path. Its new `ClearPreview` event is the narrow missing settlement operation for a canceled Core preview; it carries the drag transaction and accepted token/revision/board identity, suppresses that transaction's matching in-flight reply, and clears the displayed scene only while that transaction still owns it. The source does not support a new durable architecture finding beyond RF-001 (shared Editor/Core operation seam), RF-006 (captured board/document scope), and RF-009 (pinned React/fixture evidence). **No new refactoring takeaway was observed** within this slice. See [F3.4d](../../.scratch/dioxus-layout-authoring/specs/F34d-outline-canvas-editing.md), [Issue 14](../../.scratch/dioxus-layout-authoring/issues/14-outline-canvas-editing.md), and its pinned React baseline.

## Current findings

| ID | Finding | Evidence status | Impact priority | Affected work |
| --- | --- | --- | --- | --- |
| [RF-001](#rf-001) | Shared presentation and Runtime are integration hotspots | design risk supported by source | high | F2, F3, F4, F5, F6, F7, F8 |
| [RF-002](#rf-002) | Internal browser host types are exposed as crate APIs | confirmed observation; future cost is a design risk | high | F7, F8 |
| [RF-003](#rf-003) | CAD engine capabilities and host protocols drift apart | confirmed adapter gap | high | F6, F7, F8 |
| [RF-004](#rf-004) | Immutable export tokens do not model export-owned commits | confirmed contract mismatch for PCB workflow | high | F5, F8 |
| [RF-005](#rf-005) | Geometric edit planning lives in frontend helper policy | confirmed remaining TS policy; target placement is a hypothesis | medium | F3, F6, F7 |
| [RF-006](#rf-006) | Canonical, physical-instance and isolated sample scopes are easy to conflate | confirmed distinct scopes; abstraction improvement is a hypothesis | high | F3, F4, F5, F6, F7, F8 |
| [RF-007](#rf-007) | Runtime observation currently supports one subscriber | hypothesis requiring consumer audit | medium | F2, F3, F4, F5, F6, F7, F8 |
| [RF-008](#rf-008) | Archive packing capability is split from its UI options and asset resolution | confirmed frontend parity gap | medium | F2, F4, F8 |
| [RF-009](#rf-009) | Parity accounting and acceptance evidence are scattered | confirmed planning and evidence gap | medium | F9 |
| [RF-010](#rf-010) | Cancellation has different guarantees at worker and kernel boundaries | confirmed limitation; not automatically a design defect | medium | F6, F7, F8 |
| [RF-011](#rf-011) | CAD revision envelope has a JavaScript safe-integer ceiling | confirmed CAD-only technical limit; low-probability scale risk | low | F7, F8 |
| [RF-012](#rf-012) | Renderer host relies on reflective method names and partial capability wrappers | confirmed wrapper shape; maintenance risk needs measurement | medium | F3, F4, F6, F7 |
| [RF-013](#rf-013) | Object tree containers have invalid required-child semantics | confirmed inherited critical axe violation in candidate and React | high | F3, F9 |
| [RF-014](#rf-014) | Host callback ownership must survive browser terminal events | confirmed production defect; minimal source repair reviewed/integrated | high | F2, F3, F9 |
| [RF-015](#rf-015) | Shared viewer controls can read a different source than the rendered scene | confirmed source-selection defect; broader interface improvement is a hypothesis | medium | F3.6, F7.3b |

## RF-001

**Shared presentation and Runtime are integration hotspots** — architecture / modularity. design risk supported by source.

At the recorded baseline `c827c4e6`, the presentation spanned 1,399 lines and Runtime 960 lines, with shell, workspace routing, data/service actions and lifecycle integration sharing these modules; the reference Workbench was 1,426 lines. These are historical measurements, not current line counts. Length alone is not a defect; the concrete cost is that independent workflow edits repeatedly touch the same ownership surface.

**Impact:** Concurrent changes can conflict, and a one-to-one port could preserve broad coupling between workspace controls and service lifecycles.

**During the port:** Use private feature modules with one coordinator owning shared composition; avoid a general framework before useful screens land.

**After the port:** After parity, assess stable workspace/host orchestration modules with narrow interfaces and one authoritative session.

**Validate:** Measure cross-workflow files changed for a representative feature and show that isolated workflow edits/tests no longer require unrelated shell/runtime changes.

Evidence: [presentation.rs](../../web/src/presentation.rs), [runtime.rs](../../web/src/runtime.rs), [Workbench.tsx](../../app/src/ui/Workbench.tsx).

### RF-001 Case/shared-viewer composition evidence

The paired Case exploration at integration source `89b1de8a` confirms the visible Case hierarchy/editor split at the current composition seam: Case still calls the generic Layout-oriented Objects tree, the authored editor is mounted in the Inspector, and mechanical settings remain in the central Case panel. Existing Case body/mechanical controllers are present, so the parity correction is a root-owned contextual composition around existing feature owners. This is current correctness work; it does not establish that composition modularity caused the mismatch or authorize a broad refactor. See the [source/UI audit](../../.scratch/dioxus-frontend-v1/evidence/case-shared3d-parity-reset-20261002/AUDIT.md).

The confirmed six-stream parity reset has dispatched a bounded private composition extraction against reviewed contract v3. It splits six feature render leaves from the root dispatcher while preserving existing feature-local lifecycle and Runtime ownership; this is a mitigation for the shared edit hotspot, not evidence that the hotspot is resolved. No new architectural takeaway was observed in the contract/review handoff beyond RF-001. See the [retained contract (current SHA-256 `c6550ab5…`)](../../.scratch/dioxus-workbench-parity/evidence/workbench-composition-contract-v3-reviewed.md) and [six-stream ticket (current SHA-256 `7279b6bf…`)](../../.scratch/dioxus-workbench-parity/issues/01-private-workbench-composition.md).

PCB scene review at source correction `590140b2` also found that accepted board membership/part projection is rebuilt on selection renders and generator drawing transfers owned parameter and full definition inputs through `FootprintGraphics`. The existing owned-prop boundary is retained; the obvious duplicate local parameter clone may be removed, but broad memoization is deferred until render/allocation cost is measured on a real generated multi-part board. This is an unmeasured design risk, not a performance pass or current assembly blocker. See the [PCB Wiring/context Inspector audit](../../.scratch/dioxus-pcb-view/evidence/wiring-context-20261002/audit.md).

The bounded PCB Wiring Runtime helper independently repeats the mechanical resolver’s accepted-source and exact Core worker `Rc`/executor-epoch checks around its async request. This is necessary feature-local protection now; assess a shared tested read-query lifecycle boundary only after parity. The private Inspector also remains an unmounted source slice until root composition/build/public checks. See the [implementation handoff](../../.scratch/dioxus-pcb-view/evidence/wiring-context-20261002/implementation-handoff.md).

The protected-handoff review route exposed a distinct operation boundary: Core's normal edit path calls `preserve_handoff`, which intentionally restores downloaded protection after any ordinary replacement edit. Clearing it through `ReplaceDocument` therefore cannot implement the existing Core review action. The bounded correction adds a Session event that dispatches Core's already-existing guarded `ReviewElectricalRemap` request and lets Session own normal accepted-document/persistence settlement. Core's review operation does not add its own Undo snapshot, consistent with downloaded protection remaining outside ordinary edit history. This records RF-001's Session/Core ownership seam; it does not propose another document authority or a general operation framework. See [Issue 12](../../.scratch/dioxus-pcb-view/issues/12-protected-electrical-remap-review.md) and the [Session/Core correction receipt](../../.scratch/dioxus-pcb-view/evidence/pcb-protected-remap-20261003/followup-core-session-correction.md).

The PCB Ctrl-click regression on packaged source `32a57ed3` exposed competing selection owners: part pointer-down applied selection, while captured compatibility click could reach the SVG empty-hit handler and clear it. The bounded repair makes pointer-down own pointer selection in PCB; click-only accessibility activation keeps its current scoped admission. The later refactor should keep normalized pointer gestures and activation routing under one explicit host lifecycle owner, rather than infer an empty hit from a capture-retargeted click. The trusted browser regression goes RED with an empty selection; integrated GREEN remains open at this source freeze. See [Issue 13](../../.scratch/dioxus-pcb-view/issues/13-canvas-command-pill.md) and its [regression receipt](../../.scratch/dioxus-pcb-view/evidence/pcb-modifier-click-repair-20261003/red.json). This extends RF-001; it introduces no new API, schema or structural cleanup.

The Parts preset-to-preview slice (source pending) makes one logical library choice visible through the existing selected-definition signal plus a Parts-private preset signal. A single activation updates both, while serialized ordered recipe identity and selection generation gate asynchronous sample publication. This is a bounded mitigation under RF-001; adding more library target kinds can create inconsistent owners if those signals evolve independently. After parity, assess a typed Parts library choice that distinguishes definition, assembly preset and module variant while keeping one accepted scope owner. No active-document write or new source authority is introduced.

## RF-002

**Internal browser host types are exposed as crate APIs** — architecture / API design. confirmed observation; future cost is a design risk.

web exports cad_jobs, renderer_host and several host modules publicly; CadOperation/CadRequest and wrapper methods are public. Adding a seemingly internal operation can therefore widen a Rust crate surface even when wire/file semantics stay unchanged. The page binary imports that library, so a pub(crate) method there is not callable by the binary; private wrapper placement must respect the real crate boundary.

**Impact:** Implementation detail and intentional compatibility contract are hard to distinguish; private adapter evolution can trigger unnecessary broad API decisions.

**During the port:** Place private wrapper/wire types in the same crate as their consumers, or use already sufficient public facades. Verify library-to-binary reachability; preserve existing APIs.

**After the port:** Audit actual consumers, explicitly separate stable public contracts from host internals, and propose compatible deprecation/module boundaries rather than silently shrinking visibility.

**Validate:** List real external consumers and compile compatibility fixtures; prove internal operation changes no longer alter the intended public surface.

Evidence: [lib.rs](../../web/src/lib.rs), [cad_jobs.rs](../../web/src/cad_jobs.rs), [renderer_host.rs](../../web/src/renderer_host.rs).

The Case generation button now has a private page-local readiness projection because the page binary cannot call the library crate's private `preparation_request`. It mirrors the matched-board readiness condition and keeps `preparation_request` as the final authority; regression tests cover missing readiness, mismatched board, configured and `case_ready` cases. This closes the observed enabled-action/rejected-request mismatch without widening APIs, but leaves a small duplicated predicate across crate boundaries. After parity, assess a supported shared admission contract or another way to keep the UI projection and request authority aligned. Evidence: [Case readiness paired check](../../.scratch/dioxus-frontend-v1/evidence/layout-toolbar-paired-20261002/RESULTS.md), [private predicate](../../web/src/case_generation_admission.rs), source commit `5944a1df`.

## RF-003

**CAD engine capabilities and host protocols drift apart** — architecture / capability discovery. confirmed adapter gap.

Rust WASM already exports build_keycaps, but the Dioxus host/worker operation set covers case preview/exact/STEP/read only. The initial review mistook the absent host path for a missing provider until the engine export was traced.

**Impact:** Capabilities can be overlooked, duplicated, or falsely described as backend work; behavior and cancellation can diverge between host wrappers.

**During the port:** BND.1 proves the private keycap worker adapter using the existing engine; retain reference chunk/yield and revision/ID semantics.

**After the port:** Create an explicit capability-to-host contract map and shared conformance fixtures for required engine operations; consider reducing duplicated wrapper protocol knowledge.

**Validate:** Every required capability has a verified engine→worker→host→UI trace, same fixture outputs and cancellation/error disposition.

Evidence: [keycaps.rs](../../cad/wasm/src/model/keycaps.rs), [lib.rs](../../cad/wasm/src/lib.rs), [index.ts](../../cad/src/index.ts), [CaseClient.ts](../../app/src/CaseClient.ts), [cad_jobs.rs](../../web/src/cad_jobs.rs), [cad_worker.rs](../../web/src/cad_worker.rs).

The Case/shared-viewer audit provides a second capability-to-host example: a private model-delivery helper exists, but it is not registered/called by the page, and the current scene projection supplies empty model/surface/hole inputs. Generated CAD body meshes therefore do not demonstrate PCB model delivery. Keep implementation status distinct from source presence and public behavior; see the [audit](../../.scratch/dioxus-frontend-v1/evidence/case-shared3d-parity-reset-20261002/AUDIT.md).

The paired Case journey on the exact imported layered-Sofle archive confirms that gap at the public UI: TypeScript shows its 90/90 model assembly after adding the default plate, while Dioxus has a blank Case canvas. Before a body is added, React's `Update preview` is disabled; Dioxus enables `Generate case` and then rejects it as unready. The fixture lacks eligible physical-instance configuration and authored case bodies, so React generation success was not reproduced. The React physical assembly viewer exists independently of generated case-body geometry; Dioxus currently routes Case display through generated `CadScene` readiness. Preserve the baseline viewer gap and the separate Dioxus action/readiness inconsistency under F7. See the [paired results and screenshots](../../.scratch/dioxus-frontend-v1/evidence/case-readiness-parity-20261002/RESULTS.md) and [readiness diagnosis](/tmp/frontend-parity-reset-20261002/case/readiness-mismatch.md).

## RF-004

**Immutable export tokens do not model export-owned commits** — architecture / transaction theory. confirmed contract mismatch for PCB workflow.

Session export captures an immutable token and cancels registered exports on accepted commits. React PCB export deliberately adopts its own wiring and protection commits, with packaging before protection, while unrelated changes invalidate output.

**Impact:** A naive port either cancels its own valid export or weakens stale-output guards. Ownership of a multi-step operation and its accepted document lineage is not represented uniformly.

**During the port:** BND.2 must prove safe private operation/commit correlation; preserve ordering and reject all unrelated changes. If impossible through existing contracts, record a concrete API proposal before dependent code.

**After the port:** Evaluate explicit operation lineage/transaction ownership across session jobs, edits, persistence and artifact delivery, with a documented consistency model.

**Validate:** Race matrix: owned versus unrelated commits, save failure, navigation, cancellation and packaging failure; no stale delivery and no protection without successful packaging.

Evidence: [session.rs](../../application/src/session.rs), [context.ts](../../app/src/exports/context.ts), [pcb.ts](../../app/src/exports/pcb.ts), [runtime.rs](../../web/src/runtime.rs).

## RF-005

**Geometric edit planning lives in frontend helper policy** — architecture / domain ownership. confirmed remaining TS policy; target placement is a hypothesis.

Resize/reflow computes canonical linked-half deduplication, axis projections, spacing and center preservation in TS UI helpers. No equivalent callable Rust resize/reflow controller was found, although core owns matrix projection and accepted edits. The pinned React Case edit path also plans generated PCB clearance parts from canonical and physical-instance closure settings: it reflects flipped X coordinates, deduplicates board/position holes while preserving the largest required drill, replaces only its owned generated parts/definitions, and synchronizes layout and board membership. Rust `SetMechanical` only assigns `doc.mechanical`; it does not perform this document projection.

**Impact:** A component-only rewrite would lose behavior or leave hidden TS logic; moving the policy carelessly could duplicate geometry authority. A Case configuration-only port would also leave exported PCB holes and saved membership stale, while splitting the update into separate edits would break atomic Undo/persistence behavior.

**During the port:** F6C.3 ports the existing policy into a private Rust frontend controller with preserved helper/public workflow tests and one normal edit/history path. F7.4e preserves the bounded Case closure-clearance document projection through one existing accepted-document/Session edit, without moving geometry authority or widening public APIs.

**After the port:** After parity, decide whether interaction planning belongs in a dedicated application service using core geometry primitives, with explicit draft/commit ownership.

**Validate:** Compare identical resize traces across pointer/keyboard, mixed selections and linked boards; no semantic drift, duplicated calculations or extra history steps. For Case, compare canonical/physical-instance closure edits, reflected and coincident holes, generated definition/drill data, board/layout membership, saved archive payload, one-step Undo/Redo and reopen.

Evidence: [planKeycapResize.ts](../../app/src/ui/planKeycapResize.ts), [keycapReflow.ts](../../app/src/ui/keycapReflow.ts), [interactions.rs](../../application/src/interactions.rs), [layout.rs](../../core/src/matrix/layout.rs), [closureClearance.ts](../../app/src/closureClearance.ts), [createProjectActions.ts](../../app/src/createProjectActions.ts), [core edit operations](../../core/src/lib.rs), [Case mechanical settings evidence](../../.scratch/dioxus-case-workspace/evidence/mechanical-settings-contract/implementation-contract.md).

**Current effective-projection handoff (2026-10-02):** a private stateless carrier can reuse the existing public `captured_case_document` projection for a proposed canonical document. Keep original accepted snapshot identity and full Scope for validation, async guards and Core correlation; the ephemeral carrier shares the original scene Arc/token/epoch, exists only as projection input, and is immediately discarded. A paired call to `captured_case_document` and `captured_case_scene` costs two document plus two scene projections because each clones both values even though each call discards one half. This is a bounded per-resolution cost, not a once-total-copy or measured-performance claim; avoid per-render calls and extra caller clones. See the [retained projection review](../../.scratch/dioxus-frontend-v1/evidence/planning/mechanical-effective-projection-cost-20261002.md).

## RF-006

**Canonical, physical-instance and isolated sample scopes are easy to conflate** — design / coordinate and identity model. confirmed distinct scopes; abstraction improvement is a hypothesis.

Layout/Keymap/Keycaps use canonical boards; Case projects a physical instance; Parts builds an isolated sample. Authored Case STEP uses canonical saved bodies while generated mechanical export explicitly projects the instance. All may share viewer and artifact code.

**Impact:** A generic board/instance argument or copied projection rule can mirror the wrong geometry, leak a sample into the document, or export the wrong scope.

**During the port:** Use one shared viewer with explicit consumer inputs; preserve each reference projection and captured identity. Test cross-scope switches and stale replies.

**After the port:** Evaluate typed scope/projection inputs and coordinate-frame invariants for view, edit and export consumers; separate display projection from saved-document identity.

**Validate:** Table-driven fixtures for front/back/split/instance/sample scopes plus properties for stable IDs, reflection count, unchanged canonical document and correct outputs.

Evidence: [Workbench.tsx](../../app/src/ui/Workbench.tsx), [LibraryWorkspace.tsx](../../app/src/ui/LibraryWorkspace.tsx), [cases.ts](../../app/src/exports/cases.ts), [cad_jobs.rs](../../web/src/cad_jobs.rs).

**Current effective-projection handoff (2026-10-02):** the private proposal projection and matching contours deliberately use different inputs. Project the ephemeral proposed canonical document through `captured_case_document`; use `captured_case_scene` on the original accepted snapshot for the unchanged board contours. That preserves the physical-instance reflection/winding policy once, while preventing a reflected display document from being persisted or projected a second time. The temporary carrier never becomes Session, accepted-read-model, CAD cache/job, export or operation authority. No public API is added or widened, and no default/material policy is copied. The source-backed cost and identity limits are recorded in the [retained projection review](../../.scratch/dioxus-frontend-v1/evidence/planning/mechanical-effective-projection-cost-20261002.md).


The PCB electrical preview explicitly uses board `Scope(instance_id=None)` while physical Case/mechanical operations retain their instance identity. This source distinction is recorded in the [Wiring implementation handoff](../../.scratch/dioxus-pcb-view/evidence/wiring-context-20261002/implementation-handoff.md); do not generalize the board normalization to instance-scoped operations.

**New keyboard guide reference observation (2026-10-02):** In the isolated
React session `new17-c8a853a97c72`, a rapid Project-stage `Split keyboard` then
`Reversible layout` sequence showed `Error: Stale base revision` once. The
first observation did not capture its exact pre-action accepted revision, so
later revision reads are not attributed to it. After a separately settled
Unibody/None state at revision 11, the same rapid sequence completed at
revision 14 without an alert; a settled Reversible toggle also completed
revision 7→8 without an alert. This records an unreproduced timing observation,
not a confirmed React defect or cause. Candidate code must retain its accepted
identity/revision guards. The [paired New/guide React evidence](evidence/new-keyboard-guide-paired-20261002/README.md)
contains exact screenshots, UI captures, project ID and revision probes.

## RF-007

**Runtime observation currently supports one subscriber** — architecture / reactivity. hypothesis requiring consumer audit.

Runtime subscribe assigns a single notifier and unsubscribe clears it. This supports the current root composition but is not an independently mountable multi-consumer subscription contract.

**Impact:** If feature modules subscribe directly, one may replace another or remove notification delivery on unmount. No present lost-update bug is claimed.

**During the port:** Keep the existing root subscription and pass projections/callbacks to children; do not give each workspace an independent subscription.

**After the port:** Assess a scoped subscription lifecycle or explicit root reactive projection after measuring actual needs; avoid adding an event bus by default.

**Validate:** Mount/unmount and rapid switching with two observers in a characterization probe; verify ownership and no stale/lost notification or redundant large clone.

Evidence: [runtime.rs](../../web/src/runtime.rs).

## RF-008

**Archive packing capability is split from its UI options and asset resolution** — design / persistence orchestration. confirmed frontend parity gap.

React copy always includes local assets and optionally discovers/embeds used bundled models with the project-name filename. Runtime packing currently includes document.assets and delivers keyboard.boardstudio without that option.

**Impact:** The same visible copy action can produce archives with different portability and naming across entrypoints/hosts.

**During the port:** F2.2 owns one shared packing action with correct asset semantics; F8 owns its route checkbox/button. Preserve formats, IDs and hashes.

**After the port:** Unify explicit archive input/options and asset-source resolution responsibilities with one compatibility test corpus across hosts.

**Validate:** Reference/candidate round trips with imported assets, bundled used models enabled/disabled, missing asset failures and identical project-name output.

Evidence: [storage.ts](../../app/src/storage.ts), [Workbench.tsx](../../app/src/ui/Workbench.tsx), [runtime.rs](../../web/src/runtime.rs).

## RF-009

**Parity accounting and acceptance evidence are scattered** — verification / process design. confirmed planning and evidence gap.

The original roadmap contained invented project/export controls and coarse dependencies; UI-owned helper work was not explicit. Existing browser suites target React, bounded Dioxus evidence lives in task artifacts, and historical visual references are only in the original checkout. Actual screen-reader coverage is unavailable on this host.

**Impact:** Plans and passing partial checks can imply unsupported completeness; evidence may not identify the delivered source and environment consistently.

**During the port:** Maintain the source/task/evidence ledger and public Dioxus scenarios during the port; resolve actual AT and applicable release gates before frontend adoption.

**After the port:** After parity, consolidate maintained conformance scenarios, source/fixture/build provenance and CI integration; treat evidence as reproducible outputs rather than accumulated prose.

**Validate:** From one commit, locate every required workflow scenario and produce its current result with exact source/build/fixture identity and explicit unavailable gates.

Evidence: [tsx-inventory.json](../../.scratch/dioxus-frontend-v1/evidence/tsx-inventory.json), [CONSTRAINTS.md](../../CONSTRAINTS.md), [playwright.config.ts](../../app/playwright.config.ts), [ACCEPTANCE.md](../../.scratch/m1-production/ACCEPTANCE.md).

### RF-009 Case/shared-viewer acceptance accounting

The current F7.8 consumer list names five workflows while the confirmed migration scope has six, including PCB. PCB’s existing F5.8 acceptance is the proposed additional F7.8 join; the canonical 62-task graph remains unchanged until coordinator review. The exploratory audit deliberately does not claim same-archive paired parity. See the [Case/shared-viewer audit](../../.scratch/dioxus-frontend-v1/evidence/case-shared3d-parity-reset-20261002/AUDIT.md) and [parity reset addendum](../../.scratch/dioxus-frontend-v1/issues/07-case-3d-parity-reset-addendum.md).

### RF-006/RF-009 Parts preview source correction handoff

Dioxus 0.7.10 returns cloned `use_hook` state, so plain `Cell`/`RefCell`
mutations do not persist across renders unless the hook value is a shared
handle. The Parts preview source correction uses retained `Rc` handles and
regresses accepted-resource A→B→A identity. Its geometry fixture also records
that `footprint_forms::point` already converts native KiCad Y before the SVG
flip; bounds use the projected frame once and preserve the reference origin
policy. See the [Parts preview handoff](../../.scratch/dioxus-parts-catalogue/evidence/parts-preview-source-corrections/refactor-handoff.md).
This is source-level RF-006/RF-009 evidence, not a compile or browser result.
The packaging review also found that the generated service entrypoint omitted
its already-existing `parameters(source)` export. The private builder now
forwards and provenance-hashes that export, and self-checks the generated MX
defaults before success; root/subpath/offline delivery remains unverified.
The preview's projected-graphic bounds fixture does not yet compare React's
additional raw-form extents (which include hidden/reference text); that paired
MX framing check remains open.

### RF-009 encoder initial-value regression evidence

The built encoder candidate at source `e82c039b` / build `frontend-encoder-bindings-20261002` (34685) rendered three unassigned dynamic selectors as `key-press` despite the empty accepted sensor/binding maps; readonly evidence confirmed no document mutation. The narrow declarative `option.selected` correction at source `868edfcb`, build `frontend-encoder-select-fixed-20261002` (34687), now renders `none` in the value and selected options of all three controls. This is a bounded regression/evidence observation; it does not claim a framework-wide issue or broad architecture finding. Genuine archive interaction/recovery and F5.2/F8.2/AT/parent gates remain open. The exact retained paths are in the [encoder regression handoff](../../.scratch/dioxus-keymap-layers/evidence/encoder-select-regression/refactor-handoff.md), [baseline packet](../../.scratch/dioxus-keymap-layers/evidence/encoder-public-workflow/) and [corrected regression packet](../../.scratch/dioxus-keymap-layers/evidence/encoder-select-regression/).

### RF-009 PCB Wiring and selected-switch accounting

The pinned React Sofle flow shows a board-level Wiring plan when nothing or a controller is selected, and a read-only accepted-net/name projection for a selected switch. The current page still contains the PCB placeholder; the new private host-scene leaf awaits root mount and paired public verification. Core already owns `ResolveElectrical`; a private Runtime request/scope adapter and Inspector composition are still required. The proposed F5.2a draft keeps that adapter, the accepted selected-board scope, and the selected-switch context explicit without closing F5.2 or its INT.2 acceptance join. The candidate page observation used an unpinned local server, so it is exploratory only. See the [source/UI audit and packet](../../.scratch/dioxus-pcb-view/evidence/wiring-context-20261002/audit.md) and [draft child](../../.scratch/dioxus-pcb-view/drafts/F5.2a-wiring-preview-and-switch-context.md).

The private Runtime adapter and Wiring/selected-switch projection are now authored at source commit `626668b7`; an executor-identity/UI parity follow-up is under exact-source review. The [implementation handoff](../../.scratch/dioxus-pcb-view/evidence/wiring-context-20261002/implementation-handoff.md) records current hashes and root’s mount/CSS contract. Compilation, public pairing and F5.2/F5.3/INT.2 remain open.

### RF-009 Core ApplyElectrical lock revalidation

The selected-board Resolve path supplies persisted `ElectricalBoardConfiguration.locks`, while the existing Core `ApplyElectrical` handler constructs its re-resolution request with an empty lock map. Exact-source review found that `electrical::resolve` begins with the current target board's persisted locks and extends them with request locks, so Apply preserves them; the remaining gap is direct request-level coverage for persisted locks, not a confirmed defect. The [corrected audit](../../.scratch/dioxus-pcb-view/evidence/apply-current-plan-20261002/core-apply-lock-audit.md) and [draft Issue 09](../../.scratch/dioxus-pcb-view/drafts/tickets/09-core-apply-electrical-honors-saved-locks.md) record that characterization test. The independent F5.2d private Apply route uses the existing materializer with its exact accepted Current plan and one normal Session edit. No public request/schema change or new RF ID is proposed.

### RF-009 guarded packaging ownership extension

**Qualification correction, 2026-10-03:** the focused author suite is not a completed ownership proof. Independent matched-baseline negative cases found provider-overlap precedence, worker feature expansion and include/inline-module gaps. Reuse is withheld while Sol repairs conservative guards; full22 remains available. Preserve the initial observation below as historical proposal, not current safety qualification.

The prior fixed page-leaf provider reuse contract could not admit ordinary Rust UI module edits without rebuilding unchanged CAD/Core providers. A narrow implementation now derives eligible Rust paths from active page and worker feature module graphs, while preserving the old explicit leaf list and full22 build path. Cargo feature ownership and locked command identity remain pinned; ambiguous modules, provider overlap, shared Cargo/build inputs, deleted paths, and unknown cfg syntax fail closed. The known Core test alias and test-only Core integration source are recorded explicitly. This is a build-time ownership proof, not frontend acceptance or a completed performance result. See [the focused expected-red and guard results](../../.scratch/dioxus-frontend-v1/evidence/packaging-page-rust-reuse-20261003/RESULTS.md); real helper-matched baseline/reuse packaging and root/subpath/offline verification remain open.
### RF-009 protected-remap evidence correction

The first Issue 12 source packet proposed a normal `ReplaceDocument` edit to remove `protected_handoff`. Actual Session/Core execution proved that path preserves the baseline: `CoreEngine::edit` calls `electrical::preserve_handoff` before returning the next document. The earlier source and receipt remain preserved as history but their direct-edit semantics are superseded. The corrected route uses a new narrow Session event to dispatch Core's existing revision/fingerprint-checked `ReviewElectricalRemap` request; a production Session/Core test proves the normal edit retains protection, the dedicated operation clears it in one saved revision, and lock/assignment data survives. RF-009 retains the correction and exact evidence provenance; no duplicate RF item or Core wire/schema change is proposed. See the [follow-up receipt](../../.scratch/dioxus-pcb-view/evidence/pcb-protected-remap-20261003/followup-core-session-correction.md).

## RF-010

**Cancellation has different guarantees at worker and kernel boundaries** — theoretical / async resource lifecycle. confirmed limitation; not automatically a design defect.

Preview can yield between keycap chunks; a synchronous WASM STEP build cannot be interrupted mid-kernel call. Host cancellation can stop queued work and suppress stale completion without immediately releasing ongoing computation.

**Impact:** A single cancelled status can obscure remaining CPU/memory work or lead the UI to promise prompt preemption that the engine does not provide.

**During the port:** Preserve chunk/yield preview behavior and late-result suppression; report actual cancellation granularity, keep resource checks and do not add unproven preemption claims.

**After the port:** Review cancellation contracts, scheduling/resource budgets and operation status terminology across host/worker/kernel; evaluate cooperative work units only with measured benefit.

**Validate:** Measure cancel-to-settle and retained resources for queued, chunked preview and synchronous export; verify no stale scene/file even when computation finishes after cancellation.

Evidence: [index.ts](../../cad/src/index.ts), [cad_worker.rs](../../web/src/cad_worker.rs), [keycaps.rs](../../cad/wasm/src/model/keycaps.rs).

## RF-011

**CAD revision envelope has a JavaScript safe-integer ceiling** — theoretical / representation and identity. confirmed CAD-only technical limit; low-probability scale risk.

MAX_CAD_REVISION is 9,007,199,254,740,991 and larger CAD revisions are explicitly rejected because that worker boundary uses JS numbers. This is specific to CAD and does not negate the separately verified exact core/archive integer transport.

**Impact:** The CAD identity domain is narrower than the document integer domain; practical lifetime relevance needs evidence.

**During the port:** Preserve explicit rejection and existing boundary tests; never round a revision or weaken stale checks.

**After the port:** Audit whether this limit matters; only if warranted consider string/bigint identity across the CAD envelope with compatibility handling.

**Validate:** Max/max+1 request/reply tests and realistic revision growth estimate before changing representation.

Evidence: [cad_jobs.rs](../../web/src/cad_jobs.rs).

## RF-012

**Renderer host relies on reflective method names and partial capability wrappers** — architecture / boundary typing. confirmed wrapper shape; maintenance risk needs measurement.

RendererHost invokes wasm methods through Reflect/string names and wraps only part of the already-exported state/handles/pick/scene/camera capability set.

**Impact:** Missing wrappers and string-level mismatches can surface at runtime rather than at a typed application boundary.

**During the port:** F7.1 maps the existing exports; F7.3 supplies one private viewer adapter with public behavior/lifecycle checks and correct crate placement.

**After the port:** Evaluate a small typed internal viewer port/capability map and generated or checked boundary mappings; do not duplicate renderer behavior per workspace.

**Validate:** Exercise every mapped method, model/scene DTO and mount/update/error/context-loss/disposal path with the same fixture outputs.

Evidence: [renderer_host.rs](../../web/src/renderer_host.rs), [wasm.rs](../../renderer/src/wasm.rs).

## RF-013

**Object tree containers have invalid required-child semantics** — accessibility / semantic structure. Confirmed inherited critical axe violation in the Dioxus candidate and pinned React reference.

Axe 4.12.1 reports `aria-required-children` at the candidate `.m1-component-list[role="tree"]`, whose direct labelled disclosure-button children do not satisfy the tree role's required child structure. The pinned React `.wb-tree-viewport[role="tree"]` has the same violation. Candidate source 515f390d reports one critical violation; the React report has that same critical violation plus a separate moderate `page-has-heading-one` finding. This paired result establishes an inherited reference defect. It does not establish candidate conformance or waive the candidate's accessibility acceptance.

**Impact:** Assistive technologies may not receive the required tree/treeitem semantics for hierarchy navigation. Axe classifies the finding as critical, so it remains an active correctness gate.

**During the port:** Do not suppress the rule, weaken thresholds or accept the candidate because React shares the defect. A private candidate semantics repair at `f65b` preserves existing row roles and is under independent review; it is not integrated or accepted. Keep the paired baseline reports, and retain actual assistive-technology testing as a separate gate where applicable.

**After the port:** Review the tree semantic structure and maintained accessibility conformance scenario across workspace navigation; keep the eventual design narrow and compatible with disclosure and selection behavior.

**Validate:** Axe reports no `aria-required-children` violation for the candidate; public hierarchy, selection, keyboard and focus behavior remain correct. Retain the React report as evidence of the inherited baseline defect. Actual assistive-technology evidence remains separately required.

Evidence: repair review status: private `f65b` under independent review, not integrated/accepted. Candidate [`tree-515f390d-a11y.json`](../../.scratch/dioxus-frontend-v1/evidence/tree-and-parts-4b05d451/tree-515f390d-a11y.json), React [`tree-react-a11y.json`](../../.scratch/dioxus-frontend-v1/evidence/tree-and-parts-4b05d451/tree-react-a11y.json), paired hierarchy/public result [`tree-515f390d-hierarchy.json`](../../.scratch/dioxus-frontend-v1/evidence/tree-and-parts-4b05d451/tree-515f390d-hierarchy.json), and the [source review](<../../.scratch/dioxus-frontend-v1/evidence/tree-and-parts-4b05d451/source-review-reference.md>).

## Entry template

Use the next stable RF ID. Record: title; category; confirmed limitation, supported risk or hypothesis; source revision and specific evidence; impacted workflows and example; severity with reason; mitigation required during the port; temporary adapter/retirement implications; proposed post-port investigation/refactor; validation or falsification criterion; owner; linked task/issue; and eventual decision/fix evidence. Add source line links where they clarify the finding. Do not overwrite earlier evidence when a hypothesis changes.

The initial entries were independently source-checked; [review notes and correction](../../.scratch/dioxus-frontend-v1/evidence/planning/refactor-review.md) retain the reasoning and the keycap-engine discovery.


## First-tranche ticket audit — 2026-10-02

The [source-backed ticket proposal](../../.scratch/dioxus-frontend-tranche-1/PROPOSAL.md)
adds evidence to existing findings; it does not approve structural refactoring or
claim these behaviors fixed. Exact source locations and proposed current fixes
are retained in the [source notes](../../.scratch/dioxus-frontend-tranche-1/SOURCE-NOTES.md)
and machine register.

- **RF-005 — interaction policy:** React computes anchored rectangular matrix
  selection; the current Dioxus caller/Session path selects a flat interval and
  advances the anchor. Matrix/row/column context also differs from real part-ID
  selection. Reproduce the mismatch before repair; keep one selected-part
  authority with a proven private UI adapter. Later, review explicit ownership
  of semantic context, anchors and membership with paired selection traces.
- **RF-008 — lifecycle intent:** fixture opening currently passes through archive
  import without changing the parsed project ID; React demo creation allocates
  a fresh ID. This is a source-supported copy-identity risk, not yet a browser
  reproduction. The demo ticket must preserve prior edited copies while leaving
  ordinary archive import unchanged. Later, distinguish open/import/demo-copy
  intent and identity ownership while sharing validation and asset transport.
- **RF-009 — accounting:** the reference offers 19 demos, while Dioxus exposes
  two fixture buttons. Every family now has an explicit proposed delivery and
  open oracle. Outline navigation also needs accurate side effects: version
  activation edits the document; bridge navigation fits the camera. Missing UI
  alone is not poor design, but broad inventory labels were insufficient to
  establish parity. Retain catalogue-to-action and side-effect evidence after
  the port.


## Automatic frontier handoff — 2026-10-02

INT.1 private extraction is accepted with [integrated evidence](../../.scratch/dioxus-frontend-tranche-1/evidence/int1-integrated/README.md). Authors and independent reviewers observed no new refactoring takeaway; the bounded extraction mitigates RF-001/RF-002 while preserving the single root subscriber.

RF-009 gains source-backed reconciliation from the [next-ticket review](../../.scratch/dioxus-parts-catalogue/evidence/astra-final-review.md): catalogue source precedence, eight bundled assembly presets, shared host layer lifetime, virtual/legacy keymap projection, and permitted Base rename. Concrete feature-specific contracts remain required. All original parent joins remain open. These are accounting improvements, not evidence that the planned features are already implemented.


## Saved-library boundary diagnosis — 2026-10-02

RF-010 now has [confirmed public evidence](../../.scratch/dioxus-frontend-tranche-1/evidence/contracts-active/open-supersession-diagnosis.md): after pending A and unavailable newer B, releasing the real A worker reply adopts A and persists its active-project identity. Delayed-send and delayed-reply variants fail; no-B control passes. Full T1-02 correctness remains current work. A provider-load sequence alone cannot control the eventual adoption owner or retain worker history.

RF-002 gains the [tolerant discovery proposal](../../.scratch/dioxus-frontend-tranche-1/evidence/contracts-active/tolerant-listing-api-proposal.md). One malformed typed record currently rejects the complete library list; the page binary cannot recover its identity through existing public BrowserStore information. This is an explicit source boundary, not approval for a public API change. Pure card projection, corrected panel preferences, and tree hierarchy continue independently.


### Cards and panel review handoff

RF-001 gains a concrete immutable-ownership observation from the [cards/panels Standards review](../../.scratch/dioxus-frontend-tranche-1/evidence/cards-panels-2030c9a3/cards-panels-standards.md): Library copied complete accepted and saved documents during Runtime repaints. The bounded correction retains shared `Arc<ProjectDoc>` snapshots and borrows the saved signal; strict WASM Clippy passes, independent review/browser verification remain pending. Repaint allocation cost is unmeasured and stays a post-port validation question. The two new panel regressions remain current correctness work, with no deferred-refactoring waiver.


RF-009 gains the panel repair lesson: strict compilation did not establish browser boolean-attribute semantics or responsive cascade behavior. Actual DOM/accessibility and supported-viewport red/green checks caught inert="false" disabling visible controls and legacy fixed-grid overrides. Corrected source47cf655b and [evidence](../../.scratch/dioxus-frontend-tranche-1/evidence/panels-fixed-47cf655b/record.json) retain the failures and fixes. Broader shared DOM attribute/cascade conformance assessment is a post-port proposal, not a new current gate.

### Case and Keymap integration handoff — 2026-10-02

The [retained handoff](../../.scratch/dioxus-frontend-v1/evidence/case-keymap-current/refactor-handoff.md) adds source evidence to RF-001/RF-006 and verification limits to RF-009/RF-013. Draft identity, exact request settlement, Inspector reachability and scroll sizing corrections remain subject to the fresh WASM build and public browser gates. The offline failure was isolated to Chromium blob storage on low-space temporary profiles; the unchanged release passes controlled disk-backed profiles. This is a QA environment condition, not a new application defect. Actual assistive technology and contrast remain open. No measured performance improvement or full parent acceptance is claimed.


### RF-002 private viewer lifetime snapshot follow-up

The Case-first viewer needs private host operations across the library/binary boundary. Rust rejects direct textual inclusion of the library host because of its inner documentation comments. The temporary page host therefore copies the 475-line baseline lifetime implementation with those three comment prefixes normalized and exactly the unused unchecked update_scene method omitted. The library file and public API remain unchanged; a native source-sync test now runs with the page tests and passes. This prevents unnoticed source drift but preserves duplicate ownership until a compatible private boundary can retire the snapshot. Strict WASM/native compilation and the synchronization test pass; public viewer behavior remains a separate gate.


### RF-006 layer feedback and test-boundary follow-up

Layer editing distinguishes the stored requested layer ID from its displayed fallback, accepted field-name identity from unrelated snapshot changes, and exact acknowledgement mismatch from rejection feedback. Source review corrected these branches before integration. The Dioxus controller remains a WASM hook: strict WASM checks compile its unit tests, while native Core/Session tests and actual public UI actions prove different seams. After parity, assess whether a private state machine can expose these correlation decisions to native behavioral tests without duplicating domain edits or persistence ownership.


A bounded [Keymap focus-continuity handoff](../../.scratch/dioxus-frontend-v1/evidence/planning/binding-focus-continuity-handoff-20261002.md) adds paired candidate/React native Tab evidence to this finding. During blur-save, candidate focus falls to BODY while single-flight disables controls; React moves focus to Hold. After idle, native pointer/selection preserves and saves both fields. The earlier programmatic select red is retired because the same event order occurs on React. This is a focus-continuity gap, not demonstrated document-data loss; broader legacy/malformed/stale-race/semantic checks and actual AT remain open, with no AT pass claimed.


RF-006 gains a [bounded mechanical public-scope handoff](../../.scratch/dioxus-case-workspace/evidence/public-mechanical-settings/candidate-0cad7577/RESULTS.md). Importing the actual React Sofle split archive and editing Right to CNC through visible controls preserved Left config, mounts, generated parts/definitions and memberships exactly. Disable cleared all eight closure records; Undo/Redo, full archive-to-accepted-document equality and reload passed. This confirms the existing scope distinction without a new refactoring issue or API proposal. Draft/layout, authored mesh/STEP and full F7.4 acceptance remain open.

RF-006 also records the [paired Macro editor public result](../../.scratch/dioxus-keymap-layers/evidence/public-macro-editor/RESULTS.md): candidate and React both preserve accepted 45 ms after invalid 10001 ms input, and both retain the Core error when the accepted value is restored; a later distinct valid value clears it. This is observed parity, not a candidate-only defect. The fresh packet proves public CRUD/history/archive reopen but does not exercise firmware output, UI ceilings, or pending-document switching.


### RF-009 compact focus/resize evidence handoff

The built Case candidate exposed a confirmed keyboard-visibility gap that screenshots and canvas-intersection checks did not reveal. On the prior `b6d2af49` compact candidate, Tab could enter the covered Generate control while both panels were open; moving focus to Generate on desktop and then resizing to compact retained focus on the now-covered control. This evidence is retained in [the compact containment public packet](../../.scratch/dioxus-case-workspace/evidence/compact-containment-public/) and [the focus/resize red packet](../../.scratch/dioxus-case-workspace/evidence/compact-focus-reveal/). This records the candidate behavior only; no React parity or actual assistive-technology result is claimed.

The current repairs are source `cf67a388` for focus entry and the Case panel resize lifecycle repair integrated in `e2a84d8b00418eab4d1ec47e0bf5b6957da9ffe3`. Bounded focus-entry evidence passed on the earlier `34681` build. The final `frontend-case-focus-resize-final-20261002` package has eight commands/971 hashes at `34683`; final resize and integrated keyboard/fit public evidence remains pending verifier release. Keep viewport behavior and accessibility acceptance open until that packet is released; this handoff waives neither broader keyboard coverage nor actual AT. Evidence files and current qualification are in [the final focus/resize packet](../../.scratch/dioxus-case-workspace/evidence/compact-focus-resize-final-public/).

### Six-stream composition and Keymap integration handoff — 2026-10-02

RF-001 gains evidence from the [composition integration handoff](../../.scratch/dioxus-workbench-parity/evidence/integration-handoff-20261002.md): six private workspace leaves and the thin dispatcher are integrated and mounted at `10377780`. Standards found that newly introduced `EventHandler::new` calls in the long-lived Editor body would retain callback captures until component drop. The bounded correction at `09563bc1` allocates typed slots once in an unconditional pre-early-return hook and refreshes their bodies with `Callback::replace` each render. This is a lifecycle correction and a mitigation for shared composition contention; it is not a measured performance result or proof the wider hotspot is resolved. The Layout SVG remains inline. PCB review also observed full `PartDefinition` and generator-override copies on selection repaints; cost and user impact are unmeasured and optimization is deferred until measured.

RF-006 gains the reviewed Parts-preview findings in the same handoff. The first preview draft mutated plain `Cell`/`RefCell` values returned through `use_hook`, so owner generation did not persist across renders; its reviewed correction retains shared hook state. The first bounds implementation inverted already-Y-up graphic coordinates; its correction uses the rendered coordinate frame. Those are source-review corrections in the separate Parts candidate, not integration or paired-browser passes. Actual fixture framing remains unverified.

RF-009 records the Parts package boundary: the initial loader expected generator `parameters` absent from the generated `layout-generators.js` bundle. A later narrow export correction at `bf797f4` passed a packaged Node check, but the full Parts browser gate remains open. The retained same-fixture Keymap comparison first found a red after filtering to SW7 and moving Macros → Keys: Dioxus displayed an empty Selected key chooser while the binding editor still showed SW7; React retained SW7. The scoped correction `deba7087` is Spec/Standards-clear. Corrected integrated build `frontend-matrix-keycaps-styled-20261002` at `92db0b8f` completed eight commands/983 hashes. A fresh public run at `34691` passes the exact filtered SW7 Macros → Keys and Encoders → Keys remounts, retaining `matrix/left-keys/r1c0` and matching the retained React reference. Clear-filter SW8 canvas selection and Function → Main projection labels also pass. Read-only project revision and record hash and Undo/Redo DOM state are unchanged. Evidence and machine assertions: `/tmp/frontend-parity-reset-20261002/keymap/green-92db0b8f/`; original red and React captures remain at `/tmp/frontend-parity-reset-20261002/keymap/paired/`. This closes the narrow DOM/remount evidence gap under RF-006/RF-009; it does not establish full Keymap, F6K or parent acceptance.

Root native 34 and strict WASM Clippy passed at `98188448`; final formatting passed at `10377780`. These prior checks, the corrected build, and the targeted browser green do not close any parent or the remaining stream criteria. No new RF ID is warranted; retain the observations under RF-001/RF-006/RF-009 and preserve the remaining feature joins.

### Matrix/Keycaps integration browser evidence — 2026-10-02

RF-001/RF-009: [wave results](../../.scratch/dioxus-workbench-parity/evidence/matrix-keycaps-wave-20261002/RESULTS.md) retain the misplaced Keycaps lists and unstyled Matrix form caught in the browser, their bounded corrections, and actual compiler/build/user-journey evidence. Wider composition/visual-contract cleanup remains deferred.

RF-005/RF-009: the same Matrix→Keycaps→SW7 journey exposes a shared selection difference. Independent diagnosis traces React setting Key mode but immediately calling a closure that still sees Matrix mode. The potential confirmed-legacy-bug exception is being reviewed; original mismatch evidence remains retained. No whole workbench acceptance follows.

#### Keycaps selection diagnosis resolved

RF-005/RF-006/RF-009: [independent paired diagnosis](../../.scratch/dioxus-workbench-parity/evidence/matrix-keycaps-wave-20261002/keycaps-selection-diagnosis/DIAGNOSIS.md) confirms a legacy React stale-closure defect, rather than intended matrix selection. Its Key-mode reset is overwritten by a callback using the previous Matrix mode. Preserve the correct Dioxus single-key result under the confirmed Q1 bug exception, retain both original mismatch and intended-behavior oracle, and leave wider refactoring/full acceptance open.

### RF-001/RF-009 mounted reactive lifetime review

Independent Astra review of unintegrated Keycaps `448c3274` and firmware handoff `6fe8fb87` found current correctness bugs missed by native pure tests: a Keycaps effect reads sequence/state signals it writes and repeatedly schedules assessment, while the firmware observer compares a stable Signal handle rather than the Runtime version value and can miss delayed outcomes. Firmware feedback also loses its captured board/instance/session target after settlement. These require repair and mounted/browser regressions now; the later refactor may assess a shared tested lifecycle boundary. The [Spec](../../.scratch/dioxus-frontend-v1/evidence/source-wave-20261002/spec-review.md) and [Standards](../../.scratch/dioxus-frontend-v1/evidence/source-wave-20261002/standards-review.md) reports preserve exact source and verification limits. No feature or parent acceptance is granted.

### RF-005 Layout pointer-preview prerequisite

The [reviewed Transform/Align handoff](../../.scratch/dioxus-layout-authoring/evidence/transform-align/rf-handoff.md) records that ordinary Edit previews do not acquire GestureCancel generation suppression or clear-preview authority. Fields-only transforms and one-shot alignment use existing commits; pointer Stagger/Splay/Origin remain gated on a proven cancellation protocol. This is a current capability limitation and a later lifecycle-design question, not permission to add public Session APIs or waive gesture parity.


### RF-003 producer publication and transport review

At Case producer `2bf5b534`, independent Spec review found that successful publication invalidates the snapshot’s own retained lease and default serde Value serialization creates JS Maps while the module worker reads object fields. Decoded-model delivery also remains unconsumed. Astra is fixing these as current correctness blockers with regressions. After parity, assess explicit producer publication/ownership and real transport conformance fixtures; source presence and isolated worker tests alone did not establish the mounted capability. Production build invocation of the packager is a separate required seam.


### RF-009 browser identity and spec path correction

Keycaps evidence initially attributed a REVIUNG41/Layout screenshot to Sofle/Keycaps activation. That claim is withdrawn; the corrected report preserves it and verifies exact project/archive identity and selected workspace on the integrated `bd671ae8` package. Distinct F6C.2 settings and F6C.4 findings documents also collided at `spec.md`; `settings-spec.md` now preserves both. After parity, assess an evidence manifest that ties each claim to fixture identity, selected workspace, source/build and uniquely identified spec, rather than relying on filenames or agent summaries alone.

### Resumed wave takeaways — 2026-10-02

RF-001/RF-006 retain App-versus-panel pending-outcome ownership, rendered target identity and live guide-stage admission defects. RF-009 records the corrected stale top-level source/demo progress fields. Evidence and disposition are in `.scratch/dioxus-frontend-v1/evidence/source-wave-20261002/root-ledger-reconciliation.md`; current fixes and broader refactoring remain distinct, with browser acceptance open.

### Case13 transport owner scope — 2026-10-02

The Case wireless selector was visible but its patch was silently rejected when the active assembly had no selected part-tree item. Transport belongs to the captured Case assembly scope; selected-part ownership is an unrelated prerequisite. The bounded correction removes only that predicate while retaining accepted session/document/board/instance, generation, token/revision and current-instance checks. The focused browser-WASM owner regression was red before and green after the change. The exact public repro, accepted archive comparison, source/check packet and remaining gates are in the [Case13 handoff](../../.scratch/dioxus-case-workspace/evidence/case13-wireless-right-transport-admission-20261002/implementation-handoff.md). This records RF-006 evidence only; it does not authorize a general owner/API redesign or establish post-fix public acceptance.


### Frontend continuation source and evidence refresh — 2026-10-02

Integrated source `7be1770d` adds reviewed Case model delivery/picking and MatrixSetup to New/setup guide, physical setup, Keycaps settings and matrix owner fixes. Served build remains `4bbafae0` at 34722 until the new candidate passes source/asset checks. Live React oracle is 5173 at pinned5a472a94; cached5175 evidence is historical. RF001/003/006/009 retain shared composition/effect precedence, distinct renderer/producer identity domains, selected-owner state lifetime, static RSX root-key semantics and source-versus-serving freshness. Compact guide, Keymap and Outline repairs retain independent review and actual paired browser gates. All62 canonical parents remain unchanged and open where not previously accepted. See `.scratch/dioxus-frontend-v1/evidence/source-wave-20261002/current-wave-refresh.json`.


Frontend next candidate119c9d059c002057e73b3fd3a36648a9681c9cd6 is served at34724(root/subpath), eight page packaging commands/1280source hashes/145assets each. Keymap selected-owner reset has independent source clearance; actual paired browser acceptance is running. RF003 confirmed Case0/90 despite acceptedpreviewready; the existing packaged provider lacks Case descriptor/fetch integration. RF001 retains MatrixCancel focusBODY versusReactSetupguideheading. PartsName andPCBgenericInspector implementers active; no canonical parent closure.

## RF-014

A real IndexedDB request error can precede transaction abort or be handled while the transaction commits. The prior observer settled on that intermediate event and dropped closures still installed in the browser. Cancellation before polling also left invalid callbacks. Three real Chromium production regressions failed; five pass after terminal-only settlement and private callback detachment. Independent source review is clear and repair `d5eaafaf` is integrated. Full packaging/public checks remain open, and the original Matrix save abort cause remains unknown.

Retain this ownership rule for the later host lifecycle audit without assuming other observers are defective. [Exact finding and red/green evidence](../../.scratch/dioxus-frontend-tranche-1/evidence/idb-observer-repair-20261002/FINDING.md).

## RF-015

The paired Layout viewer at `7d09d0a60fbb2cc12e259541614b9483ee618d29` rendered its accepted scene from `LayoutPreviewSnapshot`, but derived per-component visibility rows only from the distinct `NativePreviewSnapshot` input. A current Layout scene therefore showed meshes while its Layers menu had zero component controls. The local evidence records the observed DOM and source ownership; the bounded repair selects model rows from the active source and reuses existing renderer IDs, delivery state, and visibility controls.

This confirms the consumer source-selection defect, not a need to clone the shared viewer. After parity, assess whether a private typed viewer-source descriptor should keep scene rows, decoded delivery rows, visibility projection, and accessibility context together. Verify exact-model hide/show and preservation of Case/native behavior in the fresh package before treating the repair as qualified. [F7.3b source inventory](../../.scratch/dioxus-shared-viewer/evidence/layout-canonical-layer-controls-20261002/planning-evidence.md).

### Shared integration and packaging checkpoint — 2026-10-02

RF-001 now records the private-stream handoff template at `ae232927`: exact mount/callback/scope contracts and scoped styles go through one coordinator. This is a process mitigation; shared presentation/Runtime coupling still needs post-port assessment.

RF-009 records the configured 30-agent ceiling versus the observed 11 active slots, and temporary-disk exhaustion from generated targets. Inactive build targets were preserved on larger storage with original-path symlinks; source, browser profiles and failed-project evidence were preserved. Exact-source page-only provider reuse is being specified; provider changes still require full packaging. These observations do not close parity or release gates.

### RF-001: accepted selection and Inspector anchor, 2026-10-02

Sol reproduced a Keycaps defect where warning and size-control projection used the last tree context rather than accepted Add/Toggle/Range selection IDs. Immediate correctness repair is active; later refactoring should clarify navigation-anchor versus editing-selection semantics across Inspector controllers. This does not waive production refresh, paired history or persistence gates.

### RF-009: measured provider reuse, 2026-10-02

The actual matching full build took535.29 seconds and the guarded page-only reuse88.55 seconds. Both route packages retain145 assets including126 unchanged providers, with zero hash mismatches; root/subpath offline editor geometry verified. Explicit partial comparison source792e88af at34731. Broader leaf reuse remains separately reviewed work; shared module/Runtime changes still require a new full baseline.

### RF-009: syntax-aware reuse eligibility

The unintegrated leaf-reuse expansion used line-based Rust registration detection. Independent production eligibility probes found valid module, macro and include changes that its passing guard suite missed. The expansion remains held while token handling and negative coverage are corrected. Later refactoring should evaluate a maintained syntax/dependency audit boundary; no changed provider output has been observed.

### Parts13 custom-component handoff — 2026-10-02

No new refactoring takeaway was observed in the feature-private create leaf and its isolated Parts mount. It reuses the current Runtime operation/outcome observer and accepted `ReplaceDocument`/history path; reconciliation checks both Parts-local query/selection intent generation and the existing monotonic Runtime scope generation, so returning to an equal Scope value after a board/document transition cannot admit an older completion. A mounted Dioxus test verifies the production workspace-generation owner advances once for each transition without subscribing to its own write. The Inspector resolves selected project definitions from the accepted snapshot while still displaying catalogue loading/error status. The shared Parts mount remains coordinator-owned under RF-001, while public paired browser, Undo/Redo and save/reopen evidence remains open under RF-009. See the [implementation handoff](../../.scratch/dioxus-parts-catalogue/evidence/parts13-implementation-handoff-20261002.md).

### RF-001 — joined-wave lifecycle and contextual composition observations

At isolated Parts15 `111647f6`, independent Sol review confirmed that the post-await continuation accessed editor Signals before checking component lifetime, including its stale cleanup path. That unmerged source is held and being repaired with mounted pending/unmount/resolve regressions. This follows the corrected Parts13 effect self-read hazard; neither finding establishes a crash in served `9d34f367`. After parity, evaluate a consistent lifetime-first admission interface and mounted asynchronous owner tests, without generalizing prematurely.

Actual paired browser checks on `34732/9d34f367` also retain current composition gaps: selecting generated Plate leaves the global Case mechanical panel rather than React’s contextual Plate Inspector, and the mirrored-pair form appears in Objects instead of the canvas overlay. Those require current port corrections. They do not prove module size caused the gaps and do not qualify for deferral to the refactor phase. All existing RF IDs and parent acceptance gates remain.

### F3.2c placement chooser — 2026-10-02

No new refactoring takeaway was observed in the bounded Add object/menu correction. It reuses the accepted placement owner, existing layout-target signal, catalogue search and normal placement edit path; unavailable existing-half and geometry-editor actions remain outside this slice rather than appearing as placeholder controls. Public paired placement/history verification remains governed by the current F3.2c evidence packet.

### RF-015 verified public consumer and naming continuation

The private source-selection repair is now independently reviewed, integrated and served in candidate34737/sourceb9e74e37. Paired actual Layout3D visibility toggles remove the same left-J3 display mesh in React and Dioxus. Broader stale/foreign source, readiness and responsive qualification remains open. The same rows expose storage filenames (`1.step` in React, content-hash.step in Dioxus), showing that human-facing model labels remain coupled to packaging identity. Retain naming parity as an open UI criterion; assess distinct display metadata/storage identity after the port. Evidence: [bounded public receipt](../../.scratch/dioxus-frontend-v1/evidence/layout-case-transport-public-20261002/README.md).

### 2026-10-03 combined candidate continuation

RF-001: independent consolidated reviews of `a201a96a` found that explicit Keycaps finding Part selection for a matrix switch conflicts with Layout Inspector admission, which still canonicalizes that switch to Key. Both feature-local checks passed. The current port requires the integrated routing repair and actual paired Inspector journey; the later refactor should unify selection intent and Inspector admission policy. Parts accepted-refresh dirty-draft loss remains an immediate correctness fix, not deferred cleanup. No parent criterion is waived.

### RF-009 — packaging ownership guard correction, 2026-10-03

The a57135a7 helper's 23 passing guard tests did not cover transitive Cargo features, legacy-allowlist precedence over a detected provider owner, or source edges through literal includes/nested modules. Independent production-preflight mutants admitted these shapes; a tiny actual provider compile changed WASM bytes for an admitted included-source edit. The current isolated correction adds expected-red production regressions, resolves supported feature/include/module ownership, gives provider ownership priority, and requires full builds for unsupported forms. Preserve the original helper/history and distinguish repaired guard source qualification from the pending fresh full22 baseline, measured reuse/provider identity, route/offline behavior and parent acceptance. After parity, assess whether Cargo/compiler-owned dependency evidence should replace the bounded source parser; no new RF ID or gate waiver follows from this correction.

The isolated PCB command-pill source at `d01c18af` reuses the shared menu and existing accepted selection, matrix transform, align, snap and Session gesture paths. The shared private transform/align owners now accept an exact Layout-or-PCB route, while the Layout route remains exact and PCB has its own current-owner admission. This is a bounded mitigation under RF-001; strict WASM Clippy and browser-test compilation pass, while root integration and the paired PCB journey remain open. See the [Issue 13 contract](../../.scratch/dioxus-pcb-view/issues/13-canvas-command-pill.md).

### RF-001 / RF-009 — delivery bottleneck and instruction drift, 2026-10-03

The read-only execution audit found six source packets awaiting integration/public qualification, conflicting preimplementation-review prose despite current candidate-first authority, and duplicate current-candidate fields naming older builds. Root now integrates the ready batch, retains former fields as history, and uses one current_progress record. The user directs Layout-first delivery, six queues, one waiting packet/stream and less repeated testing. Existing bug evidence and sufficient unchanged checks are reused; candidate review and all62 parent criteria remain. This mitigates coordination/accounting costs, not a claim of completed parity or a replacement for stable domain identity. The Parts index-key regression and isolated PadRowKeys repair remain recorded with their public acceptance limits.

### RF-001 continuation — accepted drafts and shared interaction composition, 2026-10-03

Actual candidate32a/34743 browser journeys exposed three small composition failures: Outline Undo revived a submitted coordinate draft, the shared Keymap/Keycaps context header overlapped view controls and changed camera framing, and trusted PCB Ctrl-click cleared selection after pointer capture. The coordinate repair `ddc53653` synchronizes changed accepted values while preserving unrelated dirty fields; header/camera and pointer lifecycle repairs are active. After parity, assess common draft lifetimes, contextual layout contracts and a single pointer-selection authority. These observations do not authorize a broad refactor or waive any parent criteria.

### PCB pointer ownership repair qualification — 2026-10-03

RF-001's captured-click finding now has bounded public GREEN on `f2d70ea627d851ea461033b44052bbf3740d37a1` (`34747`): trusted Ctrl-add/Ctrl-remove, plain selection, primary empty clear and keyboard activation pass without changing revision 12. The second RED identified a further concrete boundary: native `stopPropagation` does not stop Dioxus synthetic bubbling, so part pointer-down now stops both. Capture/movement/release/cancel/history code is unchanged and its existing evidence is reused. See the [final repair receipt](../../.scratch/dioxus-pcb-view/evidence/pcb-modifier-click-repair-20261003/final-green.json). This closes the bounded defect, not PCB13 or full F5 acceptance. The next existing PCB02 omission is actual mounted-module Footprints visibility; accepted `SceneDelta.module_scenes` supplies source-owned geometry while the current transient shell has only host-layer and LayoutFootprints owners. Preserve RF-001/RF-006/RF-009 while adding the smallest separate module visibility owner.

### RF-001 continuation — source-owned module presentation, 2026-10-03

PCB14 connects accepted module footprint geometry to a separate transient module Layers owner. The pre-existing resolver supplies host-space pose/side and remapped surface layers; the page applies the footprint transform once and never creates host circuitry. Host visibility intersects source artwork/pads/drills without persisting toggles. Remaining module categories and owning-object navigation stay F5.4 work; only working Footprints controls are added. Compile and changed public qualification are pending. This is a bounded composition mitigation, with no new RF finding or parent closure.

### RF-001 — keycap CAD worker crate boundary, 2026-10-03

The page binary and `boardstudio_web` worker host are separate crates. A crate-private method cannot carry Core-resolved `KeycapSpec` values from the page to the existing packaged `build_keycaps` worker call. The bounded accepted Keycaps STEP slice exposes one dedicated `CadWorker::request_keycaps_step` entry point while keeping the request wire private and leaving `CadRequest`, `CadOperation`, generated contracts, and CAD algorithms unchanged. This is confirmed boundary evidence under RF-001, not a new RF ID or a reason to expose a generic request API. Later refactoring can assess whether the page-specific worker host should move to the consumer crate or remain a narrow library capability. Source contract and remaining browser gate: [BND.1 source contract](../../.scratch/dioxus-keycap-cad-bridge/evidence/source-contract.md).

The Keycaps 3D preview needs the same dedicated crate-boundary facade, but owns a Runtime worker separate from Case generation and STEP export, then composes the accepted fit result into the existing shared viewer. This remains a narrow F6C.5 bridge with private wire and no duplicate Core resolution. Since `build_keycaps` runs synchronously inside the worker, cancellation cannot cooperatively interrupt the kernel call; owner/generation checks and worker termination prevent stale publication. Keep Keymap consumption and actual paired browser/lifecycle acceptance open.

The first served replay on `243aa551` also exposed an effect-ownership mistake in this route: it read the local preview sequence reactively and immediately wrote it, repeatedly restarting preview admission and leaving the shared viewer blank. The bounded source correction uses an untracked read for that effect-owned counter. Candidate `e7742d46` passed one same-fixture 1280×577 Keycaps 3D journey, including cap/legend bodies, per-key color, and global and individual layer hide/show. The retained [receipt](../../.scratch/dioxus-frontend-v1/evidence/keycaps-3d-preview-20261003/RECEIPT.md) preserves the red and green, while Keymap and remaining lifecycle/full-feature acceptance stay open.

PCB contextual inventory update (2026-10-03, ead8fe7c): the generic Objects composition exposed Layout groups and selectors in PCB. The bounded port correction restores flat actual PCB parts and the selected-reference footer while retaining the accepted selection owner. RF-001 records the current generic-tree filtering cost and a later shared outline projection; this is a deferred design observation, not a waiver of parity.

### 2026-10-03 contextual Inspector and draft/async lifetimes

RF-001: actual34758 PCB SW3/R1 prepended a non-reference Position editor before wiring. Root0e546579 removes that private PCB composition; shared selection/helper reuse must not decide workbench control placement. Flat Objects/footer/disclosure are paired GREEN; repaired Inspector delta remains queued.

RF-006: consolidated review found matrix duplicate continuation/recovery and preset drafts could cross logical owners when IDs/settings repeat. Current repairs capture clone/document/session/scope identity and restore only owned navigation. Keymap native Tab still lost focus because pending-save disabled Hold; admitted controls now remain focusable while existing pending handlers reject a second edit. These are current repairs and later ownership-design takeaways, without a broad refactor or repeated unchanged tests.
