# Handoff to Opus: matrix transforms and visible relationships

> Historical planning brief. The decisions below have been settled in ticket 01
> and the current spec. For implementation, use the
> [updated matrix-to-mechanical handoff](../typed-core-edits/handoff.md): read the
> resolved ticket 01 Answer, implement matrix Core 02/03, then mechanical 02.

## Task and deliverable

Plan a coherent fix for BoardStudio's matrix editing semantics and Relations
inspector. This is a planning handoff: investigate, resolve the decisions below,
and produce an implementation-ready `spec.md`, `map.md`, and dependency-ordered
tickets under `docs/plans/matrix-layout-consistency/`. Follow the
[tracker conventions](../../agents/issue-tracker.md). Keep proposed behavior and
source-confirmed behavior clearly separated. Implementation starts after the
user reviews the plan.

Repository: `/home/chris/01_Projects/ts-boardstudio2`. At handoff, local and remote
`dev` point to `f65ed01a6`. Recheck before planning. The root has unrelated
uncommitted edits to `AGENTS.md`, `DESIGN.md`, `PRODUCT.md`, `docs/README.md`, and
an untracked `docs/design/project-context.md`; preserve them. `.scratch/` stays
ignored. The user's preview is `http://127.0.0.1:4173/boardstudio/`.

Read [AGENTS.md](../../../AGENTS.md), [CONTEXT.md](../../../CONTEXT.md),
[architecture](../../architecture.md), [PRODUCT.md](../../../PRODUCT.md), and
the relevant Layout sections of [DESIGN.md](../../../DESIGN.md). Consult
[ADR-0005](../../adr/0005-resolve-queued-edits-at-execution.md) and its amendments
for edit lifecycles, and
[ADR-0006](../../adr/0006-replace-document-for-import-and-recovery.md) for typed
Core edit ownership. Use the installed codebase-design, GitNexus
exploration/impact, and Rust skills where relevant.

## User's problem

The user creates a blank matrix, for example 5 rows by 7 columns, and expects
shared spacing, column relationships, and key membership to exist immediately.
They expect normal edits to remain parametric, with individual adjustments
available when needed.

Two observations prompted this work:

- Inspector **Stagger** changes the relationship between columns, but toolbar
  **Transform → Stagger** changes a column offset instead. In their screenshot,
  Stagger stayed `0` while Offset Y became `−9.525`.
- **Relations** presents little useful information for a newly created matrix,
  column, or key, making its existing structural dependencies seem absent.

The user asked for a plan, after discussing the proposed direction below. These
recommendations are not approval for a new public interface or document format.

## Source-confirmed behavior

1. Matrix structure already determines placement. In
   `core/src/matrix.rs::location`, pitch and row/column/cell offsets contribute
   to local position. Y also includes the sum of `column_staggers` through the
   selected column. Splay, mirror, rotation, and matrix origin then participate
   in the resolved transform. This is parametric structure, even without an
   explicit constraint record for each key.
2. `web/crates/layout/src/matrix_transform_operation.rs::build_operation`
   handles inspector `ColumnStagger` by setting one entry in
   `Matrix.column_staggers`. `ColumnOffsetX/Y` update the independent
   `Matrix.column_offsets` entry.
3. In `web/crates/layout/src/objects/layout_transform_toolbar.rs`,
   `stagger_by` updates `row_offsets` or `column_offsets`. Both
   `transform_nudge_resolver` and `transform_drag_resolver` use it. Preview
   sampling in `sample_transform` and extraction in `transform_drag_result`
   also operate on offsets. Fixing only final commit would leave preview and
   keyboard behavior inconsistent.
4. `web/src/presentation/layout_workspace.rs::inspector` shows a matrix
   relationship summary, with fallback “No saved placement relationship on
   this selection.” It also shows a generic pitch/stagger/splay note and, where
   available, navigation to an explicit placement relationship. It does not
   fully explain the selected matrix's inherited dependencies.
5. This investigation was read-only. No new regression or browser reproduction
   was run for the stagger mismatch; source confirms the different writes.

GitNexus is indexed as `boardstudio`. Its source snapshot was refreshed before
the last commit, but metadata currently reports one commit behind. Verify
freshness. Use `node .gitnexus/run.cjs` for CLI fallback; `query` for flows,
`context` for symbols, `impact` before behavior changes, and `detect-changes`
before commits. UNKNOWN or missing graph edges require source confirmation.

## Proposed behavior to evaluate

| Concept | Intended meaning |
| --- | --- |
| Column stagger | Relative Y adjustment in matrix coordinates; the selected column and following columns inherit it |
| Column offset | Independent adjustment of only that column |
| Key local offset | Individual key adjustment within the matrix; its attached assembly follows |
| Structural relationship | Inherited matrix/assembly placement, projected from the existing model |
| Explicit constraint | A separately authored relationship, displayed with its real ownership and precedence |

