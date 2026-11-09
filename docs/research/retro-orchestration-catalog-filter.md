# Compact T3 model-catalog lookup

Researched 2026-10-19 UTC from the live T3 orchestration tool metadata and this
repository's agent-document conventions. The catalog call was inspected in
memory and only counts and an exact model match were printed. No app-server
implementation source was available in this workspace, so internal catalog
serialization and downstream rendering behavior beyond the exposed tool
response are not established here.

## Finding

The exposed `orchestrator_capabilities({})` result includes both a
`structuredContent` object and one text content block. In this session, the
text block parsed as JSON identical to `structuredContent`; each representation
was 71,949 characters (143,898 characters together, before other result
metadata). The structured catalog contained eight provider entries and 260
model entries. A prior session observation recorded an approximately 44,494
token dump while looking up one model. These are observations of tool output,
not claims about the app's internal storage or token accounting.

The tool's exposed response shape identifies the fields needed for selection:
provider instance ID, model ID and label, and optional model options. The tool
also exposes option IDs, current values, and choices. A lookup for
`opencode` / `opencode-go/deepseek-v4.1-flash` found a matching model with a
`variant` option; it did not need the other providers or models to answer the
selection question.

## Recommended ownership and behavior

Keep the first durable helper in this repository as a small agent-facing
recipe, with a conditional inbound link from root project agent guidance. Register
its deterministic tests in the [standard tooling gate](../../scripts/check.py),
which enumerates tests explicitly rather than discovering new files automatically.
The capability call belongs to T3's runtime, outside this
repository's source and configuration ownership. A repository helper can
normalize and render the returned value without requiring a T3 app-server API
change; avoid adding an app-specific API dependency or trying to change global
agent configuration in this ticket.

The demonstrated research path was Codex code mode: await the capabilities tool
inside `functions.exec`, retain the result in memory, and emit only the filtered
selection through `text`. A direct tool response may already have entered model
context before a repo helper can filter it, so direct-call harnesses are outside
this mechanism's scope. Implementation must demonstrate how the tested normalizer
executes in the code-mode isolate; do not assume repo imports, filesystem or
subprocess access there. An ACP terminal pipeline is an optional alternative only
where its environment prerequisites and behavior have been demonstrated. If no
supported pre-exposure path works, fixture tests alone cannot close the ticket.

The helper should accept an exact provider instance ID and exact model ID, and
emit only the matched provider/model IDs, display label, and that model's
option IDs, current values, and allowed choices. Preserve the selected IDs so
the result can be used directly for delegation. Do not search by display label
alone: this live catalog included the same DeepSeek model ID under different
provider instances, with different option names. Do not silently choose the
first result if the exact pair occurs more than once.

For the live shape, prefer `structuredContent`. If absent, parse the first text
block only when it is valid JSON with the expected provider/model structure.
If both forms are present, use one representation and never print both. On a
missing provider or model, report a concise not-found result and optionally
list exact provider IDs or candidate model IDs within the requested provider.
On duplicate matches, malformed JSON, or an unexpected shape, return a short
diagnostic with the reason; never fall back to dumping the full catalog. Keep
errors distinct from a valid empty options list.

## Ticket-sized acceptance checks

- Given a catalog with many providers and models, selecting an exact provider
  and model prints only that pair and its options.
- Models with the same label or model ID under different providers do not
  produce an ambiguous selection when the provider ID is supplied.
- Missing matches, duplicate exact pairs, malformed text fallback, missing
  structured content, and an unexpected response shape produce concise,
  distinct diagnostics without printing raw catalog data.
- A result containing both structured content and equivalent JSON text is
  handled once, not duplicated.
- The helper can be used with the existing runtime tool result and does not
  require a new T3 endpoint or edits to external/global configuration.
- A supported live path demonstrably filters before model exposure. The recipe
  has an inbound navigation link and its tests run in the standard tooling gate.

## Sources and limits

- Live first-party Codex tool declaration: `mcp__t3_code__orchestrator_capabilities({})`
  as exposed to this session. Its typed response and the observed result include `structuredContent`
  with `providers[]`, each provider's `providerInstanceId` and `models[]`,
  each model's `id`, `label`, and optional `options[]`. Runtime result
  observation above was derived by parsing and comparing the two response
  representations without printing their full contents. This is not evidence that
  every harness exposes the same typed declaration or pre-exposure execution path.
- [Project agent guidance](../../AGENTS.md) points project-specific agent
  guidance into `docs/agents/`; [domain guidance](../agents/domain.md) confirms
  the repository's small linked-document pattern. These support keeping a
  project-owned recipe near the agent docs rather than expanding root
  `AGENTS.md` with implementation detail.
- No publicly documented T3 implementation contract or app-server source was
  consulted. Any behavior not stated in the exposed tool schema or observed
  response is intentionally left as an implementation choice.
