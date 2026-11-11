# 003: Layout Inspect with reset-safe editor ownership

Status: ready-for-agent
Type: build
Blocked by: [002](002-toolbar-accordion-label-fitting.md)
Parent: [map.md](../map.md)
Owner: `layout_workspace.rs` Inspector content, Layout draft/outline/relations owners

## Boundary

Mount existing Layout Inspector through 009's unpinned host: component/companion fields,
matrix/key size/transform, Board, outline lifecycle/bridges, Relationships and Findings with
Back to selection. No geometry script editor here (005), duplicate Position, dock mode or
new edit/undo authority. Shared routing hunks remain coordinator-owned.

Audit reset effects and conditional keys, not merely mounting: Outline
`use_outline_lifecycle` currently resets version-name drafts on owner/active-version change.
Implement domain-owned restoration under actual scope/entity/version identity, within the
[session lifetime contract](../architecture.md#draft-owners-not-merely-hidden-dom). Pending
edits retain captured owners; deleted versions/expired sessions never admit replay.

Classify Layout reveal sites using the [per-site policy](../architecture.md#reveal-site-migration).
Ordinary picks do not open Inspect; explicit Inspect, Findings, Relationships and lifecycle
requests do. Outline selection shows the currently active version; only explicit lifecycle
actions change it. Inspect identifies linked partner/local/omitted state from records,
never names/physical side. Keep existing resolver/settlement/undo rules.

## Acceptance criteria

- [ ] Every Layout Inspector inventory row retained, including errors, pending and empty states.
- [ ] Drafts survive close/reopen, A→B→A, Objects switching and workspace round trip; no
      visibility-induced commit, stale-target replay or reset disguised as keep-mounted.
- [ ] Record every Layout reveal caller/event/destination; other adapters own their callers.
- [ ] Opening never changes camera/bounds. Existing origin/anchor/relationship Escape owners
      keep priority; closed editors are inert through 009, not a private visibility flag.

## Proof map

Follow the [captured-owner commit paths](../refactoring.md#commit-triggers-need-a-decision-not-an-accidental-refactor)
adopted for M1 in delivery.md. Retain LayoutWorkspaceState draft/pending owners above host
visibility, and Outline's actual version-keyed text/error state above its reset effect;
keep-mounted alone does not prevent that effect. Keep existing registered component owners'
signals/guards together. 005 owns MatrixSetup/script update-safety integration.

- Close/switch survival — unsubmitted component/outline draft with error in A, no legitimate
  blur/submit trigger delivered → hide/reopen and Objects↔Inspector → identical A text,
  dirty/error and captured owner, no accepted revision/history entry; later real commit
  still settles through its existing owner · mounted · planned
- Hidden Inspector safety — unresolved, non-submitted registered Layout component/companion
  draft before any legitimate commit → close Inspect, open Objects, switch workspace, request
  Update Now → deferral names Layout/editor/remedy and nothing reloads; reopening restores
  the draft and its live registration; the owner's existing resolution clears only its blocker
  · mounted · planned
- Real blur settlement — valid dirty blur-submit field in A before pointer dismissal/focus
  return → its actual blur then close/reopen → exactly one accepted A value and inverse,
  no duplicate close commit, no B edit; refused admission preserves A's draft/error · mounted · planned
- Owner restoration — draft A, then select B before settling A → return A including version
  name and workspace round trip → A restored, no A text committed/displayed as B · mounted · planned
- Stale owner — draft/pending response captured for version A → remove A or change session,
  then deliver response/reopen → no recreated/foreign-owner edit; unrelated valid draft retained · mounted · planned
- Edit/undo — valid component/matrix/outline edit explicitly committed → settle then Undo →
  exactly the existing accepted change and inverse, no extra visibility/focus Undo · mounted · planned
- Explicit reveals — closed Inspect, ordinary canvas/Objects picks → remain closed; request
  Findings/Relationships/Inspect → correct content and return-to-selection path · mounted · planned
- Escape priority — anchor/origin pick or relationship return armed, Inspect open → Escape →
  existing pick/return handled once before panel dismissal; next Escape returns utility focus · mounted · planned

## Checks and mutations

Use [verification](../verification.md): `check.py quick`, `check.py lint`, layout browser
subset and the focused page draft/component/relations files via `run-wasm-tests.py --files`.
Draft-reset proof-gap mutation must fail restored-text/no-foreign-commit assertions. The
separate hidden-Inspector mutation drops its registration while preserving text; Update Now
must still defer in the page draft-safety suite. Cover the existing registered inspector,
companion-placement and LayoutComponentInspector paths with equivalent owner inputs; no new
commit mode or registry. Reuse
existing edit/undo/native evidence where sufficient; new host timing needs mounted evidence.
