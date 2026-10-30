# 13: Split `boardstudio_core`

Status: wontfix
Type: research
Blocked by: none
Parent: [map](../map.md)

## Question

Would splitting Core give the estimated 60 to 120 s cold? Are there real seams, and does a first cut pay?

## Answer

No. Drop it as a build-time change. Best cold gain is about 15 to 30 s for a change of weeks, and warm gain
reaches only a few leaf crates. Reopen for a design reason, not a speed one.

## Evidence

Module figures come from a text scan of `crate::x` paths in `core/src` at `510861b11`, so a planning estimate.

**Cost.** Core compiles 9 times: 288 CPU-s and 138 s attributed wall (build 58, test 27, tooling 22, lint 19,
browser 13). 138 s is the ceiling for removing Core entirely, and a split removes none of its code.

**No layering to cut along.** 71,300 of 77,919 production lines are one strongly connected component of 23
modules. Only `hash` (17 lines) and `electrical_profiles` (259) sit below it; `lib`, `keycaps`, `outline*`,
`validate` and `archive` (about 6,300) sit above. `model` is not a base layer: 24 modules depend on it, and it depends on 12 of them, for
instance `model.rs:1307` (`constraints::driving_constraint`), `:2116` (`mechanical::change::MechanicalEdit`) and
`:2123` (`outline_edit::OutlineEdit`).

**A chain does not parallelize.** Rustc's frontend is serial inside a crate, and a dependent's frontend starts
only after its dependency's metadata exists. Splitting A from B where B needs A leaves frontends in series, with
only codegen overlap. Only siblings help. Candidates after untangling, at most:

- Base: `model`, `matrix`, `geometry`, `constraints`, and others, then `mechanical` (13.7k lines).
- Siblings: `artifact` (7.5k) and `mirrored_half` (21.5k), each needing `mechanical`. `mirrored_half` also has
  back-edges from `constraints` and `validate`.

So at most about 7.5k of 78k lines (10%) overlap in frontend time. Add some codegen overlap and that is about 15
to 30 s across all nine compiles, against the estimate of 60 to 120 s. Part of that hits `test`'s workspace lane,
which is not the critical path while CAD is longest.

**Test code is not a lever either.** 46k of Core's 124k lines are tests, 30k of them in `mirrored_half`. They cost
96.5 CPU-s in the two unit-test compiles. Moving them requires access to private items, and the root test lane is
not critical.

**Warm.** Every consumer uses `model`, and most use more:

| Consumer | Other Core modules used |
|---|---|
| `keycaps`, `ui-model` | none |
| `library`, `keymap` | `migrate`; `electrical_peripherals` |
| `application`, `case`, `catalogue`, `host`, `parts`, `pcb`, `runtime`, `ui-shared`, `layout`, `web` | `mechanical`, `generators`, `electrical`, `outline_edit`, `archive` and others |

An edit to an engine module rebuilds a consumer only because it depends on the crate. A `model`-only crate would
spare 4 of 14 consumers' own rebuilds, and those crates' test compiles are 1 to 8 s. The page and everything else
rebuild anyway. Incremental compilation already narrows work inside Core. Warm gain: small and not measured.

**Seam, in design terms.** There is one real seam. `CoreEngine::request` takes and returns JSON
(`lib.rs:59-98`), and the Core worker client talks through it, so the engine is a deep module behind a small
interface. The types that cross it, `model`, have broad leverage. Extracting *Model* from *Engine* would be a sensible
deepening if you ever want UI crates unable to reach engine internals. It requires the upward edges above to
become either moved types or ports. That is a design decision to take on its merits. It is not a build-time plan.

## Gain

| Cold wall | CPU | Warm |
|---:|---:|---|
| at most -15 to -30 s | about 0, slightly up | 1 to 8 s per spared crate |

Effort: weeks, across 30 modules and all consumers. Risk: high, with a large diff conflicting with active work.

## Reopen when

- A design reason wants the Model and Engine seam, or
- An hour's prototype with a real `cargo build --timings` of a two-crate Core shows more than 60 s on `check.py`.

Verification for that prototype: `python3 scripts/build-time/cold.py core-split` and
`python3 scripts/build-time/report.py rebaseline core-split`. Stop rule: drop it under 60 s.
