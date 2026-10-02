# Refactor handoff: current Case and Keymap evidence

**Status:** draft for the existing refactor register. This handoff adds evidence
to RF-001, RF-006, RF-009, and RF-013; it proposes no new RF IDs and does not
edit or approve the canonical register. It records source, native-check, and
public-browser results at their actual evidence levels. It makes no broad
architecture conclusion and no performance claim.

The canonical entries are [RF-001](../../../../docs/migration/POST-PORT-REFACTOR.md#rf-001),
[RF-006](../../../../docs/migration/POST-PORT-REFACTOR.md#rf-006),
[RF-009](../../../../docs/migration/POST-PORT-REFACTOR.md#rf-009), and
[RF-013](../../../../docs/migration/POST-PORT-REFACTOR.md#rf-013).

## RF-001 — shared presentation and Runtime integration hotspots

Two bounded source issues relevant to this existing finding are corrected:

- Keymap is now admitted by the same private `has_inspector` decision used for
  the right grid track, compact Inspect control, and InspectorPanel mount. This
  fixes the reviewed omission where the outer Inspector condition excluded
  Keymap and left its inner panel unreachable. The final Keymap mount review
  calls out all three surfaces; the later Case mount also uses the shared
  predicate. See the source reviews
  `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/.scratch/dioxus-frontend-v1/evidence/case-keymap-current/keymap-final-mount-standards-review.md`,
  `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/.scratch/dioxus-frontend-v1/evidence/case-keymap-current/keymap-final-corrected-mount-spec-review.md`, and
  `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/.scratch/dioxus-frontend-v1/evidence/case-keymap-current/case-final-mounted-standards-review.md`.
- Keymap's shared display projection is now an immutable `Rc<KeymapView>` with
  narrow layer/key display records. The reviewed correction removes whole
  keymap, binding-map and arbitrary Part payload copies from the two read-only
  surfaces. See
  `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/.scratch/dioxus-frontend-v1/evidence/case-keymap-current/keymap-corrected-standards-review.md` and
  `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/.scratch/dioxus-frontend-v1/evidence/case-keymap-current/keymap-private-standards-review.md`. This is a source-level
  ownership/copy correction only; no allocation or latency benchmark was run.

The bounded observation supports existing RF-001's integration-hotspot and
ownership concern. It does not establish that a generic workspace framework,
new subscription model, or wider extraction is needed.

## RF-006 — canonical, physical-instance, and sample scopes

Latest Case editor/controller source review covers editor source through
`03cacf16` plus the current private controller (`case_controller.rs` blob
`6cac7c86` in the review snapshot). It reports the previously identified source
findings corrected: draft/request state is keyed by editor identity and full
Scope; clean fields reconcile accepted changes including Undo while dirty
failed drafts survive unrelated updates; identical repeated failures are
settled by request identity and state; a synchronous single-flight guard keeps
a competing submission from stealing the active result; exact operation
feedback is correlated before submission; and stale-scope outcomes are
discarded. The current reviewer report is stored outside the repository at
`/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/.scratch/dioxus-frontend-v1/evidence/case-keymap-current/case-latest-standards-review.md`, with related independent
Case mount/source decisions in
`/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/.scratch/dioxus-frontend-v1/evidence/case-keymap-current/case-final-mounted-standards-review.md` and
`/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/.scratch/dioxus-frontend-v1/evidence/case-keymap-current/case-final-mounted-spec-review.md`.

This is **source-review green only** for those editor/controller details. The
latest Standards review explicitly ran no Cargo or browser check. Native tests
do not establish WASM mounting, repeated-error retry in the public UI,
draft/Undo across scope changes, persistence recovery, or accessibility. The
separate authored-body contract still requires paired busy/recovery and
save/reopen/Undo/Redo behavior. No case body is moved to a physical instance;
the distinction in RF-006 remains relevant.

## RF-009 — parity accounting and evidence provenance

The evidence here illustrates why the levels must stay separate:

- `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/.scratch/dioxus-frontend-v1/evidence/case-keymap-current/case-keymap-native-tests.log` records 12 library tests and
  4 page-binary unit tests passing on the native target. Native cfg selection
  omits page code behind `target_arch = "wasm32"`; these passes are not evidence
  that the current browser page compiles. The neighboring
  `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/.scratch/dioxus-frontend-v1/evidence/case-keymap-current/case-keymap-wasm-clippy.log` is an intermediate failed
  strict WASM check, not a current green: its diagnostics refer to earlier
  in-progress Case/Keymap source. The latest Case source review also says no
  Cargo check was run. No current page-WASM green is claimed by this handoff.
- Chromium offline behavior is a **conditioned environment result**, not a new
  application fix. The retained
  [offline diagnosis](../../../dioxus-frontend-tranche-1/evidence/parts-context-e510dd4c/packaging/offline/DIAGNOSIS.md)
  supersedes its preliminary regression hypothesis. On the unchanged f65
  artifact, isolated `/var/tmp` profiles repeatedly installed the worker and
  rendered the actual offline landing page at root and subpath; final root and
  subpath captures each asserted a controlling worker and 51 cached entries.
  An explicit `/tmp` profile reproduced failure. At the controlled runs,
  `/tmp` had about 3.48 GB available versus about 249.76 GB on `/var/tmp`; the
  report says `/tmp` was not full, but below Chromium 154's blob-paging reserve.
  The causal filesystem condition is reproduced; the precise internal Chromium
  allocation branch is inferred from version-matched source. This evidence
  covers the stated artifact, host, and offline landing shell only, not all
  offline workflows or release hosts. Captures and profile controls are in
  `packaging/offline/environment-controls/` beside the diagnosis.

These results belong in evidence provenance and environment qualification.
They do not measure app performance and do not justify changing production
configuration or renderer code.

## RF-013 — object-tree semantics and accessibility

The public verifier's retained
[f65 browser report](../../../dioxus-frontend-tranche-1/evidence/tree-and-parts-4b05d451/ui-verifier/README.md)
identifies the exact candidate build as `f65b0c83`. Axe reported zero
violations on the exercised candidate page, with one serious `color-contrast`
incomplete result covering 10 nodes; manual contrast review remains unresolved.
The raw candidate and React results are
[`a11y-f65-sofle.json`](../../../dioxus-frontend-tranche-1/evidence/tree-and-parts-4b05d451/ui-verifier/a11y-f65-sofle.json)
and
[`a11y-react-sofle.json`](../../../dioxus-frontend-tranche-1/evidence/tree-and-parts-4b05d451/ui-verifier/a11y-react-sofle.json).
This is bounded axe evidence for that build/page, not proof of all tree states
or overall accessibility. Actual assistive-technology testing remains
unperformed and open; keyboard interaction and axe do not substitute for it.

Keep the existing RF-013 inherited-reference context and current candidate
gate: a React violation does not waive the candidate's accessibility criteria.
The `f65` browser result is newer bounded evidence, not an integration or
acceptance claim for the latest working tree.

## Evidence boundary

The links above deliberately distinguish source review, native-only tests,
strict WASM diagnostics for intermediate source, and bounded browser evidence.
Case's latest changes remain uncompiled/unverified in the public UI; offline
success is profile/filesystem-dependent; candidate contrast is incomplete; and
actual assistive-technology coverage is unknown. No performance result, broad
architecture assessment, acceptance waiver, or new refactoring finding is
asserted.
