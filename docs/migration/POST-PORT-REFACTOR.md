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
| [RF-005](#rf-005) | Geometric edit planning lives in frontend helper policy | confirmed remaining TS policy; target placement is a hypothesis | medium | F3, F6 |
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

The current presentation spans 1,399 lines and Runtime 960 lines, with shell, workspace routing, data/service actions and lifecycle integration sharing these modules. The reference Workbench is 1,426 lines. Length alone is not a defect; the concrete cost is that independent workflow edits repeatedly touch the same ownership surface.

**Impact:** Concurrent changes can conflict, and a one-to-one port could preserve broad coupling between workspace controls and service lifecycles.

**During the port:** Use private feature modules with one coordinator owning shared composition; avoid a general framework before useful screens land.

**After the port:** After parity, assess stable workspace/host orchestration modules with narrow interfaces and one authoritative session.

**Validate:** Measure cross-workflow files changed for a representative feature and show that isolated workflow edits/tests no longer require unrelated shell/runtime changes.

Evidence: [presentation.rs](../../web/src/presentation.rs), [runtime.rs](../../web/src/runtime.rs), [Workbench.tsx](../../app/src/ui/Workbench.tsx).

## RF-002

**Internal browser host types are exposed as crate APIs** — architecture / API design. confirmed observation; future cost is a design risk.

web exports cad_jobs, renderer_host and several host modules publicly; CadOperation/CadRequest and wrapper methods are public. Adding a seemingly internal operation can therefore widen a Rust crate surface even when wire/file semantics stay unchanged. The page binary imports that library, so a pub(crate) method there is not callable by the binary; private wrapper placement must respect the real crate boundary.

**Impact:** Implementation detail and intentional compatibility contract are hard to distinguish; private adapter evolution can trigger unnecessary broad API decisions.

**During the port:** Place private wrapper/wire types in the same crate as their consumers, or use already sufficient public facades. Verify library-to-binary reachability; preserve existing APIs.

**After the port:** Audit actual consumers, explicitly separate stable public contracts from host internals, and propose compatible deprecation/module boundaries rather than silently shrinking visibility.

**Validate:** List real external consumers and compile compatibility fixtures; prove internal operation changes no longer alter the intended public surface.

Evidence: [lib.rs](../../web/src/lib.rs), [cad_jobs.rs](../../web/src/cad_jobs.rs), [renderer_host.rs](../../web/src/renderer_host.rs).

## RF-003

**CAD engine capabilities and host protocols drift apart** — architecture / capability discovery. confirmed adapter gap.

Rust WASM already exports build_keycaps, but the Dioxus host/worker operation set covers case preview/exact/STEP/read only. The initial review mistook the absent host path for a missing provider until the engine export was traced.

**Impact:** Capabilities can be overlooked, duplicated, or falsely described as backend work; behavior and cancellation can diverge between host wrappers.

**During the port:** BND.1 proves the private keycap worker adapter using the existing engine; retain reference chunk/yield and revision/ID semantics.

**After the port:** Create an explicit capability-to-host contract map and shared conformance fixtures for required engine operations; consider reducing duplicated wrapper protocol knowledge.

**Validate:** Every required capability has a verified engine→worker→host→UI trace, same fixture outputs and cancellation/error disposition.

Evidence: [keycaps.rs](../../cad/wasm/src/model/keycaps.rs), [lib.rs](../../cad/wasm/src/lib.rs), [index.ts](../../cad/src/index.ts), [CaseClient.ts](../../app/src/CaseClient.ts), [cad_jobs.rs](../../web/src/cad_jobs.rs), [cad_worker.rs](../../web/src/cad_worker.rs).

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

Resize/reflow computes canonical linked-half deduplication, axis projections, spacing and center preservation in TS UI helpers. No equivalent callable Rust resize/reflow controller was found, although core owns matrix projection and accepted edits.

**Impact:** A component-only rewrite would lose behavior or leave hidden TS logic; moving the policy carelessly could duplicate geometry authority.

**During the port:** F6C.3 ports the existing policy into a private Rust frontend controller with preserved helper/public workflow tests and one normal edit/history path.

**After the port:** After parity, decide whether interaction planning belongs in a dedicated application service using core geometry primitives, with explicit draft/commit ownership.

**Validate:** Compare identical resize traces across pointer/keyboard, mixed selections and linked boards; no semantic drift, duplicated calculations or extra history steps.

Evidence: [planKeycapResize.ts](../../app/src/ui/planKeycapResize.ts), [keycapReflow.ts](../../app/src/ui/keycapReflow.ts), [interactions.rs](../../application/src/interactions.rs), [layout.rs](../../core/src/matrix/layout.rs).

## RF-006

**Canonical, physical-instance and isolated sample scopes are easy to conflate** — design / coordinate and identity model. confirmed distinct scopes; abstraction improvement is a hypothesis.

Layout/Keymap/Keycaps use canonical boards; Case projects a physical instance; Parts builds an isolated sample. Authored Case STEP uses canonical saved bodies while generated mechanical export explicitly projects the instance. All may share viewer and artifact code.

**Impact:** A generic board/instance argument or copied projection rule can mirror the wrong geometry, leak a sample into the document, or export the wrong scope.

**During the port:** Use one shared viewer with explicit consumer inputs; preserve each reference projection and captured identity. Test cross-scope switches and stale replies.

**After the port:** Evaluate typed scope/projection inputs and coordinate-frame invariants for view, edit and export consumers; separate display projection from saved-document identity.

**Validate:** Table-driven fixtures for front/back/split/instance/sample scopes plus properties for stable IDs, reflection count, unchanged canonical document and correct outputs.

Evidence: [Workbench.tsx](../../app/src/ui/Workbench.tsx), [LibraryWorkspace.tsx](../../app/src/ui/LibraryWorkspace.tsx), [cases.ts](../../app/src/exports/cases.ts), [cad_jobs.rs](../../web/src/cad_jobs.rs).

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
