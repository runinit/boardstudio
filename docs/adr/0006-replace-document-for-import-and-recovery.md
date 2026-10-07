---
status: accepted
---

# Reserve whole-document replacement for import and recovery

After [ADR-0005](0005-resolve-queued-edits-at-execution.md) every product edit
resolves against the accepted document when it runs, so whole-document replacements
no longer overwrite queued work. About two dozen resolvers still answer with
`ReplaceDocument` ([inventory](../plans/edit-settlement/replace-document-resolvers.md)).
Each one copies a domain rule into the web layer, reports every part and top-level
outline feature as changed, reapplies scripts and recomputes outline geometry
without its cache, even for a single scalar such as a board name. And because a
resolver may return any command, nothing stops one from returning a document it
captured outside its snapshot argument.

On 2026-10-07 the user chose
([ticket 19](../plans/edit-settlement/issues/19-decide-first-typed-core-edits.md)) to
make whole-document replacement the exception: product edits move to typed Core
intents that state what they change, and `ReplaceDocument` is reserved for import and
recovery. This is a direction, not an enforced rule; it has no enforcement date. New
panels should not add replacement resolvers.

Permitted now, by name:

- opening or importing a project archive, and reopening the saved document during
  recovery;
- footprint import into Parts;
- uploading a routed-board reference with its assets.

The remaining replacement resolvers are each converted to a typed intent (a typed
batch intent where the action changes several entities atomically) or added to the
permitted list by their own decision. Nothing is grandfathered by default. The
bulk ones that need that decision are physical setup, geometry-script apply,
generator apply and generator model upload, assembly placement, the reviewed wiring
plan, and matrix and mirrored-half creation.

The work is the [typed Core edits](../plans/typed-core-edits/map.md) effort. Its
first slice is one small board-scoped operation, `SetWiringMode`, chosen to settle
the pattern (operation shape, precise changed IDs, whether it is outline-affecting,
Session/Core contract tests) before larger concepts copy it.

## Consequences

- Typed edits report precise changed IDs and skip replacement-only preparation and
  outline recomputation when they do not affect outlines. They do not make Undo
  cheaper: Core history still stores a full prior document per step.
- When the restriction is eventually enforced, it also closes the gap ADR-0005's
  2026-10-07 amendment leaves open: a resolver can no longer return a stale
  whole document.

## Considered Options

- **Keep `ReplaceDocument` as a first-class edit tool** and add typed edits only where
  they pay off locally: no commitment, but domain rules keep accumulating in web and
  broad changed IDs and recomputation stay for scalar edits.
- **Enforce the restriction now**: forces premature typed designs for bulk setup and
  generator workflows that have not been classified.
