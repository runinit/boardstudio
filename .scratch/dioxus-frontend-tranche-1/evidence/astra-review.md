# Independent Standards and Spec review: first frontend tranche tickets

Reviewed 2026-10-02 against clean baseline `530e60dd96403ff7e9b9e00fa95339b32276836c` in the continuation worktree. Requested reviewer profile: Astra High. Effective runtime service tier was not observed. This review used the explicitly invoked `to-tickets` skill, current local issue-tracker/model rules and relevant migration constraints.

**Standards verdict: no material issues. Spec verdict: no material issues.** The 12-ticket proposal is suitable to present for the user's breakdown approval. It is not approval to publish the tickets or implement them, nor evidence that a frontend parent is complete.

## Reviewed scope and evidence

Read PROPOSAL.md, ACCEPTANCE.md, SOURCE-NOTES.md, proposal.json, all twelve draft tickets, the four parent acceptance contracts, and the appended RF register/takeaway changes. Targeted source checks used the pinned React reference for ProjectLibrary, demo construction, panel settings/resize, semantic tree rows, selection and outline/bridge actions, plus the existing Rust Session selection and Runtime fixture/archive paths. I did not repeat the full Luna audits, run browser/build/application tests, edit the repository, or launch agents.

## Findings and disposition

No blocking or material corrective finding remains.

The proposal follows the skill's vertical slicing rule. Ticket 01 is an explicit bounded behavior-preserving prefactor, with real mounted controls and preservation checks. Subsequent tickets deliver integrated public actions rather than horizontal components, mock-only UI, or deferred integration. Tickets 02 and 10 carry their own stale/scope safety; the later range repair cannot be used to defer baseline correctness. Every ticket inherits paired evidence, affected checks and independent Standards/Spec review. The source-specific paths are in supporting notes; durable ticket bodies state behaviors and boundaries.

The dependency graph is coherent: 01 opens three independent lanes (02, 07, 10); 03 and 04 depend on the real library/card/open integration; 05 and 06 extend the tested fresh-copy action from 04; 08 and 09 extend one panel state owner from 07 without depending on each other; 11 and 12 extend the semantic tree/context adapter from 10 without depending on each other. File coordination remains scheduling, not a fabricated product edge. No whole-F2, full-outline-editor, PCB or Case dependency has been introduced.

All four selected parents are accounted for. INT.1 is covered by 01; F2.1 by saved cards/open/error states in 02, search in 03 and all demo families in 04–06; F2.3 by modes/preferences, resize and compact focus in 07–09; F3.1 by semantic hierarchy, outline/version/bridge actions, modifiers/ranges and scope behavior in 10–12. A combined frame/tree/open/edit/reopen trace covers affected joins while each ticket still includes its own integration. Parent statuses and remaining acceptance obligations stay unchanged.

## Sizing assessment

The slices are plausible fresh-context assignments with the stated dispatch packet and existing source references. Ticket 03 is intentionally small; separating it from safe real project opening gives an independently useful path. Tickets 04 and 10 carry the most design risk but do not presently require a split: 04 establishes a single demo-copy lifecycle with three closely related variants, and 10 establishes one semantic tree/context mapping with existing Session selection authority. Any confirmed copy-identity bug must remain a separately reviewed Astra repair within that work, not an unlabelled Luna behavior change.

Ticket 05 covers fifteen data entries but shares one existing measured-layout construction pattern and the copy lifecycle already delivered by 04. Reuse existing authority or verified generated data and validate every mapping; representative interaction traces supplement rather than replace all-entry open/fidelity checks. If implementation discovers a different provider boundary or algorithm, narrow the affected packet before dispatch instead of silently expanding this ticket.

**Ticket 11 is sufficiently bounded and has no demonstrated F3.4 blocker.** The reference opens outline context, clears part selection, activates Generated/saved versions through the outline edit path, and fits bridge geometry. The ticket requires those real actions and minimum usable context presentation. It explicitly excludes full construction, refinement and settings forms. The implementation must display a meaningful selected context and actual shape/fit result, not a link to absent editing controls. Its dispatch proof can identify a genuine missing boundary if one exists; that is not evidence that the entire editor is already a prerequisite.

## Targeted semantic checks

- ProjectLibrary places the current document once ahead of name-sorted saved records, performs trimmed case-insensitive local search, preserves demo independence, and has local list failure/retry. Tickets 02/03 describe those semantics accurately.
- React demo constructors allocate fresh project IDs. The candidate fixture path reuses archive import and submits parsed identity unchanged. The proposal correctly treats this as a source-supported copy-identity risk requiring public reproduction; it does not claim an already observed overwrite or alter ordinary archive semantics. Ticket 04 routes a confirmed defect to Astra.
- Reference panel bounds are 200–420 and 280–480 CSS px, breakpoints 980/820, and resize steps are side-correct 20 px. The proposal preserves optional preference storage, hidden-content focus exclusion, the opposite pinned-panel constraint and the absence of invented rollback behavior.
- React semantic matrix/row/column/key context includes empty slots, while Session validates actual document part IDs. Ticket 10 explicitly prevents fabricated IDs becoming selected parts.
- React Shift selection computes a same-matrix rectangle and retains the successful anchor; Session's current Range path consumes a flattened interval and updates its anchor. Ticket 12 is correctly classified as a behavioral repair owned by Astra, with red regression and a different reviewer. It does not absorb F3.3's unrelated small-drag threshold defect.
- Outline activation can revise the document, and bridge selection can move the camera. Ticket 11 correctly distinguishes these from revision-neutral disclosure and ordinary panel visibility.

