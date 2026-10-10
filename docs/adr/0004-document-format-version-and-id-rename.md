---
status: accepted
---

# Version persisted documents and remove Ergogen identifiers

Persisted documents had no format version. On 2026-10-05 the user chose to
rename persisted `ergogen:` definition and model identifiers instead of keeping
them for compatibility, because footprint generators no longer depend on
Ergogen. Documents now carry a format version; those without one are version 1.
Ordered migrations run at Core's load boundary for browser storage, archive
import and bundled examples, and migrated projects are written back immediately
so nothing past that boundary handles old identifiers.
The one generator source ID containing `ergogen`, `ceoloide/utility_ergogen_logo`,
becomes `ceoloide/utility_logo` in the same migration, so no persisted identifier
keeps the name.

Rejected: permanent aliases for old identifiers, which would leave two names in
every lookup, and an unversioned rewrite on each load, which gives later schema
changes no ordering.

## Follow-up

Format version 4 (generator parameters ticket 03) converts the numeric text that version 2 and
3 documents saved for generator parameters to numbers, on definitions, parts and assembly
members, and removes empty or whitespace-only number and list text. Non-empty list text such as
`"[1,2,3]"` stays text, as it did in the old 1 to 2 step. The step is ordered like the others, so version 1 documents pass through it too, and
the rewritten document is written back once. Names no generator declares and values that cannot
be converted are kept for Core's validation to report.

Format version 5 (mirrored halves, [ticket 17](../plans/typed-core-edits/issues/17-mirrored-half-schema-and-format-5.md),
[ADR-0010](0010-mirrored-half-is-a-core-maintained-reflection.md)) adds optional `constructionSource`
and `halves` to mechanical mirror pairs. The version 4 to 5 step changes only `formatVersion`, so
no data is rewritten and a pair without the new fields stays unpaired. An older build refuses a
version 5 document, so it cannot drop the record silently.
