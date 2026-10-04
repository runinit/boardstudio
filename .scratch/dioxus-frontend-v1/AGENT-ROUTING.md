# Frontend agent routing

[CONSTRAINTS.md](../../CONSTRAINTS.md) owns execution, ownership, qualification and
project skill overrides. [Issue tracker](../../docs/agents/issue-tracker.md) owns
record locations. [Machine routing](agent-policy.json) holds parent assignments.

Profile names in [machine routing](agent-policy.json) and in each parent's
`agent_routing` are logical roles. A route resolves each role to a model; assignments are
shared and are not edited per route. Use the route that matches the harness running the
coordinator.

| Role | Codex route | Claude route |
| --- | --- | --- |
| Mechanical packet with fixed oracle (`luna-low`) | Luna low | Sonnet 5.5 low |
| Default bounded implementation or evidence collection (`luna-medium`) | Luna medium | Sonnet 5.5 medium |
| Complex specified state/ownership work (`luna-high`) | Luna high | Sonnet 5.5 high |
| Coordinator (`luna-high` role) | Luna high | Opus 5.5 high |
| Consolidated candidate Standards/Spec review (`sol-review`) | Sol 6.1 High | Opus 5.5 high |
| Difficult evidenced behavioral bug (`astra-debug`, `astra-deep-debug`) | Astra high, xhigh if unresolved | Opus 5.5 high, xhigh if unresolved |

## Codex route

Unchanged allocation. Set model/effort explicitly with a concise partial/no-history fork
when supported. Reuse an existing agent only if its launched profile fits; follow-up does
not change models. Internal packets use subagents, not new user-owned chats. The 30-agent
ceiling is bounded by available runtime slots; record actual limits instead of changing
host configuration. Priority/Fast is a preference, not proof: this tool exposes no
per-call service-tier switch. Record effective runtime settings only when metadata
exposes them.

## Claude route

Models: Opus 5.5 (`claude-opus-5-5`) and Sonnet 5.5 (`claude-sonnet-5-5`).

- Dispatch with the Agent tool and set `model` to `sonnet` or `opus` per the table. Use
  `Explore` for read-only search, `Plan` for design and `general-purpose` for authoring.
- The Agent tool has no per-call effort setting. Effort comes from the agent definition or
  session, so record the requested effort and say when it was not enforced.
- Authors that edit source use worktree isolation. The coordinator (Opus 5.5) keeps sole
  commit, build and publish ownership.
- `SendMessage` continues an agent on its launched model; start a new Agent to change model.
- The consolidated review is one Opus 5.5 invocation per coherent candidate, never the
  author of the packet. The Claude `code-review` skill is the entry point.

The concurrency, heavy-job and package-build limits in `agent-policy.json` apply to both
routes.

Dispatch includes the existing task/spec pointer, relevant source base, absolute
checkout/branch, owned files, shared interfaces and intended user journey. This can be
a message. Record new RF evidence once; no extra launch/handoff document is required.
The prior allocations and first-tranche proposals remain in
[history](../../docs/migration/history/OPERATING-RULES-before-20261003.md).