For an unsplayed example, stagger values `[0, 2, -1]` contribute Y offsets
`[0, 2, 1]`. Increasing the second value by 3 moves the second and third columns
by 3. The first column's reference is the matrix base, not an imaginary preceding
column. Make this distinction explicit in the plan.

Recommended direction:

- Inspector values, toolbar dragging, keyboard nudging, previews, and committed
  edits share one semantic transform module. Absolute field edits and gesture
  deltas remain distinct inputs while producing the same result for an
  equivalent adjustment.
- Normal column Stagger edits the relative stagger parameter. Independent
  offsets remain available as clearly named adjustments with a reset action.
- Relations shows useful inherited information immediately: matrix membership,
  shared pitch, column stagger reference and downstream influence, key local
  adjustments, and attached assembly members. Display explicit constraints
  separately; navigate to or highlight live referenced objects.
- Existing saved offsets keep their meaning and geometry. New toolbar behavior
  must not silently reinterpret stored values or normalize projects on open.

## Decisions the plan must resolve

1. **Rows and axes:** the existing Stagger tool supports row and column gestures
   and two-dimensional offsets. Decide how row adjustment and horizontal motion
   remain available once column Stagger edits a scalar relative Y value.
   Specify pointer constraints, snapping, Alt behavior, and keyboard directions.
   Preserve useful existing behavior under accurate names.
2. **Transform coordinates:** define deltas through matrix rotation/mirroring
   and cumulative splay. An inspector value and a gesture must affect the same
   matrix coordinate parameter. Specify behavior at column zero and on linked
   halves.
3. **Module seam:** compare reusing/deepening the existing transform-operation
   module with a focused typed Core edit. Choose the smallest coherent design.
   Core owns durable placement rules; presentation expresses intent. List any
   required public visibility/interface changes for user approval rather than
   assuming it. Avoid separate toolbar and inspector mutation implementations.
4. **Relations projection:** define what appears for matrix, row, column, key,
   and companion selections. Explain inherited versus explicit relationships,
   local adjustments, references that depart, and independently constrained
   companions. Derive this from accepted state; no second relationship store.
5. **Settlement:** field values set against accepted state; gesture deltas compose
   with accepted edits under the existing Session contract. Confirm the relevant
   gesture route from source and preserve its actual revision/transaction
   guarantees. Keep silent retirement, inline failure, one Undo step per drag,
   cancel behavior, and newer drafts intact.

## Existing fixes to preserve

- `39c9bbac2`: bulk enable/disable/delete across matrices; deletion distinct from
  disabling; Switches and Keycaps visibility independent; resize reflow moves
  assembly members.
- `f65ed01a6`: ordinary matrix key moves update cell-local parametric offsets;
  assembly placement participates in the constraint dependency graph. Preserve
  explicit companion constraints/placements and authored companion offsets.
  See `core/tests/key_move_assemblies.rs` and `core/src/constraints.rs`.

Preserve document identities and existing project geometry. New structural
information should describe the current model without adding redundant
pairwise constraints. Keep PCB, electrical remap, and unrelated inspector work
outside this effort.

## Verification and slice requirements

Plan a RED-before-GREEN regression at the real transform seam: equivalent
toolbar and inspector adjustments must produce equivalent parameters and Core
geometry. Include preview, commit, and keyboard routes; update existing tests
that intentionally assert offset semantics only after assigning them to the
appropriate new behavior.

Required scenarios: cumulative three-column movement; independent offset
behavior; rotated/mirrored/splayed matrices; row behavior; attached assemblies;
linked halves; queued unrelated edits; Undo/Redo and drag cancellation;
save/reopen without geometry changes; meaningful Relations on a blank matrix;
selection changes and departed relationship targets. Name the test seam and
observable assertion for each slice.

Locate existing coverage in `matrix_transform_operation.rs`,
`objects/layout_transform_toolbar.rs`, Core matrix tests, and
`web/src/presentation/layout_component_inspector_tests.rs`. Native checks do not
execute WASM presentation. Include affected `check.py` steps, mounted browser
coverage, and a real product gesture check. Browser suites run sequentially with
one Chrome owner and the repo's configured timeout. `CARGO_BUILD_JOBS=2` is
appropriate here; the rustup shim at `/home/chris/.rustup-toolchain-shim` is
available if the proxy fails.

Return a concise recommendation, explicit decisions/open questions, file
ownership, dependency order, acceptance criteria, and verification commands.
Split work where edits can be independent after the semantic contract is fixed;
reserve shared transform and projection files for one owner. Report the plan's
limitations honestly: earlier source investigation and previous fix checks are
not verification of this new behavior.
