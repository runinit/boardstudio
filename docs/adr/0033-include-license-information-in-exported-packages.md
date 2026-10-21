---
status: accepted
---

# Include licence information in exported packages

Exported packages include a discoverable `license.md` file at the package root so
the recipient receives licence and attribution information with the distributed
contents rather than depending on access to BoardStudio or its source checkout.
The user selected this portable-information boundary on 2026-10-20 after research
found that preserving vendor bytes alone did not preserve their separate notices.

Carry applicable known licence/attribution notices and source references for packaged
third-party contents without inventing missing authorship, revisions or permissions.
The file documents the contents' actual terms; it does not relicense all assets under
one blanket project licence or establish legal compliance merely by existing.
Missing information must be identified honestly. Unknown distribution terms alone
do not block export, as confirmed in the amendment below; vendor-specific legal
interpretation remains separate from the application's admission policy.

This supplements [ADR-0020](0020-validate-portable-project-packages-in-core.md)'s
package integrity contract. Portable project archive admission must deliberately
support the bounded `license.md` sidecar as well as export; it currently rejects
unknown entry names, so adding the file is not an export-only change. Preserve path,
duplicate, resource and asset-hash validation. The requirement concerns packages,
not inserting Markdown into individual CAD/model file formats.

The [provenance research](../research/2026-10-20-compatibility-identity-provenance-deep-dive.md#c-portable-provenance)
inspects a real archive, sampled vendor source headers and catalogue-to-output
metadata paths. It establishes information gaps, not a licence violation. This
record accepts the packaging policy, not delivery of the file in current exports.

## Amendment: modification notes are optional (2026-10-20)

The user chose optional user-authored modification notes for customized content.
Include a note when supplied and identify unavailable history honestly; do not
invent authorship or changes, and do not block export solely because the user has
not written commentary. This does not make known applicable vendor notices optional,
replace their actual terms or decide whether unknown distribution terms admit a
particular export. No automatic change-history reconstruction is required.

## Amendment: nested PCB packages can travel independently (2026-10-20)

The user confirmed that the inner KiCad ZIP in a PCB handoff is an independently
forwardable package. Include its own root `license.md` covering its contents;
notices must not depend on the outer assembly/wiring package travelling with it.
The outer package also retains its root notice with clear coverage for what it
distributes. Preserve notices inside original vendor bytes and do not imply that
one common notice relicenses every nested asset. This establishes distribution
scope, not implementation completion or legal sufficiency of any particular text.

## Amendment: unknown terms are disclosed without blocking export (2026-10-20)

The user chose to proceed when distribution terms are unavailable, with an explicit
“licence unknown” notice identifying the affected content in `license.md`. Do not
require an acknowledgement or block export solely because those terms are unknown.
Keep any known original notices and other required integrity/readiness checks.

Unknown information is not a permission grant or a finding that redistribution is
lawful. This choice neither waives known restrictions nor authorizes substituting
invented terms. It settles product admission, not legal interpretation or a new
licence-enforcement system.
