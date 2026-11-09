---
status: accepted
---

# Render footprint generators in Rust inside Core

Footprint generators were JavaScript executed by the page and by a separate
preview worker, with Core preparing jobs before and accepting results after.
On 2026-10-05 the user chose to port all generators to Rust and delete the
JavaScript rather than keep the generator bodies behind a browser interface, so
footprint generation has one implementation and no Node packaging. Core renders
generator jobs itself; it already runs in its own worker with executor-epoch
checks, so a dedicated preview worker added only a round trip and a duplicate
validation envelope.

JavaScript coercion quirks are corrected rather than reproduced, and each
correction is listed in the
[migration plan](../investigations/footprint-generators-rust.md). Ported files
keep their source licences, including CC-BY-NC-SA-4.0. Upstream generator
changes must now be ported by hand.

## Amendment: existing projects use current generators (2026-10-20)

The user rejected a promise that saved projects preserve old generated manufacturing
geometry until explicit upgrade. Existing projects use the generator implementation
and applicable defaults in the current application; a generator update may therefore
change subsequent resolved geometry/output without a per-project adoption step.
Saving a definition is not pinning an executable generator or its historical output.
This chooses maintained current generators rather than a frozen-geometry/version
retention system; it does not require changing an explicitly saved parameter value.

This does not waive [ADR-0004](0004-document-format-version-and-id-rename.md)'s schema
migrations, [ADR-0009](0009-generator-parameters-checked-in-core.md)'s parameter admission,
or required identity/terminal migration when pad count/order or meaning changes.
Nor may an update silently override [ADR-0031](0031-preserve-pcb-handoff-protection-across-undo.md)'s
protected signal assignments. No new review/adoption gate is required solely to keep
old geometry, but accepted integrity and safety contracts still apply.

The [compatibility research](../research/2026-10-20-compatibility-identity-provenance-deep-dive.md#a-generator-compatibility)
traces current executable/default resolution and two real precedents: opt-in diode
geometry and migration of omitted-side semantics. Those examples do not establish
that every historical update or saved project has been verified.
