# Combined Spec + Standards delta recheck (Fable, 2026-11-10)

The one permitted recheck after `review-fable.md`. **Evidence only.** This file approves
nothing, resolves no ticket (001 stays open), waives no criterion, and starts no further
review round. It judges **M1 (Layout first delivery) only**; M2/M3 are not approved.

- Fixed point: uncommitted working candidate in `.worktrees/toolbar-workbench-plan`, branch
  `plan/toolbar-led-workbench`, against `cafaf4302635909b59666164ed190fa2137aabc5`. `HEAD`
  equals the base, so a commit-range diff is empty; the candidate is the working tree.
- Access: native Gortex MCP, explicit worktree view on every call, `exact=true` served.
- Candidate: 27 files, all read in full. This report is outside the candidate.
- Nothing was run: no tests, builds, mutations, doc checks or commits.

## Standards

Sources: worktree `AGENTS.md`, `CODING_STANDARDS.md`, `docs/agents/{issue-tracker,triage-labels,domain}.md`.

Previous findings: S1 (status labels), S2 (009 rows unmeetable in 009), S3 (001 doc row
`planned`) are fixed. S4 (Inspect/Dock wording) is fixed in `GLOSSARY.md`; plan prose still
uses the label "Inspect" as the surface noun, which the glossary now tolerates.

Documented-standard findings, none blocking M1:

