# Source decisions for the proposed first tranche

Checked against integration HEAD `530e60dd`, executable source `f44a3d1b`, and
React reference `5a472a9426e6e38993361da402cd4ec730feb369`. The original checkout
remains at the reference with only its pre-existing untracked scratch content.
The continuation has no CodeGraph index; known files were read directly. The
indexed original checkout was queried through CodeGraph before targeted source
reads. Both Luna source audits are preserved under evidence.

These source paths belong in supporting dispatch evidence, not durable ticket
prose. The public UI and existing session/provider behavior remain the highest
acceptance seams; no new test-only application API is proposed.

## Decisions that changed the initial packet suggestions

1. **A real prefactor, then independent lanes.** `web/src/main.rs:1` declares
   presentation/runtime in the page binary; `web/src/lib.rs:9` exports a separate
   host library. `web/src/presentation.rs:58` creates one Runtime/subscription;
   `web/src/runtime.rs:50` owns its single Session. Ticket 01 keeps private
   composition in that page crate, mounts actual existing controls, and includes
   preservation evidence. It does not introduce a generic framework. Review the
   exact callable contract at dispatch; this document is not that proof.
2. **The saved library and search are independently useful.**
   `app/src/ui/ProjectLibrary.tsx:58` catches damaged preview data, and `:114`
   onwards handles listing, failure/retry, current-document de-duplication and
   sorted filtering. `web/src/presentation.rs:551` currently renders basic
   name/ID buttons. `web/src/runtime.rs:764` already guards saved opens by
   sequence. Ticket 02 delivers safe real cards/open/list state; 03 adds local
   search. Neither needs new/delete/archive work from F2.2.
3. **The reference catalogue has 19 demos, not two.**
   `app/src/demos/keyboards.ts:7` combines three Sofle variants, fifteen measured
   layouts and VIK module review; `app/src/demos/sofle.ts:7` enumerates its variants
   and `app/src/demos/keyboard-layouts.json` contains the measured layouts.
   `web/src/presentation.rs:551` exposes only Sofle and REVIUNG41 fixture controls.
   Tickets 04–06 cover all families. This is an implementation coverage gap, not
   evidence that missing UI alone is poor architecture. The actual demo document
   construction/provider path needs private adaptation or verified generated data;
   adding card labels alone cannot complete F2.1.
4. **Demo copy identity needs a regression oracle.** React demo constructors
   create a new project ID (`keyboards.ts:17`, `sofle.ts:16`), whereas Runtime
   fixture opening (`runtime.rs:668`) passes through archive import (`:720`),
   whose parsed project is submitted unchanged (`:744–763`). Static fixture
   replay must not be assumed to create a new identity. This is a source-supported
   risk requiring public repeated-open characterization, not a claimed browser
   reproduction. Ticket 04 proves fresh-copy semantics without changing ordinary
   archive-import semantics; confirmed defects go to Astra High.
5. **Panels have two breakpoints and actual interaction policy.**
   `app/src/ui/WorkspacePanel.tsx:3–35` defines modes, optional local preferences,
   width bounds and 980/820 px compact breakpoints. `:64–103` owns reveal/focus,
   idle-hide and canvas-width constraints; `:125–138` owns 20 px keyboard resize
   and pointer capture. Tickets 07–09 reuse one panel state owner. Resize and
   compact drawers need modes/preferences but do not block each other. Optional
   storage failure retains defaults; no new warning UX is proposed.
6. **Matrix scope is richer than selected part IDs.**
   `app/src/ui/useWorkbenchTree.ts:166–245` projects matrices, groups, empty slots
   and cell components. `application/src/session.rs:102` exposes selected part IDs
   and an anchor but no equivalent semantic matrix/row/column context. Its
   SelectParts validation (`:562–579`) accepts actual document part IDs. Ticket 10
   proves a private context adapter while Session remains the selected-part
   authority. Fake empty-cell IDs must not be sent as real part selections.
7. **Rectangular range differs from the current Rust call path.**
   `app/src/ui/useWorkbenchSelection.ts:37–54` filters the row/column rectangle,
   retaining its anchor on a successful Shift range. Dioxus passes visible IDs
   (`web/src/presentation.rs:1046`), and Session selects their linear interval
   (`application/src/session.rs:602–625`) then updates its anchor. Ticket 12 first
   demonstrates the mismatch publicly, then fixes the private interaction path
   with existing events and independent review. It is routed to Astra as a
   behavioral repair; broad application API redesign remains deferred.
8. **Outline navigation has real side effects.**
   `app/src/ui/useWorkbenchTree.ts:143–157` builds Generated/version/bridge rows.
   `app/src/ui/Workbench.tsx:669–680` clears part scope/selection on outline entry;
   explicit version selection invokes the outline edit path; bridge selection
   fits the camera. Ticket 11 preserves those effects and delivers real selected
   context/activation/fit. It cannot claim an already complete Dioxus outline
   editor or blanket revision neutrality. Full outline authoring stays in F3.4.

## Coverage, boundaries and refactoring handoff

The ticket-to-parent table in PROPOSAL and the machine proposal account for the
four selected parents. Every child includes its affected integration and checks.
The existing parent graph remains unchanged; this proposal adds precise child
edges, not blanket F2-before-F3 dependencies. The final frame/tree join gets
combined evidence without postponing basic scope correctness to a last ticket.

Existing RF-001/RF-002/RF-006 remain relevant: shared composition ownership,
actual crate reachability, and scope interpretation. New source observations
extend RF-005 (interaction policy), RF-008 (archive versus demo-copy lifecycle)
and RF-009 (catalogue/acceptance accounting). Their present mitigation is recorded
in the corresponding tickets. Structural cleanup remains the later refactoring
phase; none of these notes declares a bug repaired or a milestone accepted.

## Planning provenance and limits

Requested author agents: `tranche_projects_panels` and `tranche_layout_tree`,
Luna High. Independent proposal review uses Astra High. Effective service tier
was not observed; no model/provider/global configuration was changed.

Both authors initially proposed broader chunks. The coordinator split saved
search and demo families, corrected outline selection/camera side effects, and
required safety from the first actionable slice. One source report initially
searched only the feature directory for root constraints; it was corrected after
reading the actual root file. These reconciliations are retained in the reports.

No application code, parent issue, parent task status, saved project, runtime
configuration, build or browser state changed during this proposal. Documentation
validation and independent planning review do not prove frontend parity.
