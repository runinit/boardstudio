// Deterministic fixture tests for model-lookup.js. No browser, network or Rust build.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const failures = [];
const check = (label, value) => { console.log(`${value ? 'PASS' : 'FAIL'} ${label}`); if (!value) failures.push(label); };

const source = fs.readFileSync(path.join(__dirname, 'model-lookup.js'), 'utf8');

const SENTINEL = 'UNRELATED-SENTINEL-4f9c2b';
const TARGET = { provider: 'opencode', model: 'opencode-go/deepseek-v4.1-flash', label: 'DeepSeek V4.1 Flash (OpenCode Go)' };
const option = (id, value, choices) => ({
  id, label: `${id} ${SENTINEL}`, description: SENTINEL, type: 'select',
  currentValue: value, options: choices, promptInjectedValues: [SENTINEL],
});

// Eight providers, 30 models each, plus the collisions the ticket names: the same
// model ID under another provider, and the same display label under a third.
function catalog({ duplicate = false } = {}) {
  const providers = [];
  for (let p = 0; p < 8; p++) {
    const providerInstanceId = p === 0 ? TARGET.provider : `provider-${p}`;
    const models = [];
    for (let m = 0; m < 30; m++) {
      if (p === 0 && m === 3) {
        models.push({ id: TARGET.model, label: TARGET.label, options: [
          option('variant', 'flash', ['flash', 'pro', 'max']),
          option('reasoning', 'medium', ['low', 'medium']),
        ] });
      } else if (p === 1 && m === 3) {
        models.push({ id: TARGET.model, label: `${SENTINEL} other provider`, options: [option('tier', 'gold', ['gold'])] });
      } else if (p === 2 && m === 5) {
        models.push({ id: 'provider-2/model-5', label: TARGET.label, options: [option('size', 's', ['s'])] });
      } else if (p === 3 && m === 1) {
        models.push({ id: 'provider-3/model-1', label: `${SENTINEL} null options`, options: null });
      } else {
        models.push({
          id: `provider-${p}/model-${m}`,
          label: `${SENTINEL} model ${p}/${m}`,
          options: m % 3 === 0 ? [] : [option(`o${m}`, `${SENTINEL}-v${m}`, [SENTINEL])],
        });
      }
    }
    providers.push({
      providerInstanceId, driverKind: 't3', displayName: `${SENTINEL} ${providerInstanceId}`, models,
      canRunChildTask: true, canRunCrossProviderChildTask: false, constraints: [SENTINEL],
    });
  }
  if (duplicate) {
    providers.push({
      providerInstanceId: TARGET.provider, driverKind: 't3', displayName: 'duplicate provider entry',
      models: [{ id: TARGET.model, label: 'duplicate entry', options: [] }],
      canRunChildTask: true, canRunCrossProviderChildTask: false, constraints: [],
    });
  }
  return { providers, features: { maxBatchThreads: 20, appOwnedSubagents: true }, parentThreadId: SENTINEL };
}

const both = value => ({ structuredContent: value, content: [{ type: 'text', text: JSON.stringify(value) }] });

const base = catalog();
const modelCount = base.providers.reduce((total, provider) => total + provider.models.length, 0);
console.log(`fixture: ${base.providers.length} providers, ${modelCount} models, ${JSON.stringify(both(base)).length} envelope characters`);

