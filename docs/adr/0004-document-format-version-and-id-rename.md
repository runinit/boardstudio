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
