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
