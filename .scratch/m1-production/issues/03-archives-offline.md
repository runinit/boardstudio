# 03: Reload and exchange faithful archives, including offline reopen

**What to build:** Saved REVIUNG41 and Sofle copies reload and exchange archives with React while all required assets survive; cached offline navigation works at both deployment prefixes.

**Blocked by:** 02

**Status:** archive transport and scoped offline policy implemented; full application/browser exchange acceptance in progress

**Category:** behavior-preserving migration; ticket 02 includes the already accepted recovery behavior.

**Authority:** [M1 specification](../spec.md), existing six module specifications and ADR 0003.

- [ ] Archive bytes/assets/extension data round-trip through Rust and React.
- [ ] Invalid imports preserve active document; project switching cannot discard pending commits.
- [ ] Root/subpath storage/cache separation, update, cached navigation and cold missing assets tested in Chromium.
- [ ] Rust owns service-worker policy; maintained JS exceptions require explicit approval.
