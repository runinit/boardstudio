# Post-port refactoring takeaways

**Living register, started 2026-10-02 at migration baseline `c827c4e6`.** The user requested that architectural, design, theoretical and general software-quality issues discovered during the Dioxus rewrite be preserved for a major refactoring phase after the port.

The [machine-readable register](../../.scratch/dioxus-frontend-v1/refactor-findings.json) holds stable IDs, evidence, impact, affected workflows, current mitigations, later proposals and validation questions. This document is its readable index. Findings are observations and proposals, not approved redesigns. A missing screen alone is not an architectural defect; uncertainty is stated explicitly.

## Required practice during the rewrite

Every workflow handoff and independent review must update an existing RF entry, add an evidence-backed entry, or state **“No new refactoring takeaway observed”** with the scope reviewed. Capture small local design problems as well as recurring architectural or conceptual issues. Include source/revision, the concrete example, why it matters, and what would confirm or disprove the proposed explanation.

Keep correctness required for parity in the current slice. Defer broad cleanup and redesign until after the port; a deferred entry never waives a bug, public-contract decision, accessibility requirement or release gate. Record temporary bridges and their intended retirement conditions. Preserve resolved entries with the fixing commit and evidence so the later retrospective includes what worked.

At frontend v1, F9 produces a refactoring handoff grouped by architectural boundary, recurring cause, user impact and uncertainty. Prioritize the future refactor using the accumulated evidence; do not adopt every suggested abstraction automatically.

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
