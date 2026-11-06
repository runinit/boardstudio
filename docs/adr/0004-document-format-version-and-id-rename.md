---
status: accepted
---

> The ordered migrations below record earlier formats. For planned layout-parity
> version 7, that migration policy is superseded by [ADR-0036](0036-matrices-follow-ergogen-point-layout.md)
> and [ticket 13](../plans/ergogen-layout-parity/issues/13-format-reset-and-recovery.md#answer):
> refuse older input without migration; preserve stored data until explicit deletion.

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
version 5 document, so it cannot drop the record silently. Core's `Open` refuses a record that
names a missing board or item, or repeats a correspondence, with a message that locates it.

Format version 6 introduces the persisted framed-constraint kind described in
[ADR-0010's framed-constraint amendment](0010-mirrored-half-is-a-core-maintained-reflection.md#amendment-selective-exceptions-and-framed-constraints-2026-10-11).
The ordered version 5 to 6 migration changes only `formatVersion`: it does not invent
links, rewrite existing constraints or alter geometry. The version transition lets
older builds refuse a document whose new constraint semantics they cannot preserve.

Later additions within format version 6 include optional mechanical identity reservation
counters for critical fits, hardware and mounts. They are omitted at zero, so absence
is the same unset state as zero; a successful Open starts a new document session and
seeds them from the live identities it receives. These counters needed no additional
migration step or version bump: an older build dropping the bookkeeping counters
cannot change accepted geometry, and reloading starts a new session in which the
previous session's removed identities may be reused. An explicit `u64::MAX` counter
is the exhausted sentinel and never allocates again. This compatible addition does
not remove the earlier version 5 to 6 migration for framed constraints.

## Amendment: layout-parity format reset (2026-10-23)

The user accepted document **`formatVersion: 7`**, retaining the separate
**`boardstudio/v2`** identifier, IndexedDB version 1 and existing archive metadata.
Browser list/load, archive import and Core Open must apply controlled no-migration
validation. Older/unversioned input is unsupported; newer versions need a compatible
build, while malformed input fails validation. A constant-only bump or a version-stamp
migration does not satisfy this policy.

Incompatible saved projects and assets remain unchanged, with JSON-only raw download,
fresh New project and guarded confirmed per-project deletion. There is no global data
wipe, automatic conversion or legacy archive exporter. Failed input validation preserves
current work/history; successful replacement starts a fresh editor session and follows
the existing persist-before-publication and recovery rules. Detailed requirements and
acceptance are in [ticket 13](../plans/ergogen-layout-parity/issues/13-format-reset-and-recovery.md#answer).
This amendment records the planned contract, not a claim that the version-6 code has
already switched to it.