1. **Follow-up, new — 007 and 008 `Status: ready-for-agent`.** `triage-labels.md`:
   "Fully specified, ready for an agent to start". Both are M3, unauthorized, and 007 is
   blocked by `ready-for-human` 010–015. `map.md` covers it in prose ("A ready-for-agent
   label never overrides a phase gate"), but the label is the same dishonesty N10 fixed
   elsewhere. Suggest `ready-for-human`.
2. **Follow-up, new — M1 acceptance rows have no M1 home.** `issue-tracker.md`: the proof
   map and mutations "form the slice's boundary, fixed before implementation".
   `delivery.md` assigns keyboard, narrow, contrast and packaged fidelity to "M1 portion
   of 007 criteria", while 007 "remains open" and "M1 does not close this all-workspace
   ticket". The rows and the contrast mutant exist and are meetable for Layout; what is
   missing is a statement of which 007 rows run at M1 and where their receipts go.
3. **Follow-up, new — glossary _Avoid_ term.** `domain.md`: name a concept "not as a term
   it lists under _Avoid_". `GLOSSARY.md` Unresolved draft: "_Avoid_: unsaved changes,
   dirty state". `refactoring.md`, 003, 011 and `verification.md` use "dirty/error",
   "dirty flag", "dirty layer name" as outcome vocabulary. Naming the code's local `dirty`
   variable is fine; oracles should say unresolved draft.
4. **Follow-up, new — stale approval wording in tracked files.** `DESIGN.md`:
   "Implementation awaits approval"; `docs/adr/README.md` row 0041: "implementation remains
   approval-gated". M1 is now authorized after this gate.

Heuristic smells (judgment, repo rules override; none requires a new refactor):

- **Speculative Generality, follow-up, new** — 009 "Transient interface" tests "hide
  allowed/refused" on labelled content, while 005 says "Do not add a pending-close veto
  absent from the original controller" and its row hedges "a domain refusal, when
  present". If no real M1 owner can refuse a hide, the refusal branch has only a fixture
  consumer. Confirm one real refusing owner during 009, or drop that variant.
- **Divergent Change, follow-up, previous** — 009 still owns row, fitting, anchoring,
  panel state and focus in one module.
- **Shotgun Surgery, previous, mitigated** — one 13-site table, coordinator-only hunks.
- **Mysterious Name, follow-up, previous** — `Owner:` still means a branch in 001 and a
  code area elsewhere.

Counts: 0 blocks, 4 follow-up breaches, 4 smells.

**Standards verdict: PASS** (M1 scope, with the follow-ups above).

## Spec

1. **Blocks M1, new (residual of N8) — no update-safety row for registered Layout
   Inspector owners.** Constraint, `map.md` Risks: "Retained drafts must also retain live
   update-safety registration ... registered-owner lifetime ... mandatory";
   `architecture.md`: "Prove hide/workspace-switch deferral". Candidate, 003: "Keep
   existing registered component owners' signals/guards together. 005 owns
   MatrixSetup/script update-safety integration." Source: `DraftOwnerRegistration` is
   constructed in `web/src/presentation/inspector.rs:152`,
   `inspector/companion_placement.rs:356` and `inspector/layout_component_inspector.rs:1593`.
   These mount in the Inspect pop-out, a different host from 005's task surface, so they
   fail independently. Weak oracle: 003 "Close/switch survival" and its mutant ("reset
   owner A's draft") pass when the text is restored but the guard was dropped on close.
   Missed failure: Update Now reloads over a hidden unresolved Inspector draft.
2. **Follow-up, new, from `toolbar.html`/`template.html` — open-section dismissal is
   unspecified.** Frozen: "Click it again, click the canvas, or press Escape to fold it
   away"; frozen Escape order is Objects, menu, Inspector, detail panel, section, tool.
   Candidate: "Escape priority: nested picker/menu first, armed canvas pick/relationship
   return next, panel last." No passage or 002 row places an open accordion section in
   that order or says whether a canvas pick folds it.
3. **Follow-up, new — focus after an Objects pick.** Frozen `template.html` returns focus
   to the Objects utility. Candidate: "nudge ... remains available on the focused canvas
   selection after dismissal". 004 "Navigation retention" does not say where focus lands.
4. **Follow-up, new — folded-state fitting.** `template.html` fits the folded strip
   separately (captions drop before utility labels). 002 "Bounded fitting" says "open each
   section/resize"; the folded case appears only in 007's criterion.
5. **Follow-up, new — over-strict oracle.** 002 "Solid motion": "opacity 1". The frozen
   strip and `DESIGN.md` dim disabled controls statically. `verification.md` already says
   "constant opacity"; use that.
6. **Follow-up — 005 parked-owner coverage.** "Hidden draft safety" names MatrixSetup
   only; `ExistingHalfSetup` and `MirroredPairForm` also register. Name them as
   parameterized inputs or state that they share the mount path.

Previous Spec 1–4 are closed (13 sites confirmed by text search at the listed lines; Case
rail owned by 013; exact commit table; M2 classification gated). Previous 5–8 are closed by
recorded M1 defaults, the scope limit on shared menus, the neutral Objects entry, and
002's "real built Theme/toolbar stylesheet".

Fidelity to the frozen files: direction, 42px, five sections, same-row utilities,
mutual exclusion, measure-before-motion, solid translation and 280/170ms match. Panel
widths differ (frozen 280/304px, plan 400/320px); the READMEs call geometry illustrative.

Counts: 1 block, 5 follow-ups.

**Spec verdict: CHANGES REQUIRED** (one bounded proof-row addition).

## M1 readiness

**Acceptance readiness: BLOCKED** on one unmet mandatory M1 criterion:

- Registered-owner update safety for Layout Inspector editors has no boundary row or
  mutation (Spec 1).

Precise correction (new): add to 003 one row, and to `verification.md` one mutant.

- Row: unresolved registered `LayoutComponentInspector` (or companion) draft → close
  Inspect, open Objects, switch workspace, request Update Now → deferral names
  Layout/editor/remedy and nothing reloads; reopen restores it; the existing submit/reset
  clears only that blocker · mounted · planned.
- Mutant: drop that owner's guard on close while keeping its text; runner: the page suite
  with `layout_draft_safety_tests.rs`; oracle: Update Now is deferred.

Under `issue-tracker.md` Re-review, an added case needs the author's diff read and
RED/mutation evidence at build time, not another review. With that row added and the doc
checks rerun, M1 has no other unmet mandatory criterion found by this recheck.

Not M1 blockers, still gated: M2 phase approval; Keycaps board-default relocation; Case
View/Preview interface and 44px targets (015); commit-mode classification and update-safety
treatment of unregistered Keymap/Keycaps/Case forms; M3 (007/008); any mixed-shell merge
to `main`. This is not full-programme approval.

## N1–N10 dispositions

| # | Disposition |
|---|---|
| N1 | Closed. Real owner/task rows and mutants are in 005; 009 uses labelled content; no 009→005 cycle |
| N2 | Closed. Unchanged Layout and Case `objects`/`inspector` Elements named |
| N3 | Closed. One authority per workspace; 009 Commands, 006 target reveals, 005 Browse/return |
| N4 | Closed. 005 blocked by 006; 014 by 005; graph read as acyclic |
| N5 | Closed. 13-site table matches source; Case rail in inventory and 013 |
| N6 | Closed. Host stores Objects/Inspector only; see the Speculative Generality note |
| N7 | Closed by separate ticket 015, M2-gated |
| N8 | M1 part open: 005 covers MatrixSetup/scripts, 003 lacks the Inspector row (Spec 1). M2 part correctly gated |
| N9 | Closed. Exact per-path table; 011 row and two mutants; "at most" rejected |
| N10 | Closed for 003/005/010–015. Residual: 007/008 labels (Standards 1) |

## Evidence, coverage and limits

- `change.detect`: 4 modified tracked files, risk LOW, 0 dependents. It ignores untracked
  files; the 23 untracked candidate files were inventoried by file search and read directly.
- `change.impact` on `map.md`: LOW, 0 dependents; the server noted "Contract analysis is
  rebuilding", so `tests`/`guards`/`contract` were not re-queried. Doc symbols only.
- Logs `.evidence/toolbar-plan-m1-repo.log` and `...-links.log` (766 Markdown) were **not
  opened and not rerun**; their PASS is the parent's statement. This file is not covered by
  them and contains no Markdown links.
- Dioxus macro caveat: call-site counts come from text search, not call edges.

Read in full, candidate (27): `DESIGN.md`, `GLOSSARY.md`, `docs/adr/0035-…`,
`docs/adr/README.md`, `docs/adr/0041-…`, `map.md`, `inventory.md`, `architecture.md`,
`verification.md`, `refactoring.md`, `delivery.md`, `review-fable.md`, issues 001–015.

Read in full, authority: both frozen READMEs, `option-a/toolbar.html`,
`tool-panels/template.html` (read from the saved Gortex result in four slices), worktree
`AGENTS.md`, `docs/architecture.md`, `README.md`, `CODING_STANDARDS.md`, `PRODUCT.md`,
`docs/agents/{issue-tracker,triage-labels,domain}.md`, ADR 0002, 0008, 0010, 0029.

Source, text search only: `pin_inspector_on_desktop(`, `DraftOwnerRegistration {`,
`matrix_real_create_pending_exempts_only_submitted_draft`. No source bodies were read;
`review-fable.md`'s source receipts are reused unverified.

Unread: ADR 0034 (not needed for M1), `docs/agents/model-routing.md`,
`docs/agents/ai-documentation.md`, the two evidence logs, every source body.

Ownership: this file is the only file created or changed. No plan, ticket, ADR,
production file, baseline or frozen reference was edited.

Final status: Standards **PASS**, Spec **CHANGES REQUIRED**, M1 **BLOCKED** on one
proof-row addition; M2/M3 unapproved.
