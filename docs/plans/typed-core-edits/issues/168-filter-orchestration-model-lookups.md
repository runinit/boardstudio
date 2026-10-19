# 168: Return compact orchestration model lookups

Status: ready-for-agent
Type: build
Priority: P2
Blocked by: None (can start immediately)
Related: [Catalog lookup research](../../../research/retro-orchestration-catalog-filter.md)

## What to build

Provide a small repo-owned, testable catalog normalizer and an agent-facing usage
recipe. An agent selecting a provider/model should receive only that exact match
and its supported options, ready for delegation, without displaying the complete
orchestration catalog. The first supported path is Codex code mode: transform the
tool result in memory before emitting it to the model. A helper applied after a
direct tool response enters context cannot provide this benefit. No new app-server
endpoint or global configuration change is required.

One earlier lookup printed roughly 44,494 tokens. Research observed that the live
response supplies equivalent structured and JSON-text representations. These are
session observations, not a fixed catalog size or a promised token-cost reduction.

## Acceptance criteria

- [ ] Lookup accepts an exact provider instance ID and model ID. Success emits only
  that pair, its label and its option IDs, current values and allowed choices.
  Matching labels or model IDs under other providers cannot change the selection.
- [ ] Prefer structured content; use validated JSON text as a fallback when
  structured content is absent. A response containing both forms is handled once.
- [ ] Missing provider/model, duplicate exact matches, malformed fallback JSON and
  unexpected response shape produce concise, distinct diagnostics. A valid empty
  options list remains distinguishable from an error. Errors never dump the catalog
  or silently select a different provider/model.
- [ ] Deterministic fixture tests cover successful selection from a large catalog,
  repeated labels/model IDs across providers, both representations, text fallback,
  empty options and each failure above. Include unrelated sentinel data and verify
  it is absent from rendered output.
- [ ] Demonstrate a supported Codex code-mode path that retrieves capabilities,
  applies the normalizer before emitting output, and exposes only the selected
  match. Do not assume that a code-mode isolate can import repo files or launch
  subprocesses; show how the tested normalizer is actually executed there. If no
  pre-exposure path works, report the blocker rather than closing on fixture tests.
- [ ] The narrow agent recipe documents that demonstrated path and reuse of its
  IDs/options for delegation. Direct-call harnesses are explicitly unsupported by
  this filtering mechanism. Document an ACP terminal alternative only if its
  prerequisites and piping behavior are demonstrated; do not require that fallback.
- [ ] Place the recipe in active project agent documentation with a conditional
  inbound link from root agent guidance. Keep implementation details in the linked
  recipe; do not edit external settings or depend on a new T3 API.
- [ ] Register the helper's tests in the standard tooling gate and run that gate
  plus the documentation-link check. Run the supported live smoke path when
  available, printing only the match; otherwise record it as an outstanding gate,
  without claiming provider execution or completed pre-exposure integration.

## Comments

Created at the user's request from retro item 3 after primary-source research.
Independent of the other three retro follow-ups.

Revised after Claude Opus audit: require a demonstrated pre-exposure execution path,
an inbound recipe link, and persistent tooling-test registration.
