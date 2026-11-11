# Refactoring seams and implementation order

Parent: [map.md](map.md). **Planning refinement, not authorization to refactor production.**
Use these recipes inside the existing tickets; this is not a new framework or a second
migration programme. The selected source-refactor kind is **move render-only presentation**,
not rename domain concepts or rewrite controllers. Native Gortex `refactor.move` currently
supports Go declarations only, so no Rust/Markdown semantic move was executed. Approved Rust
work must use guarded Gortex edits, with signature verification before any interface change.

## Verified anchors and what they imply

| Graph-resolved path / symbol ID | Source evidence | Planning consequence |
|---|---|---|
| `web/crates/ui-shared/src/panels.rs` — `ts-boardstudio2/web/crates/ui-shared/src/panels.rs::DockLayout` | Canvas remains in a stable child position; inactive content stays mounted/inert | Preserve actual child identity/order, not just equal canvas bounds |
| `web/src/presentation/workspace_composition.rs` — existing typed workspace inputs and routing functions | Real Objects/toolbar/Inspector variation already has a private seam | Reuse that seam; no universal entity/command registry or redundant adapter framework |
| `web/crates/runtime/src/draft_owners.rs` — `ts-boardstudio2/web/crates/runtime/src/draft_owners.rs::DraftOwnerRegistry.poll` | Dead guards and expired project scopes are pruned; hidden live owners remain blockers | Draft retention must preserve update-safety admission, not merely cached text |
| `web/crates/keymap/src/keymap/panel.rs` — `ts-boardstudio2/web/crates/keymap/src/keymap/panel.rs::KeymapPanel` | Editor branches unmount conditionally; selected-key/scope effect resets local presentation | Separate draft owner from branch visibility before changing its host |
| same file — `ts-boardstudio2/web/crates/keymap/src/keymap/panel.rs::KeymapLayerControls` | Dirty layer-name field submits Rename on blur | Resolve commit-trigger expectations, rather than assert every close causes zero revisions |
| `web/crates/case/src/cad_presentation.rs` — `ts-boardstudio2/web/crates/case/src/cad_presentation.rs::CasePanel` | Generation observation, readiness, callbacks and control rendering share one owner | Move control rendering, not the generation hook or export authority |

These are verified source identities, not claims of test coverage or a completed combined
review. In particular, a missing Dioxus caller edge is not proof a rendered control is unused.

## Refactor first, then change placement

| Ticket | First, behaviour-preserving preparation | Then, adoption | Must not move |
|---|---|---|---|
| 009 | Keep private composition inputs and stable canvas/content slots; host stores Objects/Inspector only, reads transient visibility from its typed owner | Anchor/clamp panels and route show/hide to the workspace's sole current authority | Document edits, task visibility copies or admission into visual primitives |
| 002 | Use one fitting/clip implementation in the private host, with explicit action/caption priority inputs | Feed existing Layout actions through it; PCB later reuses applicable mounts | Transform/Align/Snap resolvers or guard arithmetic |
| 003 / 011 / 012 / 014 | Hoist or retain the existing typed editor owner above a visibility/selection branch; preserve its live draft registration | Change the panel host, then prove close/switch/return behaviour | Pending edit identity or fresh admission tokens into a restoration cache |
| 005 | Separate Add entry/chooser presentation from the existing setup/placement controllers | Route to current real preview/task owners and preserve cancellation/pending rules | Setup logic or legacy-script execution/safety into the shell |
| 013 / 015 | 013 handles tree/Inspector/View; 015 first establishes one existing generation/readiness/command owner behind approved render-only consumers | Move Preview control presentation only in 015 after its contract approval | A second automatic observer, command registration or export authority |

Run the existing affected assertions after the preparation and before visible relocation.
Do not combine a mechanical extraction with a new settlement policy. Inventory changed
fields/types by compiler errors and graph references during approved implementation. A
signature/visibility change needs Gortex `change.verify` and the required human approval;
this document does not grant that approval. No Rust signature is proposed as already valid.

## One top-level surface authority

Keep one authority per state, not one copy of every state in the host. The private host
stores Objects/Inspector only. Existing typed workspace owners remain the authority for
whether a tool detail or task is visible, and expose a guarded hide/park request at the
composition seam. Do not mirror their state into the rejected four-category host enum.
An open accordion section and nested picker are not peer overlays.

