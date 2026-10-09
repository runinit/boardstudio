# Handoff: ticket 04, refresh included keyboards

> Progress since this handoff is recorded in the ticket's [Outcome](04-refresh-included-keyboards.md#outcome); the draft below was reviewed, repaired and committed.

Ticket: [04-refresh-included-keyboards.md](04-refresh-included-keyboards.md) (`Status: claimed`, claim commit `f6a6a3a46`)
Branch/worktree: `typed-core-edits/mechanical`, `.worktrees/tce-matrix`. Not merged to `dev`, not pushed (user instruction: do not merge into `dev` yet).

## State at handoff

Blockers are cleared: mechanical 02 and matrix 11 are resolved. An implementation agent was stopped by the user partway through, so **there is uncommitted, unreviewed, unbuilt work** in the worktree:

- `core/examples/demo_projects/mechanical.rs` (new, ~253 lines): applies mirror-pair state through the public `apply-mechanical-edit` op (`link_split`, `instance_edit`, `MEASURED_BAYS`, `SOFLE_BAYS`).
- `core/examples/demo_projects/mod.rs`: `mod mechanical;`; Sofle loop and measured loop call `mechanical::link_split`; `mechanical.rs` added to the provenance hash list.
- `core/examples/demo_projects/recipes.rs`: `keyboard()` and `sofle()` now return `(seed, Option<f64>)` where the number is the X offset `c` of the mirror map `right = (c - left.x, left.y)`; split seeds get a `parameters.mechanicalPair` note.
- `core/tests/fixtures/demo-projects-baseline.json`: a one-line change, not yet justified per keyboard.

Nothing in that diff has been compiled, tested, linted or reviewed. Treat it as a draft; verify before keeping. Do not `git stash`; stage explicit paths only.

## Findings from the read-only inventory

- 20 shipped fixtures (matches `core/tests/demo_projects.rs:50,85` and the baseline): 15 measured (`corne, lily58, klor, sweep, chocofi, totem, cantor` split; `reviung41, gh60, discipline, mysterium, voyager97, voyager104, plaid, lumberjack` unibody), 3 Sofle (`sofle`=v2, `sofle-rgb`, `sofle-choc`), the prebuilt `reviung41` archive (copied from `content/archives/`, not a recipe), `vik-module-review` (`review.rs`).
- Every recipe is raw JSON with `mechanical:null`, `flipped:false`; only the Sofle v2 gets a project-level gasket via `set-mechanical`.
- Halves are separate boards with the mirror baked into coordinates by the recipe (`recipes.rs` mirror loop). `CreateMirroredPair` / `LayoutMirrorLink` only join layouts on the **same** board, so they do not fit; the fit is `MechanicalChange::LinkMirrorPair` with a `Pose2` mapping and per-feature correspondences (`core/src/model.rs` ~914-1100, `change.rs`).
- KLOR and Lily58 are source-right (`recipes.rs`), so their mapping differs.
- The controller bay sits at each PCB's right edge, so its openings are deliberately local on the right half (the draft encodes this).
- `reviung41` archive and `vik-module-review` should stay untouched (a prebuilt original that five core tests and `reviung41-outline-original.json` rely on; a module review with no halves).
- The reused-PCB flipped-instance case is covered by **no** current example. Ticket requires a bounded new demonstration (a second instance sharing a `boardId` with `flipped:true`), not a retrofit.

## Plan for the remaining work

1. Compile the draft (`cargo build --example prepare_demo_projects`), run `core/tests/demo_projects.rs`, and read each baseline diff. The baseline comparison tolerates 1e-9 and requires equal array lengths, so any key-position drift is a failure, not an update.
2. Decide per keyboard: pair (symmetric splits: Corne, Sweep, Chocofi, Cantor, TOTEM, Sofle v2/RGB/Choc), pair with a different mapping (KLOR, Lily58), or independent (all unibody). Keep deliberate half-specific hardware (bay, reset, TRRS) as local/omitted members.
3. Add the flipped reused-PCB demonstration and a core test for it.
4. Core tests through the public request interface: pairs open, a paired value edits both members, a local exception stays local, Undo/Redo, save/reopen round trip.
5. Update the baseline deliberately with a per-keyboard record of intentional changes.
6. Checks: `python3 scripts/check.py repo`, `lint`, `build`; native `test` only in a **non-nested** worktree (e.g. `/tmp/boardstudio-tce-verify`; nested paths break CAD cargo discovery). Browser-affecting? Not for recipe-only changes, but the ticket requires mounted/live checks for paired editing and asymmetry: run `check.py browser` and a real-product pass on a built fixture, and report anything not run. **Never run `check.py release`.**
7. Standards and Spec reviews of the source/data changes (ticket acceptance), then write the Outcome (inventory, per-keyboard intentional changes, result pins, unmet items) and a `map.md` Progress line; set `Status: resolved` only if acceptance holds.

## Constraints that still apply

- No push, `dev` merge or PR without user approval. Main checkout holds the user's uncommitted `AGENTS.md`, `DESIGN.md`, `PRODUCT.md`, `docs/README.md` and untracked docs; leave them alone.
- Stage explicit paths; never `git stash`; run GitNexus impact before editing existing symbols and detect-changes before committing (the index is stale and compare scopes are capped, so say so).
- 12 cargo jobs (default config); sandbox lifted only for wasm-pack, Chrome, podman.
- Commit trailer: `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`.
- Rebuilding a keyboard does not establish physical fit or electrical qualification; keep hardware qualification as findings.

## Open decisions for the user

- Whether pairing should extend to all seven split keyboards or only the symmetric ones.
- Whether unibody keyboards get independent mechanical config now or are left as they are.
- Whether to keep the stopped agent's draft or restart the recipes from `HEAD`.
