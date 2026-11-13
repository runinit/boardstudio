# Map: Toolbar-led workbench

Label: planning:map
Status: **M1 planning gate complete; coordination/implementation transfer authorized, then hands-on stop. Later phases are not authorized.**
Base: local `main` @ `cafaf4302635909b59666164ed190fa2137aabc5` (contains `ec0a6165`, `ec26e46d`).
Owner: `plan/toolbar-led-workbench` in `.worktrees/toolbar-workbench-plan`.

## Destination and authority

Adopt the frozen [tool-panels](../../design/workbench-redesign/tool-panels/README.md) and
[Option A](../../design/workbench-redesign/option-a/README.md) direction, not their disposable
JavaScript or sample geometry. Both READMEs and both HTML files remain unchanged.
[ADR-0041](../../adr/0041-toolbar-led-workbench.md) records the confirmed direction; the
[architecture](architecture.md), [inventory](inventory.md) and [verification](verification.md)
make the implementation transfer. [Refactoring seams](refactoring.md) records graph-grounded
preparation steps and the follow-up safety/commit-trigger findings. [PRODUCT.md](../../../PRODUCT.md),
[DESIGN.md](../../../DESIGN.md), [GLOSSARY.md](../../../GLOSSARY.md) and existing domain ADRs
still govern document edits, geometry, links and output claims.

Shared chrome serves **Layout, PCB, Keymap, Keycaps, Case and Parts**, but their tools,
selection models, editors and edit admission remain domain-owned. The quieter topbar is
global, including project/Export surfaces; those pages do not gain irrelevant canvas panels.
Layout has one bounded **42px** accordion strip: Select / Transform / Align / Snap / View,
with Objects and Inspect utilities in the same row. Objects is compact and anchored;
Inspect is an unpinned pop-out. No permanent Objects sidebar, icon rail, pinned Inspector,
backdrop, duplicate Position editor, panel-induced canvas resize or camera reframe.

## Decision round 1 — user answers recorded (planning, not implementation approval)

1. **Shared shell confirmed:** all six workspaces, not a Layout-only migration.
2. **New superseding ADR confirmed:** ADR-0041 replaces affected ADR-0035 shell clauses;
   it does not rewrite the historical rationale or declare the new UI shipped.
3. **Dock retirement confirmed:** resize, pin/collapse, persisted dock-width/mode influence
   and narrow Sheet/tab paths go after parity. Remap Show/hide Objects/Inspector commands;
   preserve unrelated preferences. Do not delete a shared primitive with remaining users.
