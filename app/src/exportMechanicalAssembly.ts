import type { CaseResult, Contour, ProjectDoc } from '@boardstudio/v2-contracts';
import type { CoreClient } from './CoreClient';
import type { CaseClient } from './CaseClient';
import type { ExportClient } from './ExportClient';
import { prepareCase } from './prepareCase';
import { criticalFitDrawing, mechanicalFabricationNotes } from './mechanicalFabricationNotes';

const bytes = (text: string): Uint8Array => new TextEncoder().encode(text);

/** Triangle soup from CAD is already placed in the document's millimetre coordinate frame. */
export function mechanicalStl(result: CaseResult): Uint8Array {
  const positions = result.mesh.positions;
  if (positions.length % 9 !== 0) throw new Error('CAD returned incomplete triangles');
  const output = new Uint8Array(84 + positions.length / 9 * 50);
  output.set(bytes('Board Studio mechanical part; coordinates in millimetres'));
  const view = new DataView(output.buffer);
  view.setUint32(80, positions.length / 9, true);
  for (let i = 0; i < positions.length / 9; i++) {
    const offset = 84 + i * 50;
    for (let j = 0; j < 3; j++) view.setFloat32(offset + j * 4, result.mesh.normals[i * 9 + j] ?? 0, true);
    for (let j = 0; j < 9; j++) view.setFloat32(offset + 12 + j * 4, positions[i * 9 + j], true);
  }
  return output;
}

export async function exportMechanicalAssembly(input: {
  document: ProjectDoc;
  contours: Contour[];
  outlineReady: boolean;
  core: CoreClient;
  cad: CaseClient;
  exporter: ExportClient;
  isCurrent: () => boolean;
}): Promise<Record<string, Uint8Array>> {
  const { document, contours, core, cad, exporter, isCurrent } = input;
  if (!input.outlineReady) throw new Error('Resolve active outline findings before exporting plate or case artifacts');
  const revision = document.revision;
  const check = (actual = revision): void => {
    if (!isCurrent() || actual !== revision) throw new Error('Mechanical export became stale; export again');
  };
  check();
  const resolved = await core.request({ id: crypto.randomUUID(), kind: 'resolve-mechanical', document, contours });
  check();
  if (resolved.kind === 'error') throw new Error(resolved.message);
  if (resolved.kind !== 'mechanical-resolved') throw new Error('Expected resolved mechanical assembly');
  const assembly = resolved.assembly;
  check(assembly.revision);
  const errors = assembly.diagnostics.filter(finding => finding.severity === 'error');
  if (errors.length) throw new Error(errors.map(finding => finding.message).join('\n'));
  const files: Record<string, Uint8Array> = {};
  const plateMethod = document.mechanical?.partProcesses?.find(part => part.partId === 'plate')?.method
    ?? document.mechanical?.method;
  if (plateMethod === 'pcb-fr4') {
    const plate = await exporter.artifact({ kind: 'export-mechanical-plate', document, contours });
    check();
    if (plate.kind === 'error') throw new Error(plate.error.message);
    if (plate.kind !== 'export-mechanical-plate') throw new Error('Expected mechanical plate export');
    check(plate.result.revision);
    for (const file of plate.result.files) files[`plate-kicad/${file.filename}`] = bytes(file.content);
  }
  const prepared = await prepareCase(core, assembly.case);
  check(prepared.revision);
  if (!document.mechanical) throw new Error('Missing mechanical configuration');
  // Include a nominal PCB in the assembly reference only. Its blank is not
  // a manufacturing deliverable and must never appear in the per-part outputs.
  const reference = await prepareCase(core, { revision, bodies: [assembly.pcbReference ?? {
    revision, contours,
    body: { id: 'pcb-reference', name: 'PCB reference — unpopulated', boardId: document.mechanical.boardId,
      kind: 'plate', thickness: document.mechanical.pcbThickness, clearance: 0,
      z: -document.mechanical.pcbThickness },
  }] });
  check(reference.revision);
  const assembled = await cad.request({ revision, bodies: [...prepared.bodies, ...reference.bodies] });
  check(assembled.revision);
  files['assembly.step'] = assembled.step;
  for (let index = 0; index < prepared.bodies.length; index++) {
    const body = prepared.bodies[index];
    const name = `${index + 1}-${body.body.name.replace(/[^a-zA-Z0-9_-]/g, '-')}`;
    const result = await cad.request({ revision, bodies: [body] });
    check(result.revision);
    files[`parts/${name}.step`] = result.step;
    files[`parts/${name}.stl`] = mechanicalStl(result);
    const source = assembly.case.bodies[index];
    // Resolved body contours carry profile cutouts; circular mounts are also
    // included in cut-sheet outlines, while KiCad represents them as NPTH drills.
    const outlineContours = [...source.contours, ...(source.body.mounts ?? []).map(mount => ({
      hole: true,
      points: Array.from({ length: 96 }, (_, point) => ({
        x: mount.at.x + mount.holeDiameter / 2 * Math.cos(point * 2 * Math.PI / 96),
        y: mount.at.y + mount.holeDiameter / 2 * Math.sin(point * 2 * Math.PI / 96),
      })),
    }))];
    for (const format of ['dxf', 'svg'] as const) {
      const outline = await exporter.artifact({ kind: 'export-outline', request: {
        filename: `${name}.${format}`, format, contours: outlineContours,
        board: { id: source.body.id, name: source.body.name, outlineIds: [], partIds: [], netIds: [], thickness: source.body.thickness, traces: [], vias: [] },
      } });
      check();
      if (outline.kind === 'error') throw new Error(outline.error.message);
      if (outline.kind !== 'export-outline') throw new Error('Expected mechanical outline export');
      files[`outlines/${outline.result.filename}`] = bytes(outline.result.content);
    }
  }
  if (!document.mechanical) throw new Error('Missing mechanical configuration');
  files['FABRICATION.md'] = bytes(mechanicalFabricationNotes(document.mechanical, assembly));
  files['critical-fit.svg'] = bytes(criticalFitDrawing(assembly, document.mechanical));
  files['assembly.json'] = bytes(JSON.stringify({ revision, pcbReference: 'Nominal unpopulated PCB only; component solids are not included', stack: assembly.stack, diagnostics: assembly.diagnostics }, null, 2));
  check();
  return files;
}
