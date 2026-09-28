import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';
import { emptyProject, type MechanicalAssembly } from '@boardstudio/v2-contracts';
import type { GenerationState } from '../generationState';
import { defaultInternalGasket } from '../gasketEditing';
import { createMechanicalConfiguration } from '../mechanicalPresets';
import { MechanicalAssemblyPanel } from './MechanicalAssemblyPanel';

const document = emptyProject('case-controls', 'Case controls');
document.revision = 3;
document.boards = [{ id: 'board', name: 'Main board', partIds: [], netIds: [], outlineIds: [], thickness: 1.6 }];
const configuration = createMechanicalConfiguration(document, 'board');
const assembly: MechanicalAssembly = {
  revision: 3, case: { revision: 3, bodies: [] }, stack: [], diagnostics: [],
  generationBlocked: false, gasketSupports: [], gasketTracks: [], generatedHardware: [],
  suggestedMounts: [], nominalPlateContours: [], plateContours: [],
};

function render(generation: GenerationState, resolved?: MechanicalAssembly) {
  return renderToStaticMarkup(<MechanicalAssemblyPanel document={document} definitions={[]} configuration={configuration}
    assembly={resolved} generation={generation} onChange={() => {}} onResolve={() => {}} onCancel={() => {}} onExport={() => {}} />);
}

function button(markup: string, label: string) {
  return markup.match(new RegExp(`<button[^>]*>${label}</button>`))?.[0] ?? '';
}

describe('mechanical generation controls', () => {
  it('exposes determinate progress at zero completed bodies', () => {
    const markup = render({ status: 'running', revision: 3, progress: { revision: 3, stage: 'building', completed: 0, total: 4, body: 'plate' } });
    expect(markup.match(/<progress[^>]*>/)?.[0]).toContain('value="0"');
    expect(button(markup, 'Update preview')).toContain('disabled');
    expect(button(markup, 'Cancel')).not.toContain('disabled');
  });

  it('emphasizes Update preview until current geometry can be exported', () => {
    const markup = render({ status: 'required' }, assembly);
    expect(button(markup, 'Update preview')).toContain('wb-primary');
    expect(button(markup, 'Export geometry')).toContain('disabled');
    expect(button(markup, 'Export geometry')).not.toContain('wb-primary');
  });

  it('only offers export for current, successfully generated geometry', () => {
    const ready = render({ status: 'ready', revision: 3 }, assembly);
    expect(button(ready, 'Export geometry')).toContain('wb-primary');
    expect(button(ready, 'Export geometry')).not.toContain('disabled');
    const stale = render({ status: 'ready', revision: 2 }, { ...assembly, revision: 2 });
    expect(button(stale, 'Export geometry')).toContain('disabled');
    for (const status of ['blocked', 'failed', 'cancelled'] as const) {
      expect(button(render({ status }, assembly), 'Export geometry')).toContain('disabled');
    }
  });
});

describe('export readiness regression', () => {
  it('rejects a ready generation from an older revision', () => {
    expect(button(render({ status: 'ready', revision: 2 }, assembly), 'Export geometry')).toContain('disabled');
  });

  it('blocks mechanical errors while allowing warnings', () => {
    const finding = { id: 'fit', scope: 'case' as const, targetIds: [], message: 'Review fit', severity: 'error' as const };
    expect(button(render({ status: 'ready', revision: 3 }, { ...assembly, diagnostics: [finding] }), 'Export geometry')).toContain('disabled');
    expect(button(render({ status: 'ready', revision: 3 }, { ...assembly, diagnostics: [{ ...finding, severity: 'warning' }] }), 'Export geometry')).not.toContain('disabled');
  });
});

it('exposes internal gasket sizes, clearance, closure hardware and top-case layers', () => {
  const gasket = { ...configuration, mount: 'gasket' as const, internalGasket: defaultInternalGasket() };
  const resolved = { ...assembly, stack: [{ id: 'retainer', z: 8, thickness: 6 }] };
  const markup = renderToStaticMarkup(<MechanicalAssemblyPanel document={document} definitions={[]} configuration={gasket}
    assembly={resolved} onChange={() => {}} />);
  expect(markup).toContain('Gasket size');
  expect(markup).toContain('80 × 4 × 5 mm');
  expect(markup).toContain('PCB-to-support clearance');
  expect(markup).toContain('Minimum wall behind gasket pockets');
  expect(markup).toContain('Top case');
  expect(markup).toContain('Screw drive');
  expect(markup).not.toContain('captive nuts');
});
