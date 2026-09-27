import { partChoices, partCatalogLabel } from './partsCatalog';
import React, { lazy, useMemo, useRef, useState } from 'react';
import type {
  AssemblyDefinition,
  AssemblyMember,
  PartDefinition,
  PartModel,
  ProjectDoc,
} from '@boardstudio/v2-contracts';
import { isErgogen, modelBindings } from '@boardstudio/v2-ergogen';
import { bundledModels } from '../bundledModels';
import { sampleAssembly } from './sampleAssembly';
import { placeAssembly } from './assemblyPlacement';
import { storePreviewAsset } from './BoardReferencePanel';
import { attachAssemblyAsset, saveAssembly, updateAssemblyMember, updateAssemblyModel } from './assemblyEditorController';
const AssemblyViewer = lazy(() =>
  import('./AssemblyViewer').then((m) => ({ default: m.AssemblyViewer })),
);
const zero = () => ({ x: 0, y: 0, z: 0 });

export function AssemblyEditor({
  document,
  initial,
  definitions,
  boardId,
  colorScheme,
  onChange,
  onClose,
  onPlace,
  onApply,
  matrixName,
}: {
  onApply?: (assembly: AssemblyDefinition) => void;
  matrixName?: string;
  document: ProjectDoc;
  initial: AssemblyDefinition;
  definitions: PartDefinition[];
  boardId?: string;
  colorScheme: 'light' | 'dark';
  onChange: (document: ProjectDoc) => void;
  onClose: () => void;
  onPlace: (document: ProjectDoc) => void;
}) {
  const [draft, setDraft] = useState(() => structuredClone(initial)),
    [error, setError] = useState(''),
    [saved, setSaved] = useState(false);
  const latestDocument = useRef(document);
  latestDocument.current = document;
  const [origin, setOrigin] = useState({ x: 0, y: 0 });
  const modelOptions = [
    ...document.assets
      .filter((a) => /\.(step|stp|stl|wrl)$/i.test(a.name))
      .map((a) => ({ id: a.id, name: a.name })),
    ...bundledModels().map((m) => ({ id: m.id, name: m.filename })),
  ];
  const change = (value: AssemblyDefinition) => {
    setDraft(value);
    setSaved(false);
  };
  const update = (id: string, patch: Partial<AssemblyMember>) =>
    change(updateAssemblyMember(draft, id, patch));
  const prepared = useMemo(() => {
    try {
      const first = definitions.find(
        (d) => d.id === draft.members[0]?.definitionId,
      ) ?? {
        id: 'visual',
        name: 'Visual model',
        kind: 'custom' as const,
        pads: [],
        courtyard: [],
      };
      const base = sampleAssembly(document, first);
      base.project.parts = [];
      base.project.definitions = [];
      base.project.boards[0].partIds = [];
      const project = placeAssembly(
        base.project,
        draft,
        definitions,
        'sample-board',
        { x: 0, y: 0 },
        'sample',
      );
      const points = project.parts.flatMap((p) => {
        const d = project.definitions.find((d) => d.id === p.definitionId)!;
        return (
          d.courtyard.length
            ? d.courtyard
            : [
                { x: -10, y: -10 },
                { x: 10, y: 10 },
              ]
        ).map((v) => ({ x: v.x + p.pose.at.x, y: v.y + p.pose.at.y }));
      });
      const minX = Math.min(-10, ...points.map((p) => p.x)) - 3,
        maxX = Math.max(10, ...points.map((p) => p.x)) + 3,
        minY = Math.min(-10, ...points.map((p) => p.y)) - 3,
        maxY = Math.max(10, ...points.map((p) => p.y)) + 3;
      return {
        project,
        contours: [
          {
            hole: false,
            points: [
              { x: minX, y: minY },
              { x: maxX, y: minY },
              { x: maxX, y: maxY },
              { x: minX, y: maxY },
            ],
          },
        ],
        error: '',
      };
    } catch (cause) {
      return { error: String(cause) };
    }
  }, [document, draft, definitions]);
  const save = () => {
    const next = saveAssembly(document, draft, definitions);
    if (typeof next === 'string') {
      setError(next);
      return;
    }
    onChange(next);
    setSaved(true);
  };
  const attach = async (member: AssemblyMember, file: File) => {
    try {
      const { asset, document: nextDocument } = await attachAssemblyAsset(
        file,
        () => latestDocument.current,
        document,
        storePreviewAsset,
      );
      onChange(nextDocument);
      update(member.id, {
        modelMode: 'custom',
        models: [
          ...member.models,
          {
            assetId: asset.id,
            offset: zero(),
            rotation: zero(),
            scale: { x: 1, y: 1, z: 1 },
          },
        ],
      });
    } catch (cause) {
      setError(String(cause));
    }
  };
  const modelEdit = (
    member: AssemblyMember,
    index: number,
    patch: Partial<PartModel>,
  ) =>
    change(updateAssemblyModel(draft, member, index, patch));
  return (
    <div className="wb-assembly-editor-layout">
      <div className="wb-assembly-editor">
        <h2>Assembly editor</h2>
        <label>
          Name
          <input
            value={draft.name}
            onChange={(e) => change({ ...draft, name: e.target.value })}
          />
        </label>
        <button onClick={save}>Save assembly</button>{' '}
        <button onClick={onClose}>Close editor</button>
        {saved && (
          <p role="status">
            Assembly saved. Existing placements are unchanged.
          </p>
        )}
        {draft.members.map((member) => {
          const definition = definitions.find(
            (d) => d.id === member.definitionId,
          );
          return (
            <fieldset key={member.id}>
              <legend>{member.id}</legend>
              <label>
                Component
                <select
                  value={member.definitionId ?? ''}
                  onChange={(e) =>
                    update(member.id, {
                      definitionId: e.target.value || undefined,
                      models: [],
                      parameters: undefined,
                      modelMode: 'defaults',
                    })
                  }
                >
                  <option value="">Visual model only</option>
                  {partChoices(definitions, member.definitionId)
                    .filter((d) => d.kind !== 'utility')
                    .map((d) => (
                      <option key={d.id} value={d.id}>
                        {partCatalogLabel(d)}
                      </option>
                    ))}
                </select>
              </label>
              {definition?.generator && member.parameters && (
                <label>
                  Hotswap socket
                  <input
                    type="checkbox"
                    checked={Boolean(member.parameters.hotswap)}
                    onChange={(e) =>
                      update(member.id, {
                        parameters: {
                          ...member.parameters,
                          hotswap: e.target.checked,
                          solder: !e.target.checked,
                        },
                      })
                    }
                  />
                </label>
              )}
              <label>
                {member.parameters?.side ? 'Switch side' : 'Side'}
                <select
                  value={member.side}
                  onChange={(e) =>
                    update(member.id, {
                      side: e.target.value as 'front' | 'back',
                      ...(member.parameters?.side ? {parameters: {...member.parameters, side: e.target.value === 'front' ? 'B' : 'F'}} : {}),
                    })
                  }
                >
                  <option value="front">Front</option>
                  <option value="back">Back</option>
                </select>
              </label>
              <div className="wb-transform-fields">
                {(['x', 'y'] as const).map((axis) => (
                  <label key={axis}>
                    {axis.toUpperCase()} (mm)
                    <input
                      type="number"
                      step=".1"
                      value={member.pose.at[axis]}
                      onChange={(e) => {
                        if (e.target.value)
                          update(member.id, {
                            pose: {
                              ...member.pose,
                              at: {
                                ...member.pose.at,
                                [axis]: Number(e.target.value),
                              },
                            },
                          });
                      }}
                    />
                  </label>
                ))}
                <label>
                  Angle (°)
                  <input
                    type="number"
                    value={member.pose.rotation}
                    onChange={(e) => {
                      if (e.target.value)
                        update(member.id, {
                          pose: {
                            ...member.pose,
                            rotation: Number(e.target.value),
                          },
                        });
                    }}
                  />
                </label>
              </div>
              {!member.models.length && (
                <p>
                  {member.modelMode === 'custom'
                    ? 'No models in this member.'
                    : 'Uses component model defaults.'}
                </p>
              )}
              {definition && (
                <button
                  onClick={() => {
                    try {
                      update(member.id, {
                        modelMode: 'custom',
                        models: isErgogen(definition.generator?.source)
                          ? modelBindings(
                              {
                                ...definition,
                                generator: definition.generator
                                  ? {
                                      ...definition.generator,
                                      parameters: {
                                        ...definition.generator.parameters,
                                        ...member.parameters,
                                      },
                                    }
                                  : undefined,
                              },
                              {
                                id: member.id,
                                reference: member.id,
                                definitionId: definition.id,
                                side: member.side,
                                pose: { at: { x: 0, y: 0 }, rotation: 0 },
                              },
                            )
                          : [
                              ...(definition.models ?? []),
                            ],
                      });
                    } catch (cause) {
                      setError(String(cause));
                    }
                  }}
                >
                  Edit model defaults
                </button>
              )}
              {member.models.map((model, index) => (
                <details key={index} open>
                  <summary>Model {index + 1}</summary>
                  <label>
                    Asset
                    <select
                      value={model.assetId}
                      onChange={(e) =>
                        modelEdit(member, index, { assetId: e.target.value })
                      }
                    >
                      {modelOptions.map((m) => (
                        <option key={m.id} value={m.id}>
                          {m.name}
                        </option>
                      ))}
                    </select>
                  </label>
                  {(['offset', 'rotation', 'scale'] as const).map((field) => (
                    <div key={field}>
                      <strong>
                        {field}{' '}
                        {field === 'offset'
                          ? '(mm)'
                          : field === 'rotation'
                            ? '(°)'
                            : ''}
                      </strong>
                      <div className="wb-transform-fields">
                        {(['x', 'y', 'z'] as const).map((axis) => (
                          <label key={axis}>
                            {axis.toUpperCase()}
                            <input
                              type="number"
                              step=".1"
                              value={model[field][axis]}
                              onChange={(e) => {
                                const value = Number(e.target.value);
                                if (
                                  e.target.value &&
                                  Number.isFinite(value) &&
                                  (field !== 'scale' || value > 0)
                                )
                                  modelEdit(member, index, {
                                    [field]: { ...model[field], [axis]: value },
                                  });
                              }}
                            />
                          </label>
                        ))}
                      </div>
                    </div>
                  ))}
                  <button
                    onClick={() =>
                      update(member.id, {
                        modelMode: 'custom',
                        models: member.models.filter((_, i) => i !== index),
                      })
                    }
                  >
                    Remove model
                  </button>
                </details>
              ))}
              <button
                onClick={() => {
                  const asset = modelOptions[0];
                  if (asset)
                    update(member.id, {
                      modelMode: 'custom',
                      models: [
                        ...member.models,
                        {
                          assetId: asset.id,
                          offset: zero(),
                          rotation: zero(),
                          scale: { x: 1, y: 1, z: 1 },
                        },
                      ],
                    });
                }}
              >
                Add model
              </button>
              <label>
                Import model
                <input
                  type="file"
                  accept=".step,.stp,.stl,.wrl"
                  onChange={(e) => {
                    const file = e.target.files?.[0];
                    if (file) void attach(member, file);
                    e.target.value = '';
                  }}
                />
              </label>
              <button
                onClick={() =>
                  change({
                    ...draft,
                    members: draft.members.filter((m) => m.id !== member.id),
                  })
                }
              >
                Remove component
              </button>
            </fieldset>
          );
        })}
        <button
          onClick={() =>
            change({
              ...draft,
              members: [
                ...draft.members,
                {
                  id: `component-${crypto.randomUUID().slice(0, 8)}`,
                  pose: { at: { x: 0, y: 0 }, rotation: 0 },
                  side: 'front',
                  models: [],
                },
              ],
            })
          }
        >
          Add component
        </button>
        {onApply && (
          <button
            onClick={() => {
              try {
                onApply(draft);
              } catch (cause) {
                setError(String(cause));
              }
            }}
          >
            Apply to {matrixName ?? 'selected matrix'}
          </button>
        )}
        <fieldset>
          <legend>Place assembly</legend>
          <div className="wb-transform-fields">
            {(['x', 'y'] as const).map((axis) => (
              <label key={axis}>
                {axis.toUpperCase()} (mm)
                <input
                  type="number"
                  value={origin[axis]}
                  onChange={(e) =>
                    setOrigin({ ...origin, [axis]: Number(e.target.value) })
                  }
                />
              </label>
            ))}
          </div>
          <button
            disabled={!boardId || !draft.members.length}
            onClick={() => {
              try {
                if (boardId)
                  onPlace(
                    placeAssembly(
                      document,
                      draft,
                      definitions,
                      boardId,
                      origin,
                      crypto.randomUUID(),
                    ),
                  );
              } catch (cause) {
                setError(String(cause));
              }
            }}
          >
            Place on selected board
          </button>
        </fieldset>
        {(error || prepared.error) && (
          <p role="alert">{error || prepared.error}</p>
        )}
      </div>
      <div>
        {prepared.project && (
          <React.Suspense fallback={<p>Loading assembly…</p>}>
            <AssemblyViewer
              document={prepared.project}
              boardId="sample-board"
              contours={prepared.contours!}
              colorScheme={colorScheme}
              sample
            />
          </React.Suspense>
        )}
      </div>
    </div>
  );
}
