# Architecture decisions

Accepted decisions, oldest first. Read one when you change the area in its last column. A new ADR
gets the next number and a row here; `scripts/check-adr-index.py` (part of
`python3 scripts/check.py repo`) fails for an ADR that is not listed.

| ADR | Decision | Read when you change |
|---|---|---|
| [0001](0001-fixed-edited-outlines.md) | An edited board outline is a separate copy and stays fixed when components move. | outline edits |
| [0002](0002-linked-outline-refinements.md) | An accepted outline refinement stays linked to its components; Freeze makes a fixed copy. | outline suggestions and refinements |
| [0003](0003-rust-footprint-generators-in-core.md) | Footprint generators are Rust inside Core; the JavaScript is gone. | footprint generators |
| [0004](0004-document-format-version-and-id-rename.md) | Persisted documents carry a format version; migrations run at Core's load boundary and a build refuses a newer version. | the saved document shape |
| [0005](0005-resolve-queued-edits-at-execution.md) | A commit is an intent; Core resolves each queued edit against the accepted document when it runs. | edit queuing and resolvers |
| [0006](0006-replace-document-for-import-and-recovery.md) | Whole-document replacement is reserved for import and recovery. | `ReplaceDocument` or an edit resolver |
| [0007](0007-matrix-gestures-edit-named-parameters.md) | Matrix gestures edit named placement parameters through Core. | matrix placement, stagger, splay |
| [0008](0008-linked-halves-stay-linked.md) | A mechanical pair keeps everything linked; exceptions are explicit. | mechanical features of a pair |
| [0009](0009-generator-parameters-checked-in-core.md) | Core checks generator parameter types and names. | generator parameters |
| [0010](0010-mirrored-half-is-a-core-maintained-reflection.md) | A mirrored half is a Core-maintained reflection with explicit exceptions. | mirrored halves, correspondences |
| [0011](0011-part-offset-is-its-own-constraint-kind.md) | A part-axes offset is its own constraint kind, not a property on Offset. | placement constraints |

## Notes

- Number 0003 was used twice. The first ADR 0003, "Rust application ownership and browser trial
  boundaries" (accepted 2026-10-01), was removed with the migration machinery in `3cdeb2ac2`
  and the number was reassigned on 2026-10-05. Read the original with
  `git show 3cdeb2ac2^:docs/adr/0003-rust-application-ownership.md`. Numbers are not reused.
