import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const here = dirname(fileURLToPath(import.meta.url));
const driver = resolve(here, '../../core/target/debug/examples/prepare_case');

export function prepareAssembly(ir) {
  const request = { id: 'cad-test', kind: 'prepare-case', ir };
  const output = execFileSync(driver, { input: `${JSON.stringify(request)}\n`, encoding: 'utf8' });
  const reply = JSON.parse(output);
  if (reply.kind !== 'case-prepared') throw new Error(reply.message ?? `Native preparation failed: ${reply.kind}`);
  return reply.ir;
}

export function prepareCase(ir) {
  const prepared = prepareAssembly({ revision: ir.revision, bodies: [ir] });
  return prepared.bodies[0];
}

export function resolveMechanical(document, contours) {
  const output = execFileSync(driver, { input: `${JSON.stringify({ id: 'mechanical-cad-test', kind: 'resolve-mechanical', document, contours })}\n`, encoding: 'utf8' });
  const reply = JSON.parse(output);
  if (reply.kind !== 'mechanical-resolved') throw new Error(reply.message ?? `Mechanical resolution failed: ${reply.kind}`);
  if (reply.assembly.diagnostics.some(finding => finding.severity === 'error')) throw new Error(JSON.stringify(reply.assembly.diagnostics));
  return reply.assembly;
}

export function resolveModules(document, boardId) {
  const output = execFileSync(driver, { input: `${JSON.stringify({ id: 'module-cad-test', kind: 'resolve-modules', document, boardId, previewTopZ: 0 })}\n`, encoding: 'utf8' });
  const reply = JSON.parse(output);
  if (reply.kind !== 'modules-resolved') throw new Error(reply.message ?? `Module resolution failed: ${reply.kind}`);
  return reply.result;
}
