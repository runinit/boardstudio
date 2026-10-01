import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, test } from 'vitest';

type ManifestEntry = {
  row: string;
  variant: string;
  file: string;
  sha256: string;
};

type ModelAsset = {
  path: string;
  assetId: string;
  catalogRow: string;
  bundledFile: string;
  sha256: string;
  nativeBoundsMm: {
    XMin: number;
    XMax: number;
    YMin: number;
    YMax: number;
    ZMin: number;
    ZMax: number;
    XLength: number;
    YLength: number;
    ZLength: number;
  };
  appliesToVariants?: string[];
};

type ModuleEntry = {
  row: string;
  definition: {
    id: string;
    variant: string;
    source: { sha256: string };
    board: { contours: unknown[] };
    gates: { output: string; code: string }[];
    interfaces: { id: string; role: string; signals: string[] }[];
    constituents: { reference: string }[];
    candidateModels: {
      assetId: string;
      boundsMin: { x: number; y: number; z: number };
      boundsMax: { x: number; y: number; z: number };
      source: { repository: string; revision: string; path: string; license: string; sha256: string };
      verifiedAlignment: boolean;
    }[];
    models?: { assetId: string; offset: { x: number; y: number; z: number }; rotation: { x: number; y: number; z: number }; scale: { x: number; y: number; z: number } }[];
    volumes?: { purpose: string; qualified: boolean; geometry: { points: { x: number; y: number }[]; z: number; height: number } }[];
    openings?: { purpose: string; qualified: boolean; source: string; geometry: { points: { x: number; y: number }[]; z: number; height: number } }[];
  };
};

const directory = dirname(fileURLToPath(import.meta.url));
const manifest = JSON.parse(readFileSync(resolve(directory, 'import-manifest.json'), 'utf8')) as { entries: ManifestEntry[] };
const ledger = JSON.parse(readFileSync(resolve(directory, 'asset-ledger.json'), 'utf8')) as { models: ModelAsset[] };
const catalogue = JSON.parse(readFileSync(resolve(directory, 'imported-modules.json'), 'utf8')) as { modules: ModuleEntry[] };
const sha256 = (bytes: Buffer) => createHash('sha256').update(bytes).digest('hex');
const moduleSignals = ['sclk', 'miso', 'cs', 'gpio2', 'mosi', 'gpio1', 'v5', 'rgb', 'scl', 'sda', 'gnd', 'v3v3'];

