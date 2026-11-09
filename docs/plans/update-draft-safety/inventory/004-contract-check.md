# 004 contract check — adversarial review of the draft-safety interface contract

Authoritative adversarial review for Wayfinder ticket 004
(`docs/plans/update-draft-safety/issues/004-choose-draft-safety-interface.md`).
No product code changed; the 004 ticket and `map.md` were not edited.
Source via native Gortex MCP (exact worktree overlay views) plus bounded
line-span reads. Symbol IDs omit the `ts-boardstudio2/` prefix.

## Contract under test (user-approved in grilling, not yet recorded)

1. Small scoped registry of live draft owners, polled synchronously by
   `update_safety_inputs` at both Update Now checkpoints. `PendingEditSignals`
   gains bound-field dirtiness reporting; narrow adapters register unbound
   composite forms. `aria-invalid` DOM query retained as additive-only fallback.
   No editor behavior changes.
2. Unresolved = live draft ≠ accepted baseline (owner-canonical comparison, i.e.
   the value that would be submitted) AND draft ≠ that owner's in-flight
   submitted value. Landing updates baseline; failure/refusal never clears;
   newer typing re-raises; revert-to-baseline clears.
3. Submission-in-progress marked from the Apply/Save click, covering async
   admission (e.g. module attach connector load).
4. Registration keyed (project scope, workspace, owner identity, field/composite
   id) + binding epoch; retires on genuine unmount/owner retirement;
   hidden/portal-retained owners still block; project/scope switch retires
   with owner.
5. Only owner's own Cancel/Reset clears outside submission; gate only defers,
   never force-submits/discards.
6. Required coverage: the 14 Class A owners incl. creation forms (baseline =
   initial defaults). Class B (blur-commit) registration NOT required
   initially; registry must accept them later without redesign.
7. Each registration carries a user-facing area label; initial deferral copy
   names area + Apply/Save-or-cancel remedy; installed-phase copy distinct;
   cancel-only; focus preserved; automatic resume retained.

## Verdict

**Not falsified in its core, but NOT recordable as stated.** The registry +
owner-canonical comparison + per-key submitted memory + epoch lifetime is the
right shape and survives every owner I could throw at it. However, five
**blocks-recording** findings show clauses 2, 4, 6, 7 are false or
underspecified exactly as worded: creation-like keys with no accepted value
false-block on mount (L10-new), parameter-only fields have no accepted
baseline at all (A4 origin), single per-owner submitted memory cannot express
multi-admission owners (A4 three admissions, A2 one click → two collections),
and no domicile/pruning rule names the synchronous Runtime ← UI path the
whole contract depends on. Each has a minimal amendment that fixes it without
redesign (listed per finding and summarized in §Amendments). Record the
contract only with all five applied.

Counts: **5 blocks-recording · 6 should-amend · 7 notes**. What survives is
listed last so the record shows both sides.

---

## BLOCKS-RECORDING (must be fixed in the contract text before recording)

### B1. Clause 2's "accepted baseline" is undefined for keys with no accepted value — L10-new false-blocks on mount

- **Claim attacked:** "Unresolved = live draft ≠ accepted baseline", with
  creation-form baseline = initial defaults (clause 6).
