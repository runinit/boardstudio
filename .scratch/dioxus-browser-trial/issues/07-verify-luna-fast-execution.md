# Verify Luna Fast execution for the existing prototype

Labels: wayfinder:research
Type: research
Mode: AFK
Status: resolved
Assignee: /root
Parent: [Dioxus browser trial and Rust application migration](../map.md)
Blocked by: none

## Question

What model, reasoning and Fast-tier requests are supported by installed Codex
0.159.3 for gpt-6-luna, and what independently observable runtime evidence can
confirm each? Identify a session-scoped invocation for future coding without
editing global configuration or widening permissions. Distinguish Fast service
from low reasoning effort, configured requests from observed routing, and
catalogue support from measured latency or guaranteed speedup.

## Comments

- Requested by the user on 2026-10-01: coding in Luna 6 Fast for speed.
- Existing architecture/version/scope/quality decisions stay settled. No coding,
  configuration change, dependency install or model escalation is part of this
  investigation. Main session remains coordinator; research uses Luna/medium.
- Read-only investigator; coordinator captures findings under research/ and
  owns tracker updates. No worker delegation or writes.
- Planning worktree: /home/chris/.local/share/boardstudio/worktrees/p2-wayfinder-20261001;
  base 689f8962f77e04b1150a9541128fec7aca28addc. Existing worktrees preserved.

## Answer

Resolved 2026-10-01 from the read-only Luna/medium investigation and independently
checked runtime metadata. [Luna Fast execution evidence](../research/luna-fast.md)
records supported session-only model/effort/Fast requests, doctor exit0 and the
catalogue Fast-to-priority mapping. The observed thread confirms Luna/medium;
its stored context lacks a service tier, so served Fast and latency remain
unverified. Global settings stay unchanged. This is a routing investigation,
not acceptance or renewal of P2. Evidence is captured on the isolated planning
branch with a research branch pointer after the documentation commit.
