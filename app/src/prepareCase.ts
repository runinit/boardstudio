import type { CaseAssemblyIR, PreparedCaseAssemblyIR } from '@boardstudio/v2-contracts';
import { CoreClient } from './CoreClient';
import { cadProfilingEnabled, recordCadMeasure } from './cadPerformance';

export async function prepareCase(core: CoreClient, ir: CaseAssemblyIR): Promise<PreparedCaseAssemblyIR> {
  const start = cadProfilingEnabled() ? performance.now() : undefined;
  let succeeded = false;
  try {
    const reply = await core.request({ id: crypto.randomUUID(), kind: 'prepare-case', ir });
    if (reply.kind === 'error') throw new Error(reply.message);
    if (reply.kind !== 'case-prepared') throw new Error('Unexpected core worker response for case preparation');
    if (reply.ir.revision !== ir.revision) throw new Error('Core prepared a different case revision');
    succeeded = true;
    return reply.ir;
  } finally {
    if (start !== undefined) recordCadMeasure('boardstudio.cad.core-preparation', {
      start, end: performance.now(), detail: { revision: ir.revision, succeeded },
    });
  }
}