- **Counterexample (source):** `GeometryScriptsEditor`
  (`web/crates/ui-shared/src/geometry_scripts.rs`,
  ID `…/geometry_scripts.rs::GeometryScriptsEditor`) holds a JSON-serialized
  draft, `draft = use_signal(|| encode_script_draft("", "", true))` (~lines
  100–140; `encode_script_draft` serializes `(name, source, enabled)`), bound
  with `pending.bind_field(ScriptActionKey::Apply(String::new()), draft,
  failure)`. Its settle-time accepted projector returns
  `accepted_scripts.iter().find(...).map(...).unwrap_or_default()` — i.e.
  **`""` when no script with that id exists** (the New-script case). Mount-time
  draft `encode("","",true)` (`["","","",true]`-shaped JSON) ≠ `""`, so a
  freshly opened, untouched New-script form reads "unresolved" under clause 2
  as literally worded (compare live draft against the accepted projection).
  The same structural trap applies to every creation form (S2 new matrix,
  S2b mirrored-pair, L20 existing-half): clause 6 says "baseline = initial
  defaults" but clause 2's formula only names the accepted baseline, and the
  only accepted-projector in evidence (L10's settle closure) returns
  empty/`""` for not-yet-existing objects — not the defaults.
- **Why it matters:** recording clause 2 verbatim mandates either a permanent
  false deferral on untouched creation forms or an implementation that quietly
  deviates from the recorded contract.
- **Minimal amendment:** baseline is per-key and owner-supplied at
  registration: either a live accepted projector returning `Option<D>`
  (`None` = no accepted value exists) plus a mount-time initial-default
  snapshot, with `None` falling back to the snapshot. I.e. unresolved =
  draft ≠ (accepted projector ?? mount snapshot). L10-new then compares
  `encode("","",true)` vs snapshot `encode("","",true)` → clear. This also
  reconciles clauses 2 and 6 into one rule.

### B2. Parameter-only fields (A4 origin X/Y) have no accepted baseline and no single remedy — per-owner comparison is inexpressible

- **Claim attacked:** clause 2 (single draft/baseline per owner) + clause 7
  (single Apply/Save-or-cancel remedy).
- **Counterexample (source):** A4 assembly draft
  (`web/crates/parts/src/parts/assembly_editor.rs`,
  ID `…/assembly_editor.rs::SavedAssembliesEditor`, lines 331–774):
  `origin_x`/`origin_y` are per-mount `use_signal "0"` strings consumed only
  by the Place/Matrix admissions (`begin_place_assembly_on_board`, line 777;
  `begin_apply_assembly_to_matrix`, line 933), never by Save. They are
  parameters of a one-shot action, not edits of any accepted value — there is
  no accepted baseline for them, and mount-default (`"0"`) is the only
  baseline-like thing. Failure mode under the contract as stated: user types
  origin `5`, clicks **Save assembly** (not Place). Save submits the unchanged
  assembly tree, lands, baseline updates — origin still reads `5` vs default
  `"0"` → still "unresolved", but the recorded remedy ("Save or cancel") is a
  lie: no number of Saves clears origin dirtiness; only Place / Apply-to-matrix
  / Close does.
- **Minimal amendment:** register parameter fields under their consuming
  admission key(s) with mount-default baselines, and make the area label +
  remedy per admission key (B5 below is the same fix from the copy side).
  A4 then registers (at least) Save-keyed tree state and Place/Matrix-keyed
  origin state instead of one owner blob. Clause 2 must say "per (owner,
  admission key)".

### B3. "That owner's in-flight submitted value" (singular) cannot express multi-admission / multi-collection owners

- **Claim attacked:** clause 2's singular per-owner submitted value.
- **Counterexamples (source):**
  - A4 has **three** admissions over one draft tree — Save, Place-on-board,
    Apply-to-matrix (`assembly_editor.rs:777–930`, `begin_one_shot`
    `AssemblyKey::Save/Place/Matrix`; `pending: Vec<AssemblyAction>` intents).
    Two can be in flight at once (Save + Place), each with a different
    submitted value (tree vs tree+origin). One slot cannot hold both; the
    older settlement must not clear the newer draft per key.
  - A2 module profile (`module_profile_editor.rs:295–813`,
    ID `…/module_profile_editor.rs::ModuleProfileEditor`): one **Save project
    module profile** click calls **two** `begin_value`s in two collections —
    `profile_edits.begin_value(ModuleProfileKey::Save, …)` (line 617) and
    `rotary_edits.begin_value(ModuleRotaryKey::Save, …)` (line 608) — plus
    staged volume/shape state. One Save → two keys in two helpers; "the"
    in-flight value is two values.
  - A1 generator settings (`generator_settings.rs:785–1363`): Apply
    (one-shot `GeneratorKey::Apply`) + model Upload (one-shot
    `GeneratorKey::Upload`) over one local `edits: BTreeMap<String, Value>`
    map (lines 815–820).