const context = {};
vm.createContext(context);
vm.runInContext(source, context, { filename: 'model-lookup.js' });
const loaded = context.normalizeCatalogLookup;
check('helper source uses no Node globals', !/require\(|module\.exports|process\.|__dirname/.test(source));
check('helper loads as one function declaration in a bare V8 context', typeof loaded === 'function');
check('the file text is exactly the function source', source.trim() === String(loaded).trim());

const selected = loaded(both(base), TARGET.provider, TARGET.model);
check('exact pair selects from the large catalog', selected.ok === true);
check('selection carries the exact provider/model IDs and label',
  selected.providerInstanceId === TARGET.provider && selected.id === TARGET.model && selected.label === TARGET.label);
check('selection carries the target options only', JSON.stringify(selected.options) === JSON.stringify([
  { id: 'variant', value: 'flash', choices: ['flash', 'pro', 'max'] },
  { id: 'reasoning', value: 'medium', choices: ['low', 'medium'] },
]));
check('rendered selection omits unrelated sentinel data', !JSON.stringify(selected).includes(SENTINEL));
check('rendered selection is compact', Object.keys(selected).join(',') === 'ok,providerInstanceId,id,label,options');

// Slice 2: cross-provider collisions, repeated pairs and missing selection.
const otherProvider = loaded(both(base), 'provider-1', TARGET.model);
check('a model ID shared by another provider resolves inside the requested provider',
  otherProvider.ok === true && otherProvider.id === TARGET.model
  && otherProvider.label === `${SENTINEL} other provider`
  && JSON.stringify(otherProvider.options) === JSON.stringify([{ id: 'tier', value: 'gold', choices: ['gold'] }]));
const targetSelection = loaded(both(base), TARGET.provider, TARGET.model);
check('a matching label under another provider cannot change the selection',
  targetSelection.label === TARGET.label && !JSON.stringify(targetSelection).includes(SENTINEL));

const duplicate = loaded(both(catalog({ duplicate: true })), TARGET.provider, TARGET.model);
const providerMissing = loaded(both(base), 'provider-none', TARGET.model);
const modelMissing = loaded(both(base), TARGET.provider, 'opencode-go/absent');
check('a repeated exact pair reports duplicate-match', duplicate.ok === false && duplicate.code === 'duplicate-match');
check('an unknown provider reports provider-not-found', providerMissing.ok === false && providerMissing.code === 'provider-not-found');
check('an unknown model reports model-not-found', modelMissing.ok === false && modelMissing.code === 'model-not-found');
check('the three diagnostics are distinct',
  new Set([duplicate.code, providerMissing.code, modelMissing.code]).size === 3
  && !JSON.stringify([duplicate, providerMissing, modelMissing]).includes(SENTINEL));
check('diagnostics carry only ok/code/message',
  [duplicate, providerMissing, modelMissing].every(result => Object.keys(result).join(',') === 'ok,code,message'));

// Slice 3: representations, malformed fallback, empty options, missing input.
const textOnly = value => ({ content: [{ type: 'text', text: JSON.stringify(value) }] });
const textSelect = loaded(textOnly(base), TARGET.provider, TARGET.model);
check('validated JSON text is a fallback when structured content is absent',
  textSelect.ok === true && JSON.stringify(textSelect) === JSON.stringify(selected));

const disagreeing = catalog();
disagreeing.providers[0].models[3].label = `${SENTINEL} text disagrees`;
const bothForms = { structuredContent: base, content: [{ type: 'text', text: JSON.stringify(disagreeing) }] };
check('a response carrying both forms is handled once', JSON.stringify(loaded(bothForms, TARGET.provider, TARGET.model)) === JSON.stringify(selected));
check('a non-text block before the JSON block is skipped',
  JSON.stringify(loaded({ content: [{ type: 'image', data: SENTINEL }, { type: 'text', text: JSON.stringify(base) }] },
    TARGET.provider, TARGET.model)) === JSON.stringify(selected));

const malformed = loaded({ content: [{ type: 'text', text: '{oops' }] }, TARGET.provider, TARGET.model);
check('malformed fallback JSON reports malformed-text-json', malformed.ok === false && malformed.code === 'malformed-text-json');
const oddText = loaded({ content: [{ type: 'text', text: '{"providers":"nope"}' }] }, TARGET.provider, TARGET.model);
const noText = loaded({}, TARGET.provider, TARGET.model);
const nullResult = loaded(null, TARGET.provider, TARGET.model);
const oddStructured = loaded({ structuredContent: { providers: 'nope' } }, TARGET.provider, TARGET.model);
check('unexpected response shapes report unexpected-shape',
  [oddText, noText, nullResult, oddStructured].every(result => result.ok === false && result.code === 'unexpected-shape'));
check('malformed present structured content is rejected rather than rescued by fallback',
  loaded({ structuredContent: { providers: 'nope' }, content: [{ type: 'text', text: JSON.stringify(base) }] },
    TARGET.provider, TARGET.model).code === 'unexpected-shape');
const malformedOptions = catalog();
malformedOptions.providers[0].models[3].options = [null];
check('malformed selected options are not reported as a valid empty option',
  loaded(both(malformedOptions), TARGET.provider, TARGET.model).code === 'unexpected-shape');
check('malformed selected provider models are not reported as a missing model',
  loaded(both({ providers: [{ providerInstanceId: TARGET.provider, models: {} }] }),
    TARGET.provider, TARGET.model).code === 'unexpected-shape');

const emptyOptions = loaded(both(base), 'provider-3', 'provider-3/model-0');
const nullOptions = loaded(both(base), 'provider-3', 'provider-3/model-1');
check('a valid empty options list stays a success', emptyOptions.ok === true && Array.isArray(emptyOptions.options) && emptyOptions.options.length === 0);
check('absent model options stay a success', nullOptions.ok === true && nullOptions.options.length === 0);
check('empty options are distinguishable from an error', emptyOptions.ok === true && malformed.ok === false);

const missing = [loaded(both(base), '', TARGET.model), loaded(both(base), TARGET.provider, null), loaded(both(base))];
check('missing provider/model IDs report missing-input', missing.every(result => result.ok === false && result.code === 'missing-input'));
check('missing-input stays distinct from unexpected-shape', missing[0].code !== oddText.code);
check('no representation or missing-input diagnostic leaks sentinel data',
  !JSON.stringify([malformed, oddText, noText, nullResult, oddStructured, ...missing]).includes(SENTINEL));

// Slice 4: the probe copied into docs/agents/model-lookup.md must match this helper exactly.
const probeCatalog = { providers: [
  { providerInstanceId: 'p', models: [{ id: 'm', label: 'L', options: [{ id: 'o', currentValue: 'v', options: ['v', 'w'] }] }] },
  { providerInstanceId: 'q', models: [{ id: 'm', label: 'L', options: [] }] },
] };
const probeResult = [
  loaded({ structuredContent: probeCatalog }, 'p', 'm'),
  loaded({ structuredContent: { providers: [...probeCatalog.providers, probeCatalog.providers[0]] } }, 'p', 'm'),
  loaded({ structuredContent: probeCatalog }, 'p', 'absent'),
];
const PROBE_EXPECTED = '[{"ok":true,"providerInstanceId":"p","id":"m","label":"L","options":[{"id":"o","value":"v","choices":["v","w"]}]},{"ok":false,"code":"duplicate-match","message":"2 entries match p/m"},{"ok":false,"code":"model-not-found","message":"provider p has no model absent"}]';
console.log(`recipe probe: ${JSON.stringify(probeResult)}`);
check('the recipe probe literal matches the tested helper', JSON.stringify(probeResult) === PROBE_EXPECTED);
const recipe = fs.readFileSync(path.join(__dirname, '../docs/agents/model-lookup.md'), 'utf8');
check('the documented behavior probe stays in sync with the tested literal',
  recipe.includes(`const PROBE_EXPECTED = '${PROBE_EXPECTED}';`));

assert.deepEqual(failures, [], failures.join('\n'));
