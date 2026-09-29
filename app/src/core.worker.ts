import type { CoreReply, CoreRequest } from '@boardstudio/v2-contracts';
import init, { CoreEngine } from '../../core/pkg/boardstudio_core.js';

let engine: CoreEngine | undefined;
let ready: Promise<unknown> | undefined;

async function handle(request: CoreRequest): Promise<CoreReply> {
  ready ??= init();
  await ready;
  engine ??= new CoreEngine();

  const started = request.diagnostics ? performance.now() : 0;
  const json = engine.request(JSON.stringify(request));
  const parsed = request.diagnostics ? performance.now() : 0;
  const reply = JSON.parse(json) as CoreReply;
  // Deserialization failures cannot recover the ID inside the core request.
  reply.id = request.id;

  if (request.diagnostics) {
    reply.timing = { wasmMs: parsed - started, parseMs: performance.now() - parsed };
  }

  return reply;
}

self.onmessage = async (event: MessageEvent<CoreRequest>) => {
  try {
    const reply = await handle(event.data);
    self.postMessage(reply);
  } catch (error) {
    const reply: CoreReply = {
      id: event.data.id,
      kind: 'error',
      message: error instanceof Error ? error.message : String(error),
      revision: 0,
    };

    self.postMessage(reply);
  }
};