- **Minimal amendment:** submitted memory and the clause-2 comparison are
  keyed per (owner, admission key, collection) — i.e. reuse the existing
  `Submission { key, text, field_epoch, one_shot_epoch }` shape already in
  `pending_edit_helpers.rs` (lines ~230–300: `begin_value`/`begin_one_shot`
  replace per-key records; `settle` drains per key), extended by adapters to
  unbound keys. The contract text must say "per admission key", never "per
  owner".

### B4. No domicile or pruning rule names the synchronous Runtime ← UI path — the feasibility leg is missing

- **Claim attacked:** clause 1 ("polled synchronously by
  `update_safety_inputs`") + clause 4 (keying/retirement).
- **Evidence (source):**
  - `Runtime::update_safety_inputs`
    (`web/crates/runtime/src/runtime.rs:930–946`,
    ID `…/runtime.rs::Runtime.update_safety_inputs`) is `&self`, reads
    `self.session`, `self.pending_document`, and calls the free function
    `visible_invalid_draft()` (lines 281–293, DOM query). It has **no input
    for component-local signals** and no registry field exists on `Runtime`
    (struct definition `runtime.rs:343–~430`: session/core/store/persistence/
    save/update fields — no draft registry).
  - `PendingEditSignals` (`pending_edit_helpers.rs:1–64` module doc, ID
    `…/pending_edit_helpers.rs::PendingEditSignals`) keeps `fields:
    RefCell<Vec<Binding>>` private with **no key enumeration and no
    draft≠accepted query** (confirmed in the seams table,
    `001-setup-dialogs-seams.md` §Shared seams); `OwnedEdits`
    (`owned_edits.rs:1–109`, ID `…/owned_edits.rs::OwnedEdits`) is scoped to
    one controller/owner (`follow_owner(scope, generation)`), with no global
    inventory. So "PendingEditSignals gains bound-field dirtiness reporting"
    is necessary but not sufficient: N heterogeneous helpers
    (`PendingEditSignals<ScriptActionKey>`, `<ModuleProfileKey,
    ModuleProfileDraft>`, `<ModuleEditKey, MountedModule>`,
    `<(), (String,String)>`, …) cannot be polled without a type-erased
    registration (e.g. `Box<dyn Fn() -> bool>` + label + key) held somewhere
    `update_safety_inputs` can reach synchronously.
  - Dioxus signals *can* be read outside render (`peek()`), and components
    already hold `Rc<Runtime>` via context (e.g. `geometry_scripts.rs`:
    `use_context::<Rc<Runtime>>()`), so both candidate domiciles work — but
    the contract names neither, and the lifetimes differ critically: a
    `thread_local!` registry leaks across project switches unless pruned by
    key; a `Runtime`-field registry (`RefCell<Vec<Entry>>`) lives as long as
    the `Rc<Runtime>` (which outlives project open/close — `Session::new()`
    per open, same `Runtime`), so entries keyed to a closed project must be
    pruned on poll (`entry.project_scope != current → ignore-and-drop`).
- **Minimal amendment:** name the domicile — recommended: `Runtime`-owned
  `RefCell<Vec<RegisteredDraft>>` with `register/unregister` called from
  components holding `Rc<Runtime>` (mount / `use_drop`), each entry
  `{key: (project_scope, workspace, owner_id, admission_key), epoch,
  is_unresolved: Rc<dyn Fn() -> bool>, label, remedy}`; `update_safety_inputs`
  drains/prunes (stale project scope, unmounted-without-unregister) and ORs
  `is_unresolved()` into a new `UpdateBlocker::UnresolvedDraft`-style input.
  State the pruning rule in the contract: entries whose project scope ≠
  current accepted document are dropped at poll time (this is also what makes
  "project/scope switch retires with owner" in clause 4 mechanically true).

### B5. Remedy copy "Apply/Save-or-cancel" is false for multi-admission owners — copy must be per admission key

- **Claim attacked:** clause 7's single remedy formula.
- **Counterexample:** B2's A4 origin case (Save never clears origin dirtiness),
  plus A1 (Upload dirtiness is not cleared by Apply either — file state vs
  parameter map). A single "where to Apply/Save" pointer misdirects the user
  for exactly the owners with the most complex state.
- **Minimal amendment:** the per-registration remedy names the consuming
  action(s) for that key: e.g. A4-origin → "Place on selected board, Apply to
  a matrix, or close the editor (Close discards)"; A4-tree → "Save assembly
  or close"; A6 → "Attach module or close". Clause 7 must say the label +
  remedy pair is per admission key, and the initial vs installed copy split
  applies to the template, not to one global string.

## SHOULD-AMEND (do not block recording, but the contract is weaker without them)

### S1. Canonical-equality rule missing: text-vs-value formatting can false-block forever

- Owner-canonical comparison ("the value that would be submitted") is the
  right idea, but several owners submit *parsed/normalized* values while the
  live draft is *text*: L8 legacy position tuple `(String, String)` committed
  via `commit_numeric` (`web/src/presentation/inspector.rs:293`,
  ID `…/inspector.rs::commit_numeric` — parses, returns early on invalid
  input); A4 `origin_x/y` parsed to `f64` at Place time (invalid silently
  ignored at input, `MemberPatch::apply` `assembly_editor.rs:1769–1856`);
  L1 constraint numeric texts parsed in `save_constraint`
  (`layout_component_inspector.rs:1366–1478`); A3 contour vertex inputs parse
  `f64` keeping the old value on failure. `"1.0"` vs accepted `"1"`, or L8's
  accepted-`f64`-to-string projection format vs the user's keystrokes, can
  read dirty-forever unless each adapter defines canonical equality (compare
  post-parse values, or normalize text both sides). L10's JSON tuple encoding
  is deterministic (good); A1's `BTreeMap` is deterministic (good); A5's typed
  `MountedModule` compares structurally (good — cite
  `pcb_module_inspector.rs:937–947` `edit_module_draft` value-equality dirty
  marker). **Amendment:** each registration states its equality (canonical
  parse-then-compare vs exact), and reviewers check the false-dirty case per
  owner at implementation time.

### S2. Class B exclusion vs "cover all project drafts" must be recorded as explicit risk acceptance with its mechanism

- Clause 6 defers all 22 Class B owners (blur/focusout/Enter-committing:
  census fragments `001-layout-metadata.md` L2/L3/L4/L9/L11–L14/L16/L17,
  `001-pcb-parts.md` B1–B4, `001-keymap-keycaps-case.md` K1/K3/K4/K5/C1/S1/
  S3/S4). The exclusion is defensible — activating Update Now moves focus,
  which fires the panels' standard blur-commit *before* the gate reads, and
  the resulting `Session::has_pending_work` then blocks via the existing
  `PendingWork` leg — but it is an ordering argument over async admission,
  and the census itself flags the races: Enter→blur double paths (L16/L17),
  combobox-portal focusout topology (K3/KeycodeField container `onfocusout`,
  `binding_editor.rs:715–785`), programmatic unmount-without-blur dropping
  dirty B drafts (K1/S1 unknowns), and native `type=number` sanitization (U3
  browser-dependent). **Amendment:** record the B-deferral as an explicit,
  revocable risk acceptance naming the blur-dispatch mechanism and the three
  listed races, with re-inclusion (registry accepts B later — already
  required) as the fallback. Do not let "cover all project drafts" read as
  already satisfied.

### S3. "Small scoped registry" understates the adapter majority: most Class A owners have zero bound fields

- Of the 14 Class A owners, the unbound/no-bound-field set is the majority:
  L1 (constraint composite binds one-shots only —
  `bind_one_shot(Constraint/RemoveConstraint/…)` — all ~10 draft signals
  unbound), A3 (one-shot-only binding; draft never touches the helper until
  Save — `mechanical_profile_ui.rs` shell owns `ProfileKey::Save`), A4 (tree
  in parent `editing: Option<AssemblyDraft>` signal + origin strings; helper
  holds one-shots only), S2/S2b/L20 creation forms (no helper at all —
  `MatrixSetup` `matrix_setup.rs:105–220`, `MirroredPairForm`
  `mirrored_pair.rs:126–286`, `ExistingHalfSetup` `existing_half.rs:568–760`;
  nothing enters any pending collection before submit). Bound-field dirtiness
  reporting serves only L8 (bound `(String,String)` tuple), L10 (bound JSON
  string), A2 (typed `ModuleProfileDraft`/`ModuleRotaryDraft`), A5 (typed
  `MountedModule`). The contract already includes adapters, so this is not a
  falsification — but the text's emphasis ("PendingEditSignals gains …
  reporting; narrow adapters register…") inverts the workload. **Amendment:**
  name the adapter path as the primary Class A vehicle with per-owner
  closures, and bound-field reporting as the convenience for the four already-
  bound owners.

### S4. Baselines must be re-projected live every poll — bind-time snapshots go stale (L1 clobber, L10 switch, undo)

- 007 established that accepted updates **clobber** open L1 drafts with no
  dirty guard (the `accepted_constraint` effect rewrites every constraint
  draft, `layout_component_inspector.rs:~978–1024`) and that L10's script
  picker replaces locals with no guard (`on_select_script`). Undo likewise
  moves the accepted document under an open draft. A registry that snapshots
  the baseline at bind/registration time will false-block (or false-clear)
  after any of these. The existing `settle(owner_is_live, |key|
  accepted(key))` pattern already projects accepted values live per settle;
  the registry's `is_unresolved` closures must do the same per poll
  (capture `Rc<Runtime>` + key, read `runtime.model().accepted` inside the
  closure). **Amendment:** one sentence — "baselines are projected live at
  each poll, never snapshotted, except creation-form initial defaults which
  are snapshotted at mount."

### S5. Stale-entry discipline: fail-closed is safe but can wedge updates; require RAII handles

- If a component unmounts without unregistering (missed `use_drop`, panic
  path), its entry's signals are dropped; a `peek()` inside the closure would
  panic or (if guarded) read stale → permanent fail-closed deferral with no
  on-screen draft to resolve — updates bricked until reload, and reload is
  what's gated (Cancel remains, but the user can never satisfy the gate).
  Mitigation is standard: registration returns an RAII guard whose `Drop`
  unregisters, owned by the component (mirroring the existing
  `use_drop(move || { helpers.unbind_field(...); })` discipline documented in
  `pending_edit_helpers.rs` module doc). **Amendment:** require guard-owned
  registration + `use_drop` release, and specify poll-time behavior for dead
  entries (drop-and-ignore with a one-time console diagnostic, not a
  blocker) — fail-open on *demonstrably dead* entries, fail-closed on live
  ones.

### S6. Clause 3's "marked from the click" needs the transfer spelled out (A6 async gap, A4 preparing flags)

- A6 module attach (`module_attachment.rs:43–402`,
  ID `…/module_attachment.rs::ModuleAttachment`) calls
  `begin_one_shot(AttachKey::Attach, …)` only *after* async connector load +
  re-validation; A4's Place/Matrix admissions carry explicit `preparing:
  Signal<bool>` flags (`begin_place_assembly_on_board`, line 777 signature).
  During click→`begin` the draft leg (draft ≠ baseline) still blocks, so
  there is no hole — but only because the draft is untouched. The mark's real
  job is the handoff: click sets a per-key `preparing` flag; `begin`
  transfers it into submitted-memory + `is_pending`; async failure clears
  `preparing` (draft leg continues to hold — "failure never clears" falls
  out naturally, no special case needed). **Amendment:** record the
  click→preparing→begin→pending/failed transfer so implementers don't try to
  infer in-flight state from `is_pending` alone (which is false throughout
  the async gap).

## NOTES (correctness confirmations, caveats, copy details)

### N1. State-rule matrix verified against source — the core temporal logic survives

- Submit-then-type-then-settle: protected by the existing
  `draft_may_restore(submitted, current)` rule
  (`pending_edit_helpers.rs`, "An unknown submitted draft never qualifies";
  `settle` → `apply_to_field` only writes when the live draft still equals
  the submitted value), which the registry design inherits per key.
- Revert-to-baseline while in flight: draft == baseline clears the draft leg,
  but `Session::has_pending_work` (wired into `update_safety_inputs`,
  `runtime.rs:930–946`) continues to block via `PendingWork` until the
  in-flight edit lands — then accepted == the landed value vs the reverted
  draft re-raises correctly (genuine divergence).
- Failed admission after async prepare (A6 connector failure, A4 origin
  non-finite → `feedback` text, L1 parse failure → shared `error` signal):
  no `begin` call occurs, draft still ≠ baseline → still blocked. "Failure
  never clears" holds without extra machinery.
- Landing updates baseline: automatic if baselines are live-projected (S4);
  creation-form mount-default snapshots retire with the form on unmount
  (S2/S2b/L20 unmount drops locals — `001-setup-dialogs-seams.md` S2 note).

### N2. Lifetime rules check out against the hardest owners

- A5 mounted-module (`pcb_module_inspector.rs:88–933`): `Collapsible`
  `keep_mounted` hidden panels keep signals alive → closures keep reporting →
  hidden still blocks, as clause 4 requires. Genuine unmount drops signals;
  `use_drop` unbinds all keys — the registry guard (S5) rides the same hook.
- L10 section Escape → `on_back` drops the draft (007 gap): owner's own
  cancel clears — consistent with clause 5.
- L1 scope/owner: `follow_owner(scope, generation)` retires observations while
  Session edits continue (`owned_edits.rs`); registry epoch in the key gives
  the same semantics per entry. Remount-with-same-key gets a new epoch; old
  submitted memory is epoch-tied so the old outcome can't clear the new draft
  (mirrors the tested `a_remounted_field_with_the_same_key_never_receives_
  the_old_outcome` guarantee in `pending_edit_helpers.rs` tests).
- Portal retention (K3 combobox popup): registry reads signals, not focus or
  DOM — portal topology is irrelevant to the draft leg. The `aria-invalid`
  fallback keeps its existing DOM-topology limits (additive-only, so no new
  risk).

### N3. Invalid text with no submittable form is handled — no special case needed

- L8 invalid parse returns early (no commit, no `aria-invalid` —
  `001-layout-metadata.md` L8 note); K5 `NumberDraft` unparseable path (U,
  browser-dependent); A3 vertex invalid text silently keeps the old value;
  A4 member vectors silently ignore unparseable input. Under the amended
  clause 2, invalid text is simply draft ≠ baseline with no in-flight value →
  unresolved → defer. The fallback query stays irrelevant for these (none
  sets `aria-invalid`), which is exactly why the registry is needed. Confirm
  the contract keeps the fallback strictly additive (a DOM/query failure
  already collapses to `false` in `visible_invalid_draft`).

### N4. Area labels are feasible — but the label table is per admission key, not per owner

- Every Class A owner has a static, truthful area name available at
  registration (no user-typed content needed): "Layout constraint",
  "Geometry script", "Legacy position", "Companion placement", "Generator
  settings", "Module profile", "Fit profile", "Assembly", "Module placement",
  "Module attach", "New net name", "New matrix", "Mirrored-pair matrix",
  "Existing half". Multi-admission A4 needs three remedy strings (B5), but
  the *area* label can stay "Assembly" for all three. No i18n infra is
  assumed; static English strings match the existing warning copy
  (`notifications.rs::blocker_message`, `UpdateWarningDialog`).

### N5. Previews are correctly outside the contract (L8, A1, S6)

- L8's live Core preview and A1's `GeneratorDraftStore` preview fire *without*
  submission and never enter the accepted document or Undo (per GLOSSARY
  "Edit preview": transient, no commit phase). The contract compares drafts
  to baselines, not previews to documents, so previews neither block nor
  clear. L8's open question (how long a preview blocks Session post-settle)
  affects the `PendingWork` leg, not the draft leg — out of scope here.

### N6. What I did not verify (caveats)

- No browser run: K3 portal-focusout topology, U3 native-`type=number`
  sanitization, and Enter→blur ordering at the real Update Now entry points
  remain with tickets 002/007 as browser-dependent unknowns. None affects
  the registry design (signal reads don't depend on focus topology), except
  S2's B-exclusion rationale which cites blur dispatch.
- Line spans above are worktree-read citations; dirty-overlay reindexing may
  shift exact lines — symbol IDs are the stable reference.
- 007's two promoted owners (L20, S2b) were re-verified via the census text,
  not re-read in source here; their shapes (local signals + form `onsubmit`
  only) match S2 exactly per the census.
- `Class B registration must be accepted later without redesign` (clause 6,
  second half): the type-erased `Fn() -> bool` entry design satisfies this —
  a B owner registers the same entry shape with a blur-aware projector. No
  redesign needed, provided B1–B5's amendments (per-key baselines, domicile)
  are in place first.

### N7. Confirmation: the discard/force-submit prohibitions hold end to end

- `update_safety` (`update_safety.rs:139–174`) returns `Err(blockers)` with
  cancel-only dialogs (`UpdateDialog::Deferred` / `OtherTabs`); both
  checkpoints (`begin_update`, `runtime.rs:965`; `reload_when_safe`,
  `runtime.rs:1024`) consume the same inputs; installed-phase text already
  reports waiting rather than discarding. Nothing in the contract adds a
  submit-or-discard path. Clause 5 survives unchanged.

---

## Minimal amendments to apply before recording (summary)

1. **Baseline rule (fixes B1, B2):** "Each admission key registers either a
   live accepted projector returning `Option<D>` or, when no accepted value
   can exist (creation forms, action parameters like A4 origin/A6 connector
   choice), a mount-time initial-default snapshot; `None` falls back to the
   snapshot. Unresolved = draft ≠ (projected ?? snapshot) AND draft ≠ that
   key's in-flight submitted value."
2. **Keying rule (fixes B3):** replace "that owner's in-flight submitted
   value" with "that (owner, admission key)'s in-flight submitted value";
   submitted memory is per admission key (the existing per-key `Submission`
   shape, extended by adapters).
3. **Domicile rule (fixes B4):** "Registry lives in `Runtime`
   (`RefCell<Vec<…>>`), `register`/`unregister` via the components'
   `Rc<Runtime>`, polled inside `update_safety_inputs`; poll prunes entries
   whose project scope ≠ current accepted document and drops dead entries
   per S5."
4. **Liveness rule (fixes S4, supports landing semantics):** "Accepted
   baselines are projected live at each poll; only creation/parameter
   defaults are snapshotted at mount."
5. **Copy rule (fixes B5):** "Area label + remedy are per admission key; the
   initial/installed template split applies per key."

With 1–5 applied, every counterexample in this note resolves without
redesign: L10-new compares against its mount snapshot; A4 origin registers
under Place/Matrix keys with default baselines and Place-specific remedy;
A2/A1 multi-key submissions get per-key submitted memory; the Runtime poll
path is named and prunable; baselines track clobbers/undo/switches live.
Class B deferral (S2) should additionally be recorded as explicit risk
acceptance, but it does not block the interface.
