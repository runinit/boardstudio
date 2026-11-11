# Fresh combined Spec + Standards review and additional planning (Fable, 2026-11-10)

Reviewer handoff for the toolbar-led-workbench plan. **Recommendations only.** This file
approves nothing, resolves no ticket, authorizes no phase and amends no plan file.

- Fixed point: working candidate in `.worktrees/toolbar-workbench-plan` vs
  `cafaf4302635909b59666164ed190fa2137aabc5`. `HEAD` equals base; the candidate is uncommitted.
- Access: native Gortex MCP, explicit worktree view on every call, `exact=true` served.
- Candidate: 24 files, all read in full (list at the end). This report is outside the candidate.
- No tests, builds or mutations were run. Evidence is read-only.

## Standards

Sources: `AGENTS.md`, `CODING_STANDARDS.md`, `docs/agents/{issue-tracker,triage-labels,domain}.md`.
`CONTRIBUTING.md` does not exist.

Documented-standard breaches:

1. **Blocks — tickets 003, 005, 012, 013 `Status:`.** `triage-labels.md`: `ready-for-agent` =
   "Fully specified, ready for an agent to start"; `issue-tracker.md`: the boundary is "fixed
   before implementation". 003 says its preconditions are not final ("Before finalizing these
   preconditions, 001 must resolve"), 005 implements "only the approved" table that is not
   approved, 012 is "Subject to 001 approval", 013 needs a contract sketch approved. Use
   `ready-for-human` until the named choice is recorded.
2. **Blocks — 009 proof rows "Pending close", "Hidden draft safety", "Registration scope".**
   `issue-tracker.md`: a boundary row must be meetable in its slice; `CODING_STANDARDS.md`
   ("Synthetic data, production behavior"): new test-only seams need approval. These rows
   demand a "real task" and a "registered explicit-submit draft", but no registered owner or
   task is hosted by the new shell until 005/010/014. 009 can only meet them with a fixture
   its own text forbids.
3. **Follow-up — 001 proof map.** Status vocabulary is `planned | reused | RED | GREEN | proof
   gap` with a cited log. "Documentation integrity ... · native · planned" contradicts the
   ticked acceptance box and the cited passing logs.
4. **Follow-up — `domain.md` "Name a domain concept as GLOSSARY.md defines it".** GLOSSARY:
   "Inspect is the action that reveals them"; the plan uses Inspect as the surface noun
   ("Inspect pop-out", `WorkbenchSurface` Inspect). GLOSSARY also calls Dock "the earlier
   workbench shell" while DESIGN.md says docks are "the shipped baseline".

Heuristic smells (judgment, not breaches):

- **Divergent Change, follow-up** — 009: "owns one bounded 42px row, shared fitting/clip
  mechanics, anchoring/clamping, the proposed `Option<WorkbenchSurface>` arbiter and
  focus/dismissal/inertness". Five reasons to change in one private module and one ticket.
- **Duplicated state / Middle Man, follow-up** — architecture.md: "Objects, Inspect,
  ToolDetail and Task as mutually exclusive top-level UI categories" while "Domain workflow
  content, IDs and admission stay with existing owners". Task/ToolDetail visibility is then
  held twice.
- **Shotgun Surgery, follow-up** — "Adapter tickets own their callers; the coordinator
  applies shared-file hunks serially": one reveal policy spread over 13 call sites and six
  tickets.
- **Mysterious Name, follow-up** — `Owner:` means a branch in 001 (AGENTS.md worktree rule)
  and a code area in 002–014 ("Owner: Layout toolbar/transform/align modules").

Counts: 2 blocks, 2 follow-up breaches, 4 smells. Worst: 009 rows that cannot be met in 009.

**Standards verdict: CHANGES REQUIRED.**

## Spec

1. **Blocks — reveal inventory undercount.** Constraint: "Preserve every existing workflow
   through a named destination." Candidate (architecture.md, inventory.md, map.md): "all ten
   callers in `web/src/presentation.rs`". Source has 13 call sites: the ten listed plus
   `export_workspace.rs:109`, `:118` and `workspace_navigation.rs:42` (`InspectorActions`).
   No ticket owns the Export pair.
2. **Blocks — missing Case inventory row.** Constraint: "One bounded 42px toolbar"; no second
   permanent action row. `shared_viewer.rs:2930/3090` renders Assembled / Exploded / Section,
   which DESIGN.md calls "a wrapping rail above the drawing". inventory.md's Case table has no
   row or destination for it.
3. **Blocks — commit-trigger clarification does not reconcile the proofs.** Pending item:
   "blur-submit vs explicit-submit". Candidate (refactoring.md): "at most the existing commit
   to the captured owner". "At most" is a weak oracle: zero commits plus a discarded draft
   passes. In `KeymapLayerControls` the `dirty` flag is component-local and `name_draft` is
   one signal not keyed by layer. Hide without a blur event, pick another layer, then blur:
   layer B is renamed with A's text. Row missed: 011 "Editor lifetime" lists
   binding/macro/encoder only; no row holds a dirty layer name across a close.