describe('pinned VIK module catalogue', () => {
  test('covers every source row and variant with matching imported provenance', () => {
    expect(new Set(manifest.entries.map(entry => entry.row)).size).toBe(29);
    expect(manifest.entries).toHaveLength(37);
    expect(catalogue.modules).toHaveLength(38); // DRV2605L's explicitly repaired pullup option.
    expect(new Set(catalogue.modules.map(entry => entry.definition.id)).size).toBe(catalogue.modules.length);

    for (const entry of manifest.entries) {
      const sourceBytes = readFileSync(resolve(directory, entry.file));
      expect(sha256(sourceBytes), `${entry.row} / ${entry.variant} source`).toBe(entry.sha256);
      const imported = catalogue.modules.filter(item => item.row === entry.row && item.definition.variant === entry.variant);
      expect(imported, `${entry.row} / ${entry.variant} snapshot`).toHaveLength(1);
      expect(imported[0]!.definition.source.sha256).toBe(entry.sha256);
      expect(imported[0]!.definition.gates).toContainEqual(expect.objectContaining({ output: 'mechanical', code: 'assembled-envelope' }));
      expect(imported[0]!.definition.gates).toContainEqual(expect.objectContaining({ output: 'model', code: 'assembly-model' }));
      const definition = imported[0]!.definition;
      expect(definition.interfaces.length, `${entry.row} / ${entry.variant} source VIK role`).toBeGreaterThan(0);
      for (const interfaceProfile of definition.interfaces) {
        expect(interfaceProfile.signals, `${entry.row} / ${interfaceProfile.id} VIK signal ordering`)
          .toEqual(interfaceProfile.role === 'host' ? [...moduleSignals].reverse() : moduleSignals);
        expect(definition.constituents.some(part => part.reference === interfaceProfile.id),
          `${entry.row} / ${interfaceProfile.id} source connector`).toBe(true);
      }
    }
  });

  test('keeps source model bounds and applicability attached to the matching variant', () => {
    for (const asset of ledger.models) {
      const bounds = asset.nativeBoundsMm;
      const dimensions = [
        [bounds.XMin, bounds.XMax, bounds.XLength],
        [bounds.YMin, bounds.YMax, bounds.YLength],
        [bounds.ZMin, bounds.ZMax, bounds.ZLength],
      ];
      for (const [minimum, maximum, length] of dimensions) {
        expect(Number.isFinite(minimum) && Number.isFinite(maximum) && Number.isFinite(length)).toBe(true);
        expect(maximum).toBeGreaterThan(minimum);
        expect(length).toBeCloseTo(maximum - minimum, 5);
      }
      const modelBytes = readFileSync(resolve(directory, '../../..', asset.bundledFile));
      expect(sha256(modelBytes), asset.bundledFile).toBe(asset.sha256);

      const applicable = catalogue.modules.filter(item => (asset.catalogRow === 'shared connector' || item.row === asset.catalogRow)
        && (!asset.appliesToVariants || asset.appliesToVariants.includes(item.definition.variant)));
      expect(applicable.length, `${asset.assetId} belongs to a catalog row/variant`).toBeGreaterThan(0);
      for (const item of applicable) {
        const candidate = item.definition.candidateModels.find(model => model.assetId === asset.assetId);
        expect(candidate, `${item.definition.variant} should offer ${asset.assetId}`).toBeDefined();
        expect(candidate!.verifiedAlignment).toBe(false);
        expect(candidate!.source.sha256).toBe(asset.sha256);
        expect(candidate!.boundsMin).toEqual({ x: bounds.XMin, y: bounds.YMin, z: bounds.ZMin });
        expect(candidate!.boundsMax).toEqual({ x: bounds.XMax, y: bounds.YMax, z: bounds.ZMax });
      }
      for (const item of catalogue.modules.filter(module => (asset.catalogRow === 'shared connector' || module.row === asset.catalogRow)
        && !applicable.includes(module))) {
        expect(item.definition.candidateModels.some(model => model.assetId === asset.assetId),
          `${item.definition.variant} must not offer incompatible candidate ${asset.assetId}`).toBe(false);
      }
    }
  });

  test('does not invent complete boards when pinned source outlines are missing', () => {
    const expectedMissing = new Map([
      ['solenoid-2', 'VIK-Solenoid'],
      ['peacock trackpad', 'trackpad'],
    ]);
    for (const [row, variant] of expectedMissing) {
      const definition = catalogue.modules.find(item => item.row === row && item.definition.variant === variant)?.definition;
      expect(definition, `${row} / ${variant}`).toBeDefined();
      expect(definition!.board.contours).toHaveLength(0);
      expect(definition!.gates.some(gate => gate.output === 'mechanical'
        && ['board-outline', 'board-outline-missing'].includes(gate.code))).toBe(true);
    }
  });

  test('retains the Kiwano standard VIK input connector as a module role', () => {
    const kiwano = catalogue.modules.find(item => item.row === 'per56-pmw3360-cirque-leds (Kiwano)')?.definition;
    expect(kiwano?.interfaces.find(item => item.id === 'J1'))
      .toEqual({ id: 'J1', role: 'module', signals: moduleSignals });
  });

  test('seeds one source-datum-aligned display assembly with nominal, unqualified envelope and aperture', () => {
    const display = catalogue.modules.find(item => item.row === 'vik-display-adapter'
      && item.definition.variant === 'pcb/1.47inch/pcb')?.definition;
    const asset = ledger.models.find(model => model.path === 'pcb/1.47inch/1.47inch.step') as ModelAsset & {
      boardToModelTransform?: { offset: { x: number; y: number; z: number }; rotation: { x: number; y: number; z: number }; scale: { x: number; y: number; z: number }; evidence: { asymmetricFeatures: string } };
      nominalProfile?: { viewingAperture: { points: { x: number; y: number }[] }; viewingApertureSource: string };
    };
    expect(display?.models).toEqual([{ assetId: asset.assetId,
      offset: asset.boardToModelTransform?.offset,
      rotation: asset.boardToModelTransform?.rotation,
      scale: asset.boardToModelTransform?.scale }]);
    expect(asset.boardToModelTransform?.evidence.asymmetricFeatures).toContain('H1/H2');
    expect(display?.volumes).toHaveLength(1);
    expect(display?.volumes?.[0]).toMatchObject({ purpose: 'nominal occupied', qualified: false });
    expect(display?.volumes?.[0]?.geometry).toMatchObject({ z: -2.96, height: 5.17 });
    expect(display?.volumes?.[0]?.geometry.points).toEqual([
      { x: -11, y: -20.38 }, { x: 11, y: -20.38 }, { x: 11, y: 19.5 }, { x: -11, y: 19.5 },
    ]);
    expect(display?.openings).toHaveLength(1);
    expect(display?.openings?.[0]).toMatchObject({ purpose: 'nominal viewing aperture', qualified: false });
    expect(display?.openings?.[0]?.source).toContain('17.39 × 32.35 mm');
    expect(display?.openings?.[0]?.geometry.points).toHaveLength(4);
    expect(display?.openings?.[0]?.geometry.points).toEqual([
      { x: -8.75, y: -17.065 }, { x: 8.64, y: -17.065 }, { x: 8.64, y: 15.285 }, { x: -8.75, y: 15.285 },
    ]);
    expect(display?.gates.some(gate => gate.code === 'nominal-display-assumptions')).toBe(true);
    expect(display?.gates.some(gate => gate.code === 'assembled-envelope')).toBe(true);
    expect(display?.gates.some(gate => gate.code === 'assembly-model')).toBe(true);
  });
});