Resolved clarification: re-read the final ticket 07 and machine-proposal wording. Idle-hide is verified with pointer/focus now; active-resize suppression is exercised in ticket 08, and ticket 07 explicitly does not depend on resize completion. This removes a possible hidden reverse dependency without adding an edge.

## Approval, evidence and refactoring

The drafts and manifest remain pending breakdown approval. Publication to one local issue file per ticket comes only after the skill's explicit user approval step. Parent issues are untouched. Public visibility/API/schema changes, actual AT, carried performance/resource failures, production cutover and unrelated configuration changes retain their existing gates. No new requirement is represented as a passed executable check.

The RF additions are evidence-backed and preserve existing IDs, statuses and original evidence. RF-005 appropriately adds semantic selection/anchor ownership, RF-008 distinguishes archive import from fresh-copy intent without claiming browser reproduction, and RF-009 records complete demo accounting and real outline side effects. Their current mitigations remain required in the tickets, while broader ownership/consolidation work remains deferred. No additional independent refactoring finding was observed in this review.

## Reviewed SHA-256 snapshots

| File | SHA-256 |
| --- | --- |
| `.scratch/dioxus-frontend-tranche-1/PROPOSAL.md` | `a6565f5ab31a847b61177c845fed0202e5abdba94fd7c7074233ab4d06593cac` |
| `.scratch/dioxus-frontend-tranche-1/ACCEPTANCE.md` | `e18e8420acce90712a190e722014f9b9dd5204f3ba785cd23aa4d134a6906a01` |
| `.scratch/dioxus-frontend-tranche-1/SOURCE-NOTES.md` | `edc418eac66a769d62be66a404557db11d1842b5cfed0d4f9e032874762d50e0` |
| `.scratch/dioxus-frontend-tranche-1/proposal.json` | `45a9cd220222bb33ec6034ed96d956fef648ef3c68941cae2eeb642482d383da` |
| `.scratch/dioxus-frontend-tranche-1/drafts/01-private-workspace-composition.md` | `f36062a4458cac2fb68c815f86cdee6196d2bd11606fa2f7fa1e96f9474dd2f6` |
| `.scratch/dioxus-frontend-tranche-1/drafts/02-saved-keyboards.md` | `38cb2ee11ad4c7f89a0d1a170966f512c5a2b52a6bfa5b5167c51273a0f79ab4` |
| `.scratch/dioxus-frontend-tranche-1/drafts/03-saved-search.md` | `b849bde49d7402d3dac16d94fbf8bbfe2949aac820b9394925eb2cd7652ea1f8` |
| `.scratch/dioxus-frontend-tranche-1/drafts/04-sofle-demo-copies.md` | `ca43033f1a5a4fe6d875f6f35798c1aedd78f9858a6f84336eeb5eb8c7474670` |
| `.scratch/dioxus-frontend-tranche-1/drafts/05-measured-demo-copies.md` | `c4c94030234cd4d2bdbae73d0ee912ae7b34c3e8c11ee2e3bcc144f8c6f1c9eb` |
| `.scratch/dioxus-frontend-tranche-1/drafts/06-module-review-demo.md` | `2bba079bff51d0e43a7de781518caddea3218a1f8132be86955a074ad5bbcda2` |
| `.scratch/dioxus-frontend-tranche-1/drafts/07-desktop-panel-modes.md` | `e10d1301cffaf7eb957723e5d448f8cdca99ec619ca3231b3831d4dbf184d634` |
| `.scratch/dioxus-frontend-tranche-1/drafts/08-panel-resize.md` | `95cb243b0e97e166d66164f992582dc5a7b653d659408033c0e4032eb104654c` |
| `.scratch/dioxus-frontend-tranche-1/drafts/09-compact-drawers.md` | `6a4eb092f44aa46cabb6241a7d64a3a1eb3ad4262c668d4bc53a911f04886c56` |
| `.scratch/dioxus-frontend-tranche-1/drafts/10-layout-object-tree.md` | `1f17d14a2ee8ee0df17034c5134060b462396a87c1e8fe5649a265e63126836a` |
| `.scratch/dioxus-frontend-tranche-1/drafts/11-outline-tree-navigation.md` | `f9623a3f704858b366fb3d741449dfcf0bface8192570628f536243172ef9174` |
| `.scratch/dioxus-frontend-tranche-1/drafts/12-rectangular-selection.md` | `a60bab8ad188b69dfbaa12440b4ce0413d745f367d3aac2ce2c577d98a412add` |
| `.scratch/dioxus-frontend-v1/refactor-findings.json` | `df4cb1a22142725137211c5df4935aa6ca1ce863d3e316e76c157ebb2bd789ea` |
| `docs/migration/POST-PORT-REFACTOR.md` | `168fdcb5b27a1e7b9f4c4962989f4d326edc407093df3eac32f21572de07fff9` |
