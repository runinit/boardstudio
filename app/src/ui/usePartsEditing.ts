import { useEffect, useMemo, useRef, useState } from 'react';
import type { CompiledFootprint, EditCommand, FootprintCompileJob, JsonValue, PartDefinition, ProjectDoc } from '@boardstudio/v2-contracts';
import { isErgogen, normalizeDefinition } from '@boardstudio/v2-ergogen';
import { runLatest } from './compileLatest';
import { generatorDraft, generatorParameters } from './generatorSettings';
import { libraryPreviewFor, libraryPreviewsToCompile } from './libraryPreview';
import { remapDefinitionNets } from './remapDefinitionNets';
import { makeId } from './workbenchGeometry';

type Inputs = {
  document: ProjectDoc;
  selectedDefinition?: PartDefinition;
  previewDefinitions: PartDefinition[];
  compileFootprints: (jobs: FootprintCompileJob[]) => Promise<CompiledFootprint[]>;
  onEdit: (command: EditCommand) => void;
  setDefinitionError: (error: string) => void;
};

export function usePartsEditing({ document, selectedDefinition, previewDefinitions, compileFootprints, onEdit, setDefinitionError }: Inputs) {
  const [edits, setEdits] = useState<Record<string, JsonValue>>({});
  const [compiled, setCompiled] = useState<{ key: string; results: CompiledFootprint[]; error?: string; pending: boolean }>({ key: '', results: [], pending: false });
  const sequence = useRef(0);
  const parameterSchema = useMemo(() => generatorParameters(selectedDefinition), [selectedDefinition?.generator?.source]);
  const generatorPreview = useMemo(() => selectedDefinition
    ? generatorDraft(selectedDefinition, edits, new Map(document.assets.map((asset) => [asset.id, asset.name])))
    : undefined, [selectedDefinition, edits, document.assets]);
  const definition = previewDefinitions[0] ?? generatorPreview?.definition;
  const definitions = useMemo(() => definition ? [definition, ...previewDefinitions.slice(1)] : [], [definition, previewDefinitions]);
  const compileDefinitions = useMemo(() => libraryPreviewsToCompile(definitions), [definitions]);
  const compileKey = JSON.stringify(compileDefinitions.map((item) => [item.id, item]));
  const currentResults = compiled.key === compileKey && !compiled.pending ? compiled.results : [];
  const compilePending = compileDefinitions.length > 0 && (compiled.key !== compileKey || compiled.pending);
  const compileError = compiled.key === compileKey ? compiled.error : undefined;
  const compiledDefinitions = useMemo(() => definitions
    .map((item) => libraryPreviewFor(item, currentResults))
    .filter((item): item is CompiledFootprint => Boolean(item)), [definitions, currentResults]);

  useEffect(() => {
    if (!compileDefinitions.length) return;
    const snapshot = compileDefinitions;
    setCompiled({ key: compileKey, results: [], pending: true });
    return runLatest(sequence, () => compileFootprints(snapshot.map((item) => ({ id: `library-preview:${item.id}`, definition: item, side: 'front' }))), (results) => {
      setCompiled(results.length === snapshot.length
        ? { key: compileKey, results, pending: false }
        : { key: compileKey, results: [], error: 'Rust returned an incomplete footprint preview batch.', pending: false });
    }, (cause) => {
      setCompiled({ key: compileKey, results: [], error: cause instanceof Error ? cause.message : String(cause), pending: false });
    });
  }, [compileFootprints, compileKey]);

  useEffect(() => {
    const defaults: Record<string, JsonValue> = {};
    for (const [key, parameter] of Object.entries(parameterSchema)) {
      if (parameter.value !== undefined) defaults[key] = parameter.value as JsonValue;
    }
    Object.assign(defaults, selectedDefinition?.generator?.parameters ?? {});
    setEdits(defaults);
  }, [selectedDefinition?.id, selectedDefinition?.generator?.parameters, parameterSchema]);

  const updateGenerator = (parameter: string, value: JsonValue) => setEdits((current) => ({ ...current, [parameter]: value }));
  const saveGenerator = () => {
    if (!selectedDefinition || !isErgogen(selectedDefinition.generator?.source) || !definition?.generator || !isErgogen(definition.generator.source) || generatorPreview?.error) return;
    const next = normalizeDefinition(definition);
    const original = document.definitions.find((item) => item.id === next.id) ?? selectedDefinition;
    const result = remapDefinitionNets(document, original, next);
    if (!result.ok) {
      setDefinitionError(result.error);
      return;
    }
    const instances = document.parts.filter((part) => part.definitionId === next.id);
    const definitions = document.definitions.some((item) => item.id === next.id)
      ? document.definitions.map((item) => item.id === next.id ? next : item)
      : [...document.definitions, next];
    setDefinitionError('');
    onEdit({ baseRevision: document.revision, transactionId: makeId(), phase: 'commit', targetIds: [next.id, ...instances.map((part) => part.id)], operation: { kind: 'replace-document', document: { ...document, definitions, nets: result.nets } } });
  };

  return { edits, updateGenerator, generatorPreview, previewDefinition: definition, compiledDefinitions, compilePending, compileError, saveGenerator };
}
