# Frontend agent routing

[CONSTRAINTS.md](../../CONSTRAINTS.md) owns execution, ownership, qualification and
project skill overrides. [Issue tracker](../../docs/agents/issue-tracker.md) owns
record locations. [Machine routing](agent-policy.json) holds parent assignments.

| Work | Profile |
| --- | --- |
| Mechanical packet with fixed oracle | Luna low |
| Default bounded implementation or evidence collection | Luna medium |
| Complex specified state/ownership work | Luna high |
| Consolidated candidate Standards/Spec review | Sol 6.1 High |
| Difficult evidenced behavioral bug | Astra high, xhigh if still unresolved |

Set model/effort explicitly with a concise partial/no-history fork when supported.
Reuse an existing agent only if its launched profile fits; follow-up does not change
models. Internal packets use subagents, not new user-owned chats. The 30-agent ceiling
is bounded by available runtime slots; record actual limits instead of changing host
configuration. Priority/Fast is a preference, not proof: this tool exposes no per-call
service-tier switch. Record effective runtime settings only when metadata exposes them.

Dispatch includes the existing task/spec pointer, relevant source base, absolute
checkout/branch, owned files, shared interfaces and intended user journey. This can be
a message. Record new RF evidence once; no extra launch/handoff document is required.
The prior allocations and first-tranche proposals remain in
[history](../../docs/migration/history/OPERATING-RULES-before-20261003.md).
