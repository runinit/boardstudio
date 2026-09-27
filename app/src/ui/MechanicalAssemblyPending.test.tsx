// @vitest-environment jsdom
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { expect, it, vi } from 'vitest';
import { demoProject } from '../demo';
import { MechanicalAssemblyPanel } from './MechanicalAssemblyPanel';

Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });

it('publishes generated-case controls only after the configuration is committed', async () => {
  const host = document.createElement('div');
  const target = document.createElement('div');
  document.body.append(host, target);
  const root = createRoot(host);
  const project = demoProject();
  const change = vi.fn();
  const props = { document: project, boardId: project.boards[0].id, definitions: project.definitions, onChange: change, generationTarget: target };
  try {
    await act(async () => root.render(<MechanicalAssemblyPanel {...props} />));
    await act(async () => Array.from(host.querySelectorAll('button')).find(button => button.textContent === 'Configure mechanical stack')!.click());
    expect(change).toHaveBeenCalledOnce();
    // Authored-case controls still own the shared target while the edit is queued.
    expect(target.querySelectorAll('[aria-label="Case generation"]')).toHaveLength(0);
    const configuration = change.mock.calls[0][0];
    await act(async () => root.render(<MechanicalAssemblyPanel {...props} document={{ ...project, revision: project.revision + 1, mechanical: configuration }} configuration={configuration} />));
    expect(target.querySelectorAll('[aria-label="Case generation"]')).toHaveLength(1);
  } finally {
    await act(async () => root.unmount());
    host.remove(); target.remove();
  }
});

it('waits for automatic mounting defaults to commit before enabling Generate', async () => {
  const host = document.createElement('div');
  document.body.append(host);
  const root = createRoot(host);
  const project = demoProject();
  const { createMechanicalConfiguration } = await import('../mechanicalPresets');
  const configuration = createMechanicalConfiguration(project, project.boards[0].id);
  const assembly = {
    revision: project.revision, case: { revision: project.revision, bodies: [] }, stack: [], diagnostics: [],
    generationBlocked: false, gasketSupports: [], gasketTracks: [], generatedHardware: [],
    suggestedMounts: [{ id: 'mount', at: { x: 0, y: 0 }, kind: 'boss' as const, holeDiameter: 2.2 }], nominalPlateContours: [], plateContours: [],
  };
  const change = vi.fn();
  const generate = vi.fn();
  const props = { document: project, boardId: project.boards[0].id, definitions: project.definitions, configuration, assembly, onChange: change, onResolve: generate };
  const button = () => Array.from(host.querySelectorAll('button')).find(button => button.textContent === 'Generate')!;
  try {
    await act(async () => root.render(<MechanicalAssemblyPanel {...props} />));
    expect(change).toHaveBeenCalledOnce();
    expect(button().disabled).toBe(true);
    await act(async () => button().click());
    expect(generate).not.toHaveBeenCalled();
    const committed = { ...configuration, closureMounts: [] };
    await act(async () => root.render(<MechanicalAssemblyPanel {...props} configuration={committed} document={{ ...project, revision: project.revision + 1, mechanical: committed }} />));
    expect(button().disabled).toBe(false);
    await act(async () => button().click());
    expect(generate).toHaveBeenCalledOnce();
  } finally {
    await act(async () => root.unmount());
    host.remove();
  }
});