On a utility request, first ask the visible owner to hide; a refusal changes neither
surface. On a domain open request, close compact panels before that owner reveals its own
content. A domain's own Close/Cancel remains authoritative. Hiding parks live drafts;
intentional Cancel retains its existing discard/settlement meaning. Accepted or vetoed
workspace/board navigation follows existing domain admission, with no shell cancellation
or late cross-owner callback. 009 tests this private interface with labelled synthetic
content; 005 proves the real Layout task wiring. This is not a new generic workflow bus.

## Draft retention includes update safety

`DraftOwnerRegistry` is the existing runtime inventory, not a draft-value store. Owners in
matrix/existing-half setup, PCB and Parts editors and legacy scripts already register there.
Retain their RAII guard and synchronous unresolved-state predicate for the actual retained
owner lifetime. Do not drop registration when a panel hides, and do not solve that loss by
copying editor values into a new global cache or adding a second safety registry.

For an unmounted editor with parked unresolved values, the responsible domain owner must
still keep the applicable admission key live; whether to retain the owner or use an existing
typed owner store is decided per domain. Pending results remain captured to their original
identity. Session replacement invalidates the old owner and its registration normally.

Required new proof: unresolved registered editor → hide/switch workspace → Update Now still
defers with that editor's existing remedy and no reload; return/apply/reset → gate clears.
Also prove session replacement retires only the old registration. Use the existing runtime
registry tests for pruning rules and a page-mounted test for host wiring; no full cross product.

## Commit triggers need a decision, not an accidental refactor

Preserve existing commit triggers; do not force every field into explicit-submit behaviour.
`KeymapLayerControls` submits a dirty name on blur, but its local dirty flag and shared name
signal are not a per-layer restoration store. 011 must retain text + dirty/error + captured
layer identity together; keeping text alone can lose the rename or target the next layer.

| Dismissal path / critical timing | Required independent result |
|---|---|
| Valid dirty field receives its real blur before toggle/outside-pointer/Objects activation | Exactly one accepted rename of captured live A, saved name equals typed text; B unchanged |
| Escape returns focus and consequently emits the normal blur | Same exactly-one A commit, no second synthetic close commit |
| Close/hide occurs without a blur event | Zero close-induced revisions; reopen A restores exact text, dirty flag and error; its later legitimate blur commits exactly once |
| Layer pick changes to B before a delayed A event | B's saved name/text unchanged; A's captured event is admitted only by existing owner rules, otherwise A's full draft/error remains available |
| Workspace switch before blur | A's draft remains A's across return; if a real blur already settled it, saved A has exactly that one change, never a duplicated rename |
| A deleted/session replaced before delayed event | No replacement/foreign commit and no refreshed admission from restoration |

These paths are parameterized only where event timing is equivalent. For an admission
refusal, require retained A draft/error, not merely absence of a B write. The previous
“at most” oracle is rejected: zero commits plus discarded text cannot pass. Layout 003's
no-revision stories explicitly use unsubmitted explicit-submit drafts without a blur trigger;
normal blur fields get the captured-owner story instead. This is the conservative preservation
recommendation adopted for M1; 011 implements its own layer-name proof in M2.

## Dependency and review discipline

Keep the existing graph except that 005 also depends on 006, 014 depends on 005, and 015
is the separately gated Case Preview slice. [delivery.md](delivery.md) makes the complete Layout vertical slice
the first authorized milestone; later adapters keep their human/phase gates. Preparation lives within its owning
slice, so it does not serialize unrelated adapters behind a speculative refactor ticket.
009 alone owns generic visibility/fitting/focus mechanics; 002 owns Layout action wiring
and motion/fitting acceptance. The coordinator serializes shared-host/routing hunks. Domain
adapters own their controller/render-only splits and identity checks.

The Fable review completed with native access and findings in review-fable.md; the old
child-access failure is historical, not the current blocker. A single combined delta recheck
covers these corrections and its previously unread frozen/standards sources. Its findings
and the phase authorization in delivery.md remain distinct; no later phase is authorized.
