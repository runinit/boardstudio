import { describe, expect, test } from 'vitest';
import { loadVikHostConnectorDefinition } from './hostConnector';

describe('VIK host connector source definition', () => {
  test('clones the pinned horizontal host footprint with bundled model and reversed-role metadata', async () => {
    const definition = await loadVikHostConnectorDefinition();
    expect(definition.id).toBe('vik:source:horizontal-host-connector');
    expect(definition.hardwareProfile?.vikRole).toBe('host');
    expect(definition.hardwareProfile?.source.revision).toMatch(/^[0-9a-f]{40}$/);
    expect(definition.hardwareProfile?.source.sha256).toMatch(/^[0-9a-f]{64}$/);
    expect(definition.pads.map(pad => pad.number).filter(number => /^(?:[1-9]|1[0-2])$/.test(number))).toHaveLength(12);
    expect(definition.models).toEqual([{
      assetId: 'ergogen:model:vik/sadekbaroudi-vik/kicad/3dmodels/vik-connector-horizontal.stp',
      offset: { x: -2.75, y: 2.3, z: 0 }, rotation: { x: 0, y: 0, z: 0 }, scale: { x: 1, y: 1, z: 1 },
    }]);
    expect(definition.kicadSource?.source).not.toContain('(model "../../kicad/3dmodels/vik-connector-horizontal.stp"');
    expect(definition.kicadSource?.source).toContain('(pad "1"');
    expect(definition.kicadSource?.source).toContain('(pad "12"');
  }, 30000);
});
