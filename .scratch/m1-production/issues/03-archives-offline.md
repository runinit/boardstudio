# 03: Reload and exchange faithful archives, including offline reopen

**What to build:** Saved REVIUNG41 and Sofle copies reload and exchange archives with React while all required assets survive; cached offline navigation works at both deployment prefixes.

**Blocked by:** 02

**Status:** Implementation complete; real archive exchange and final8f root/subpath offline/storage checks pass; overall milestone acceptance remains open.

**Category:** behavior-preserving migration; ticket 02 includes the already accepted recovery behavior.

**Authority:** [M1 specification](../spec.md), existing six module specifications and ADR 0003.

- [x] Archive bytes/assets/extension data round-trip through Rust and React.
- [x] Invalid imports preserve active document; project switching cannot discard pending commits.
- [x] Root/subpath storage/cache separation, update, cached navigation and cold missing assets tested in Chromium.
- [x] Rust owns service-worker policy; maintained JS exceptions require explicit approval.

See [current acceptance evidence](../ACCEPTANCE.md#current-evidence--final-release-8f509433) for artifact scope and remaining gates. Checked items record bounded completed checks; ticket closure follows the task graph and full acceptance requirements.
