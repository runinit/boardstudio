function normalizeCatalogLookup(result, providerInstanceId, modelId) {
  // The whole file is this one function declaration, with no imports and no Node globals: its
  // text is pasted into a code-mode isolate and checked there (docs/agents/model-lookup.md).
  function fail(code, message) { return { ok: false, code, message }; }
  function isId(value) { return typeof value === 'string' && value.trim() !== ''; }
  function providersIn(value) {
    return value && typeof value === 'object' && Array.isArray(value.providers) ? value.providers : null;
  }
  function firstText(content) {
    for (const block of Array.isArray(content) ? content : []) {
      if (block && block.type === 'text' && typeof block.text === 'string') return block.text;
    }
    return null;
  }

  if (!isId(providerInstanceId) || !isId(modelId)) {
    return fail('missing-input', 'providerInstanceId and modelId are both required exact IDs');
  }
  if (!result || typeof result !== 'object') return fail('unexpected-shape', 'expected a tool result object');

  let providers = providersIn(result.structuredContent);
  if (result.structuredContent != null && !providers) {
    return fail('unexpected-shape', 'structured content has no providers array');
  }
  if (!providers) {
    const text = firstText(result.content);
    if (text === null) return fail('unexpected-shape', 'no structuredContent.providers and no text content block');
    try {
      providers = providersIn(JSON.parse(text));
    } catch (error) {
      return fail('malformed-text-json', 'text content block is not valid JSON');
    }
    if (!providers) return fail('unexpected-shape', 'text content JSON has no providers array');
  }

  const requested = providers.filter(entry => entry && entry.providerInstanceId === providerInstanceId);
  if (!requested.length) return fail('provider-not-found', `no provider instance ${providerInstanceId}`);
  if (requested.some(provider => !Array.isArray(provider.models))) {
    return fail('unexpected-shape', 'selected provider has no models array');
  }
  const matches = requested.flatMap(provider =>
    (Array.isArray(provider.models) ? provider.models : []).filter(entry => entry && entry.id === modelId));
  if (!matches.length) return fail('model-not-found', `provider ${providerInstanceId} has no model ${modelId}`);
  if (matches.length > 1) return fail('duplicate-match', `${matches.length} entries match ${providerInstanceId}/${modelId}`);

  const model = matches[0];
  if ((model.options != null && !Array.isArray(model.options))
      || (model.options || []).some(entry => !entry || !isId(entry.id)
        || (entry.options != null && !Array.isArray(entry.options))
        || (entry.currentValue != null && !['string', 'boolean'].includes(typeof entry.currentValue)))) {
    return fail('unexpected-shape', 'selected model has malformed options');
  }
  return {
    ok: true,
    providerInstanceId,
    id: modelId,
    label: typeof model.label === 'string' ? model.label : null,
    options: (Array.isArray(model.options) ? model.options : []).map(entry => ({
      id: entry && entry.id,
      value: entry && entry.currentValue !== undefined ? entry.currentValue : null,
      choices: entry && Array.isArray(entry.options) ? entry.options : [],
    })),
  };
}