4. **Blocks — retained drafts with no update-safety owner.** Constraint: "Drafts survive
   panel/selection/workspace switches." 009 "Hidden draft safety" covers a "registered"
   draft only. `DraftOwnerRegistration` has 18 production sites in layout, parts, pcb,
   ui-shared and the page inspectors, and none in the keymap, keycaps or case crates. 011–013
   promise workspace-round-trip survival without saying whether Update Now may reload over
   those drafts. I did not verify whether those editors hold explicit-submit drafts.
5. **Follow-up, human decision — multi-select.** "Picking closes Objects" vs architecture.md
   "modifiers retain current multi-selection intentions. A pick closes the popover." Add /
   Range / Toggle becomes one pick per reopening.
6. **Follow-up — scope creep.** Constraint covers label movement. Candidate: "Apply the same
   no-opacity rule to panel transitions ... Nested copied controls must not sneak in a fade",
   which reaches shared `ui-shared` controls used across the app.
7. **Follow-up — pending choice stated as vocabulary.** GLOSSARY Objects: "distinct from
   creating new objects" presumes the unapproved overflow routing.
8. **Follow-up — weak oracle, 002 "Bounded fitting".** Precondition says "font ready" but not
   that the built stylesheet is loaded in the mounted page suite; an unstyled or
   fixture-styled DOM can pass 42px. 007's contrast row does name the real stylesheet.

Not assessed: literal fidelity to `template.html` and `toolbar.html` (unread, see limits).

Counts: 4 blocks, 4 follow-ups. Worst: the layer-name rebinding path with no proof row.

**Spec verdict: CHANGES REQUIRED.**

**Acceptance readiness: BLOCKED.** Unmet: 001's review criterion now has findings to
dispose; human approval of the pending choices; the six blocks above.

## Additional planning

### Focused questions, answered

- **Typed routing.** Confirmed: `workspace_composition.rs` is private, typed per workspace, and
  returns `Element`. `workbench_shell.rs` does not exist. Reusing the seam is right. No
  registry is needed.
- **DockLayout.** Confirmed: `{children}` renders first in a stable position; both tab panels
  stay mounted with `hidden` + `inert`. The plan's "preserve child identity" is accurate.
- **Draft registry.** One registry is enough, but "retain the RAII registration" understates
  the work. The guard and the `is_unresolved` closure both live in the editor component and
  read its signals. An unmounted editor cannot keep either. Survival needs keep-mounted, or
  hoisting signals and guard together to an owner above the branch (the pattern
  `LayerEditsContext` already uses). Session invalidation is already handled: `poll` prunes
  any entry whose `(document, SessionEpoch)` scope differs.
- **Keymap blur.** Not reconciled; see Spec 3.
- **`Option<WorkbenchSurface>`.** Sound for Objects/Inspect. Task and ToolDetail are the seam
  problem; see N6 and the seam alternatives.
- **CasePanel.** Render-only extraction is not enough. Readiness
  (`mechanical_export_ready`), `on_generate`/`on_cancel`, the export click's owner re-checks,
  the automatic-generation effect and `use_commands` are all computed in the component body,
  under the canvas. `WorkspaceToolbarInput::Case` is a unit variant. See N7.
- **009 vs 002.** Ownership is coherent on paper but serial in practice: 002's mutations run
  against 009's file, and 009 has no fitting row. See N1.

### Necessary plan corrections

| # | Correction | Tickets | Proof requirement |
|---|---|---|---|
| N1 | Say that 009 closes with the fitting engine exercised only by 002, and that the foundation phase is the acceptance unit. Move the three real-owner rows and their mutations out of 009 to the first ticket hosting a registered owner or one-shot task (005: matrix setup or legacy scripts). 009 keeps arbiter-level rows against a labelled in-module fixture. | 009, 002, 005, verification.md | Same oracles, runner changed to 005's page files |
| N2 | Name 009's "two real content shapes": the unchanged `objects`/`inspector` Elements of Layout and Case. No adapter edits in 009. | 009 | Canvas-invariance row runs on both |
| N3 | Add an interim rule: until 008, each workspace has exactly one panel authority and Show/hide commands route by workspace. The plan's "do not retain a second dock-state authority" is false while shells are mixed. | 009, 006, 005 | "Commands" and "Browse/return" rows parameterized over one migrated and one unmigrated workspace |
| N4 | Fix dependencies: 014 needs 005 (its creation entries live in the Add chooser) or its own commands; 005 touches `header.rs` and should list 006. | map.md, 005, 014 | Graph stays acyclic |
| N5 | Record all 13 reveal sites and assign the three unlisted ones (suggest 006). Add the Case assembly-view rail to the inventory with a destination or an explicit retain decision. | inventory.md, architecture.md, 006, 013 | Per-site disposition table |
| N6 | Add two transition rows: workspace or board change while a task refuses to close, and a task closed by its own domain control. | architecture.md, 009 | Oracle: host surface and domain task state agree after every transition |
| N7 | Split 013 into 013a (tree, Inspect) and 013b (Preview controls, gated on the contract). State the contract invariants: one automatic-generation observer and one command registration, however many views mount. | 013, map.md | Entering Case starts exactly one generation with both consumers mounted; mutation: a second observer |
| N8 | Each adapter lists its draft owners and chosen mechanism (keep-mounted or hoisted), and whether unregistered drafts are in scope. Blocker text must name the workspace, since the editor may be off screen (`workspace` is diagnostic-only today). | 003, 010–014 | Update Now row per adapter that retains an explicit-submit draft |
| N9 | Replace "at most" with a per-dismissal-path table (Escape, toggle, outside pointer, Objects↔Inspect, layer pick, workspace switch): exactly one commit to the captured layer, or draft retained as that layer's. | refactoring.md, 011 | New 011 row plus mutation: rename fires against the newly selected layer |
| N10 | Relabel 003, 005, 012, 013 `ready-for-human` until their named choice lands. | issues | none |

