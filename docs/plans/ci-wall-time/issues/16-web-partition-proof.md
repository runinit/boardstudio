# 16: Web partition completeness prototype

Status: claimed
Type: prototype
Parent: [CI wall time map](../map.md)
Related: [04: Web-split design](04-web-split-design.md)

## Scope

Claimed 2026-10-29 by the local web partition implementation sub-agent. Prototype only a
deterministic partition and fail-closed completeness check based on the existing browser plan,
with hermetic tests. Do not execute shards, change CI, reuse artifacts, or assert a speedup.
This does not decide or approve ticket 04's human-owned topology and artifact-sharing decision.

Acceptance for the prototype: preserve the exact existing browser commands, including separate
page listing, page-run isolation and filtered crate launches; reject omitted, duplicated and
unexpected launches and missing, failed or stale shard results. Explain the source-identity and
execution evidence still needed before any aggregate can serve as a required gate.