4. **Larger overflow surfaces confirmed in principle.** The exact proposed destinations are
   in [the flow table](inventory.md#overflow-routing-proposal). The earlier “Y” was not
   approval of an arbitrary replacement wizard or removal of a workflow.
5. **Findings and outline lifecycle confirmed in Inspect**, not another dock or full page.
6. **“Placed from …” confirmed as anchor association only**, never side validation or a
   clearance guarantee; no invented clearance check.
7. **Now approved:** Objects → Outline and canvas-boundary picking select the currently
   active outline, without activating another version, changing shape/outputs or opening
   Inspect. Inspect retains Generated / linked refinement / fixed edited-outline lifecycle
   and explicit active-version changes. Existing `Outline` / `OutlineVersion` contexts can
   remain; no new outline model or independent inactive-version editor.
8. **Now approved:** use actual half/layout names; selecting a half through Objects or canvas
   aligns view-only editing focus. Inspect identifies saved links and existing local/omitted
   exceptions. Focus does not mirror, unlink, swap construction source, edit the document or
   add Undo. Never infer source identity from Left/Right, names or screen position. Mirror
   existing half remains a creation action, distinct from selecting an existing half.
9. **Quieter topbar confirmed globally**, preserving all existing actions and save honesty.
10. **42px confirmed even for narrow/touch.** This is not a general waiver of target sizing,
    keyboard access, contrast or accessibility evidence. The proposed narrow/focus rules in
    [architecture.md](architecture.md#responsive-and-keyboard-contract-proposed) awaited approval
    at that round; the current M1 execution instruction adopts their conservative defaults.

## Tickets and dependency order

001 gates the corrected M1 transfer/recheck. A ready label never overrides a
phase gate. Numbers retain the Layout slices; 009 adds shared chrome, 010–014 domain
adapters, and 015 separates Case Preview's human-approved owner/interface work. Later
adapters stay awaiting-decision until M2 approval and their named decisions/classification.

| Ticket | Owned slice | Direct dependencies |
|---|---|---|
| [001](issues/001-reconcile-design-adr-scope.md) | Direction, corrected M1 transfer/review and execution authorization record | none |
| [009](issues/009-shared-toolbar-panel-host.md) | Shared row, panel/focus/lifetime host, command remapping | 001 |
| [002](issues/002-toolbar-accordion-label-fitting.md) | Layout tools, measured accordion and motion | 009 |
| [003](issues/003-inspect-popout.md) | Layout Inspector, draft reset audit and explicit reveals | 002 |
| [004](issues/004-objects-popover.md) | Layout Objects, outline and half selection | 002 |
| [005](issues/005-retain-objects-capabilities.md) | Layout Add/setup, legacy overflow and real task/draft integration; reusable by PCB | 003, 004, 006 |
| [006](issues/006-topbar-simplification.md) | Global quieter header, no navigation/save loss | 009 |
| [010](issues/010-pcb-workspace-adapter.md) | PCB Objects, Selection/Routing & nets, own tool mounts | 002, 005 |
| [011](issues/011-keymap-workspace-adapter.md) | Keymap layers/keys, binding/macro/encoder ownership | 009 |
| [012](issues/012-keycaps-workspace-adapter.md) | Keycaps defaults, overrides and fit findings | 004, 009 |
| [013](issues/013-case-workspace-adapter.md) | Physical assemblies, Inspector and existing assembly/display View controls | 009 |
| [015](issues/015-case-preview-owner-contract.md) | Separately gated Case Preview generation/command owner and presentation | 009, 013 |
| [014](issues/014-parts-workspace-adapter.md) | Catalog/assembly navigation, authoring and placement | 009, 005 |
| [007](issues/007-responsive-accessibility-acceptance.md) | All-workspace responsive, keyboard, Theme and packaged acceptance | 003–006, 010–015 |
| [008](issues/008-remove-dock-paths-docs.md) | Parity-gated retirement, proof retention and shipped docs | 007 |

[delivery.md](delivery.md) replaces the infrastructure-first phase split with **M1: the
complete actual Layout redesign** (009/002/003/004/005/006 plus Layout-targeted acceptance),
then **M2: five adapters and gated Case Preview** (010–015), then **M3: final acceptance/
retirement** (007/008). The current execution instruction authorizes M1 after its review
corrections, not M2/M3. Stop for hands-on review at M1; an empty host is not delivery.
The coordinator owns full-suite/build launches and shared routing files; no parallel edits
to `presentation.rs`, `workspace_composition.rs` or shared-host code.

## Remaining choices for approval

M1 takes the already recommended Project/Add + palette routing, real existing task owners,
fixed-trigger/action-scroll/narrow/focus literals and session-scoped draft retention as its
implementation target. Existing native blur semantics are preserved by the exact per-path
commit/retention table, not suppressed or weakened to “at most”. The user's current
“Execute the recommendations ... coordination and implementation” is the M1 authorization;
the earlier Yes remains only decisions 7/8.

Still deferred to M2: Keycaps board-default relocation; Case View/Preview interfaces and
44px task controls; classification/update-safety treatment of currently unregistered
Keymap/Keycaps/Case forms. Preserve actual domain triggers until those decisions land.
All Objects picks still close; no global restyling of unrelated shared menus; no automatic
merge of mixed shells to main. New alternatives require approval, not inference.

## Review corrections for the first delivery

Fable's source-backed review is in `review-fable.md`. This table records planning dispositions,
not production proof or automatic acceptance of every suggested alternative.

| Finding | Disposition |
|---|---|
| N1 / 009 owner-dependent proofs | Move real registered-owner/task cases and mutations to 005; 009 tests its production private interface, 002 proves real Layout fitting, M1 requires both without a closure cycle |
| N2 / content shapes | Name unchanged Layout and Case Objects/Inspector Elements; no adapter edits or fake public methods in 009 |
| N3 / mixed shells | One current authority per workspace, target-aware commands/reveals and real browse/return proof; development branch only |
| N4 / dependencies | 005 depends on header routing 006; Parts014 depends on005; graph remains acyclic |
| N5 / inventory | Central 13-site disposition including Export/shortcuts; Case assembly/display View rail explicitly owned by013 |
| N6 / visibility authority | Host stores Objects/Inspector only; typed owners own task/detail visibility, guarded hide, intentional Cancel and navigation admission |
| N7 / Case generation | Separate015 with one observer/command owner and concrete human-approved interface; not on M1 critical path |
| N8 / draft safety | 003 now separately proves hidden registered Inspector safety with a guard-drop mutant;005 retains real setup/script signals+guards; PCB/Parts keep known registrations; unclassified future forms remain gated |
| N9 / commit triggers | Exact real-blur versus no-blur captured-owner outcomes, new dirty-layer proof/mutants in011 |
| N10 / status honesty | M1 choices recorded; later007/008 and010–015 remain awaiting-decision pending their phase/decisions |

Glossary now distinguishes Inspect's label without prescribing creation routing or claiming
old docks are already gone. 002 requires the actual built stylesheet. Optional module/Owner
wording smells do not authorize a new refactor programme.

The [single delta recheck](review-fable-recheck.md) returned Standards PASS and Spec CHANGES
REQUIRED on one missing 003 registered-Inspector safety row. The author added that row and
its separate guard-drop mutant, read the change and passed the requested doc gates. This
disposes the sole mandatory M1 planning gap under the reviewer's stated added-case rule;
it does not rewrite the report, run its future mutation, or authorize another review round.

Carry nonblocking follow-ups into their owning M1 slices:002 folded fitting, section/canvas
and Escape behavior and static-disabled-versus-motion opacity;004 Objects-pick focus per
frozen template;005 equivalent registered existing-half/mirrored setup inputs;009 confirm an
actual hide-refusal consumer before retaining an optional refusal variant. Use glossary
Unresolved draft outcome vocabulary while preserving actual code-variable names. The
review report retains the exact source/authority limits. Named M1 acceptance/receipt ownership
is now in delivery.md; future007/008 statuses and tracked approval wording are corrected.

## Planning validation and transfer gate

The refactoring refinement passed `python3 scripts/check.py repo` (including ADR index:
41 records) and `python3 scripts/check-doc-links.py` across 763 Markdown files. Logs:
`.evidence/toolbar-plan-refactor-repo.log`, `.evidence/toolbar-plan-refactor-links.log`.
The earlier snapshot also passed whitespace and its 14-ticket dependency/header checks.
The current correction adds015 and fixes005/014 dependencies; the new15-ticket graph is
self-read as acyclic, with001 on every implementation path. Current doc checks are recorded
in001 after the correction/recheck, not inferred from the older763-file receipt.
No production Rust/browser/build/mutation checks ran; implementation proofs remain planned.

The original Codex access failure (0/23 earlier files) is historical. Fable subsequently used
native Gortex and read24/24 candidate documents, reporting CHANGES REQUIRED on both axes.
Its frozen HTML/selected authority-source gaps and findings are named in review-fable.md.
The completed delta recheck covered27/27 candidate documents and the frozen/required M1
authority sources; its sole mandatory case has the addition/disposition above. 001 is resolved
for M1 transfer only. Current repo/link checks pass767 Markdown files including the report
(41 ADRs,64 Core registration files); receipts are `.evidence/toolbar-plan-m1-repo.log` and
`.evidence/toolbar-plan-m1-links.log`. A low-risk document graph contract is not runtime
proof or review approval. No unrelated MCP configuration was changed.

## Risks and deliberate limits

- Keep-mounted alone does not preserve drafts: Outline owner effects and Keymap editor keys
  can reset them. Retained drafts must also retain live update-safety registration: dropping
  its guard while parking text can let Update Now miss unresolved work. Draft-owner audits,
  registered-owner lifetime and stale/deleted-owner rejection are mandatory.
- Thirteen `pin_inspector_on_desktop` call sites plus non-pin reveal paths have a central
  disposition; actual migration must preserve target generation and not turn picks into reveals.
- A generic Layout tree cannot represent Case's physical-assembly actions or Parts' catalog
  identity. Share chrome, not a new universal object model.
- Narrow 42px controls, long names, zoom, theme contrast and SVG chrome need fresh evidence.
  Frozen mockup checks are not production accessibility/CAD/export/fabrication proof.
- Legacy geometry scripting is out of product scope (deprecated ADR-0029), but exists on this
  base. Preserve existing access/safety in this migration; a separate approved removal owns
  retirement. Do not introduce or advertise new scripting capability.
- No geometry algorithms, saved formats, arithmetic, manufacturing readiness or link rules
  change. All behavioural proof statuses remain planned until actual implementation evidence.