### Seam alternatives

- **Surface arbiter (N6).** A: keep the four-variant enum; every domain task owner must mirror
  open/close into the host. B: the host stores Objects/Inspect only, and Task/ToolDetail
  visibility is read from the domain owner through an `is_open` / `can_close` pair in the
  typed workspace input. B has one source of truth and cannot appropriate task state; it
  costs two fields per task-owning input. I recommend B.
- **Case Preview controls (N7).** A: relocate into the strip behind one shared owner hook
  (new cross-crate surface, needs approval). B: leave the controls in the `CasePanel` footer
  and give Case only Objects/Inspect in the strip (no contract change; the footer keeps an
  action bar). B is cheaper and keeps 44px targets. Which reading of "one bounded toolbar"
  applies to the footer is the user's call.
- **Workspace-round-trip drafts (N8).** A: keep each workspace's Inspect content mounted and
  inert (simple; hidden effects keep running, including Case's). B: hoist per owner (precise;
  touches up to 18 editors). Decide per domain, but decide in the ticket.

### Human decisions (none approved by this review)

1. The four already pending: overflow routing, narrow/keyboard/session-draft literals,
   Keycaps board-default relocation, commit triggers (now with the N9 table).
2. Do modified picks (Add / Range / Toggle) keep Objects open?
3. May Case generation and Export geometry controls drop from DESIGN.md's 44px to 32px?
4. Must Keymap, Keycaps and Case drafts block Update Now once they outlive their panel?
5. May a phase boundary with mixed old/new shells merge to `main`?
6. Does the no-fade rule extend to nested shared controls?

### Optional follow-ups

Standards 3–4 and the `Owner:` naming; Spec 6–8; confirm the mounted page suite loads the
built stylesheet before relying on pixel literals.

## Evidence, coverage and limits

- `change.detect` (unstaged): 4 modified tracked files, risk LOW, 0 dependents. It does not
  see untracked files; the 20 new files were inventoried by file search and read directly.
- `change.tests`: 0/3 doc symbols covered. `change.guards`: no rules configured.
  `change.contract`: allow / low. These are doc symbols; none of this is review approval.
- Logs read: `.evidence/toolbar-plan-refactor-repo.log` (repo step passed; 763 Markdown, 41
  ADRs, 64 core test files) and `...-links.log` (763 Markdown). Not re-run. This new file is
  not covered by those runs; it contains no Markdown links.
- Macro caveat: caller counts for Dioxus components come from text search, not call edges.

Read in full — candidate (24): `DESIGN.md`, `GLOSSARY.md`, `docs/adr/0035-…`, `docs/adr/README.md`,
`docs/adr/0041-…`, `map.md`, `inventory.md`, `architecture.md`, `verification.md`,
`refactoring.md`, issues 001–014.

Read — authority and standards: both frozen READMEs, `CODING_STANDARDS.md`, `PRODUCT.md`,
`docs/agents/{issue-tracker,triage-labels,domain}.md`, ADR 0002, 0008, 0010.

Read — source: `draft_owners.rs`, `workspace_composition.rs`, `cad_presentation.rs` (full);
`KeymapPanel`, `KeymapLayerControls`, `DockLayout`, `LayerEditsContext` (symbols); text
searches for `DraftOwnerRegistration {`, `pin_inspector_on_desktop(`, `Exploded`.

Unread: `template.html`, `toolbar.html` (frozen; Spec fidelity to them not assessed);
`docs/architecture.md`, `README.md`, ADR 0029 and 0034 bodies; the worktree's own `AGENTS.md`
(the copy supplied to this session came from the original checkout); every other source file.

Ownership: this file is the only file created or changed by the reviewer. No plan, ticket,
ADR, production file, baseline or frozen reference was edited.

Final status: both axes **CHANGES REQUIRED**; review gate completed with findings; plan
remains unapproved and implementation unauthorized.
