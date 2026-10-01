import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';
import type { ResolvedModule } from '../../../contracts/src/index';
import { ModulePcbOverlay } from './ModulePcbOverlay';

const moduleScene: ResolvedModule = {
  id: 'module/review/above', definitionId: 'vik:splitter', at: { x: 12, y: 7 }, rotation: 0,
  midplaneZ: 2.4, flipped: false, board: [{ points: [{ x: 10, y: 5 }, { x: 14, y: 5 }, { x: 14, y: 9 }], z: 1, height: 0.8 }],
  boardHoles: [], volumes: [{ id: 'service/example', geometry: { points: [{ x: 9, y: 4 }, { x: 15, y: 4 }, { x: 15, y: 10 }], z: 1, height: 4 }, purpose: 'example service space', source: 'designer', qualified: true }], openings: [],
  mounts: [{ sourceId: 'PART0', at: { x: 10.5, y: 5.5 }, diameter: 2.2 }],
  mountSupports: [{ mountId: 'PART0', at: { x: 10.5, y: 5.5 }, outerDiameter: 4, holeDiameter: 2.4, z: -1, height: 2 }],
  footprints: [{ id: 'module/review/above/footprint/R1', sourcePartId: 'R1', reference: 'R1', definitionId: 'resistor', name: 'Resistor', pose: { at: { x: 12, y: 7 }, rotation: 0 }, side: 'front', courtyard: [{ x: -1, y: -1 }, { x: 1, y: -1 }, { x: 1, y: 1 }], pads: [{ id: '1', number: '1', at: { x: 0, y: 0 }, size: { x: 1, y: 1 }, shape: 'circle' }], surfaces: [
    { layer: 'B.SilkS', points: [{ x: 0, y: 0 }], width: 0.15, filled: false, text: 'R1', rotation: 0, textSize: 1 },
    { layer: 'F.Fab', points: [{ x: -1, y: -1 }, { x: 1, y: 1 }], width: 0.12, filled: false, text: '', rotation: 0, textSize: 0 },
  ] }], models: [], gates: [],
};

describe('mounted module PCB overlay', () => {
  it('renders ghost source geometry, source holes and configured standoffs without turning them into host parts', () => {
    const markup = renderToStaticMarkup(<ModulePcbOverlay module={moduleScene} hidden={new Set(['module-outlines', 'module-clearances', 'module-holes', 'module-standoffs', 'module-findings', 'module-fab-front', 'module-fab-back'])} hostHidden={new Set()} onSelect={() => undefined} />);
    expect(markup).toContain('class="wb-module-source-footprint is-front"');
    expect(markup).toContain('class="wb-module-footprint-courtyard"');
    expect(markup).toContain('class="wb-module-source-pad"');
    expect(markup).toContain('data-source-layer="B.SilkS"');
    expect(markup).not.toContain('data-source-layer="F.Fab"');
    expect(markup).not.toContain('wb-module-board-outline');
    expect(markup).not.toContain('wb-module-clearance');
    expect(markup).not.toContain('wb-module-mount-hole');
    expect(markup).not.toContain('wb-module-standoff');
  });

  it('uses the resolved host-side layer for artwork on a flipped module', () => {
    const flipped = { ...moduleScene, flipped: true, footprints: moduleScene.footprints?.map(footprint => ({ ...footprint, surfaces: footprint.surfaces?.map(surface => ({ ...surface, layer: surface.layer.startsWith('B.') ? `F.${surface.layer.slice(2)}` : surface.layer })) })) };
    const markup = renderToStaticMarkup(<ModulePcbOverlay module={flipped} hidden={new Set(['module-outlines', 'module-clearances', 'module-holes', 'module-standoffs', 'module-findings', 'module-fab-front', 'module-fab-back'])} hostHidden={new Set()} onSelect={() => undefined} />);
    expect(markup).toContain('data-source-layer="F.SilkS"');
  });

  it('keeps mounting holes and standoffs independently visible from board outlines', () => {
    const markup = renderToStaticMarkup(<ModulePcbOverlay module={moduleScene} hidden={new Set(['module-findings', 'module-fab-front', 'module-fab-back', 'module-silkscreen-front', 'module-silkscreen-back'])} hostHidden={new Set()} onSelect={() => undefined} />);
    expect(markup).toContain('class="wb-module-board-outline"');
    expect(markup).toContain('class="wb-module-clearance"');
    expect(markup).toContain('class="wb-module-mount-hole"');
    expect(markup).toContain('class="wb-module-standoff"');
    const withoutClearance = renderToStaticMarkup(<ModulePcbOverlay module={moduleScene} hidden={new Set(['module-clearances', 'module-findings', 'module-fab-front', 'module-fab-back', 'module-silkscreen-front', 'module-silkscreen-back'])} hostHidden={new Set()} onSelect={() => undefined} />);
    expect(withoutClearance).not.toContain('class="wb-module-clearance"');
    expect(withoutClearance).toContain('class="wb-module-mount-hole"');
    expect(withoutClearance).toContain('class="wb-module-standoff"');
  });

  it('applies host courtyard, pad, copper, reference and face-specific artwork toggles to the ghost footprint', () => {
    const markup = renderToStaticMarkup(<ModulePcbOverlay module={moduleScene} hidden={new Set(['module-outlines', 'module-clearances', 'module-holes', 'module-standoffs', 'module-findings'])} hostHidden={new Set(['Courtyards', 'Pads', 'F.Cu', 'References', 'B.SilkS'])} onSelect={() => undefined} />);
    expect(markup).not.toContain('class="wb-module-footprint-courtyard"');
    expect(markup).not.toContain('<rect');
    expect(markup).not.toContain('data-source-layer="B.SilkS"');
    expect(markup).toContain('data-source-layer="F.Fab"');
    expect(markup).not.toContain('wb-module-source-reference');
  });
});
