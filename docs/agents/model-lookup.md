# Orchestration model lookup

Reading the T3 orchestration catalog for one provider/model without printing the whole
catalog. The helper is [`scripts/model-lookup.js`](../../scripts/model-lookup.js); its
fixture tests are [`scripts/test-model-lookup.cjs`](../../scripts/test-model-lookup.cjs).

## When

Before `delegate_task`, `create_threads` or `t3_thread_launch`: you have an exact
`providerInstanceId` and model ID, and you need that entry's label and option IDs, current
values and allowed choices to pass on. A printed `orchestrator_capabilities({})` result is
not a selection: it carries every provider and model.

## Implementation handoff

A delegated implementation brief names the concrete completion artifact, the verification
required to accept it, and the exact blocker to report if completion is not possible.

## Codex code mode

The isolate has no imports, filesystem, subprocess, `eval` or `Function` constructor, so the
helper is pasted as code. Read and paste the whole of `scripts/model-lookup.js` verbatim;
the probe below checks three representative behaviors, not source identity or complete
equivalence. The capability result stays in an isolate-local binding so only the selection
is emitted. Do not print the raw result, including on errors.

```js
function normalizeCatalogLookup(result, providerInstanceId, modelId) {
  // ... the whole of scripts/model-lookup.js, pasted verbatim ...
}

// Behavioral smoke check: this does not establish source identity.
const PROBE_EXPECTED = '[{"ok":true,"providerInstanceId":"p","id":"m","label":"L","options":[{"id":"o","value":"v","choices":["v","w"]}]},{"ok":false,"code":"duplicate-match","message":"2 entries match p/m"},{"ok":false,"code":"model-not-found","message":"provider p has no model absent"}]';
const probeCatalog = { providers: [
  { providerInstanceId: 'p', models: [{ id: 'm', label: 'L', options: [{ id: 'o', currentValue: 'v', options: ['v', 'w'] }] }] },
  { providerInstanceId: 'q', models: [{ id: 'm', label: 'L', options: [] }] },
] };
const probe = [
  normalizeCatalogLookup({ structuredContent: probeCatalog }, 'p', 'm'),
  normalizeCatalogLookup({ structuredContent: { providers: [...probeCatalog.providers, probeCatalog.providers[0]] } }, 'p', 'm'),
  normalizeCatalogLookup({ structuredContent: probeCatalog }, 'p', 'absent'),
];
if (JSON.stringify(probe) !== PROBE_EXPECTED) throw new Error('inline normalizer failed the behavior probe');

// The capability result never leaves this isolate: keep it in a local const.
// (Tool name carries the harness's MCP prefix, e.g. mcp__t3_code__orchestrator_capabilities.)
const capabilities = await tools.mcp__t3_code__orchestrator_capabilities({});
const selection = normalizeCatalogLookup(capabilities, 'opencode', 'opencode-go/deepseek-v4.1-flash');
text(JSON.stringify(selection));
```

Then reuse the selection for delegation, which takes `{id, value}` options:

```js
if (!selection.ok) throw new Error(selection.code);
await tools.mcp__t3_code__delegate_task({ task, target: {
  providerInstanceId: selection.providerInstanceId,
  model: selection.id,
  options: selection.options.filter(({ value }) => value !== null).map(({ id, value }) => ({ id, value })),
} });
```

## Result shape

`{ ok: true, providerInstanceId, id, label, options: [{ id, value, choices }] }`. `options: []`
is a valid model with no options, not an error. When `ok` is false, the value is a dialog
`{ ok: false, code, message }`:

| code | meaning |
| --- | --- |
| `missing-input` | `providerInstanceId` or model ID missing, blank or not a string |
| `provider-not-found` | no provider instance with that exact ID |
| `model-not-found` | that provider has no model with that exact ID |
| `duplicate-match` | more than one entry matches the exact pair; select from the catalog yourself |
| `malformed-text-json` | the fallback text block is not valid JSON |
| `unexpected-shape` | neither `structuredContent.providers` nor a JSON text block carrying `providers` |

A matching label or model ID under another provider cannot change the selection: only the
exact pair is matched. Structured content wins; JSON text is used only when structured
content is absent, so a result carrying both is read once. Malformed present structured
content is an error. An unknown current option value is `null`: omit it from delegation
to inherit the provider default, or explicitly choose a supported value from `choices`.

## Limits

- Direct-call harnesses are unsupported: a tool result that has already entered model context
  cannot be filtered by this helper.
- An ACP terminal pipeline is not demonstrated here and is not required.
- Verified so far: the fixture tests in `scripts/test-model-lookup.cjs` (registered for the
  tooling step), and the paste-probe-emit block above running in a confined code-mode runtime.
  A live capabilities lookup verifies catalog transport and pre-exposure selection; it does
  not prove that delegating to the selected provider/model succeeds.
