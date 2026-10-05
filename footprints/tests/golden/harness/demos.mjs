// Builds every bundled demo project with the production demo builders and the
// native Core driver, then returns the documents. Core runs as one long-lived
// process because `open` and `edit` requests share state.
import { spawn, spawnSync } from 'node:child_process';
import { createInterface } from 'node:readline';
import { readFileSync } from 'node:fs';

export function startCore(driver) {
  const child = spawn(driver, [], { stdio: ['pipe', 'pipe', 'inherit'] });
  const lines = createInterface({ input: child.stdout })[Symbol.asyncIterator]();
  let chain = Promise.resolve();
  const request = (input) => (chain = chain.then(async () => {
    child.stdin.write(`${JSON.stringify(input)}\n`);
    const { value, done } = await lines.next();
    if (done) throw new Error('Core driver exited');
    return JSON.parse(value);
  }));
  return { request, stop: () => child.kill() };
}

export function readArchiveProject(path) {
  const python = 'import sys,zipfile;sys.stdout.write(zipfile.ZipFile(sys.argv[1]).read("project.json").decode())';
  const result = spawnSync('python3', ['-c', python, path], { encoding: 'utf8', maxBuffer: 1 << 28 });
  if (result.status !== 0) throw new Error(result.stderr);
  return JSON.parse(result.stdout);
}

export async function buildDemoDocuments(root, driver) {
  const demoRoot = `${root}tooling/demo-projects/src/`;
  const core = startCore(driver);
  let counter = 0;
  const request = async (input) => {
    if (input.kind === 'open') input.document.id = `golden-${counter++}`;
    const reply = await core.request(input);
    if (reply.kind === 'error') throw new Error(reply.message);
    return reply;
  };
  const documents = [];
  try {
    const { demoProject } = await import(`${demoRoot}demo.ts`);
    documents.push({ project: 'starter', document: demoProject() });
    const { openSofleDemo, sofleDemos } = await import(`${demoRoot}demos/sofle.ts`);
    for (const { id } of sofleDemos) documents.push({ project: `sofle-${id}`, document: (await openSofleDemo(id, request)).document });
    const { keyboardDemos, openKeyboardDemo } = await import(`${demoRoot}demos/keyboards.ts`);
    for (const { id } of keyboardDemos) documents.push({ project: `measured-${id}`, document: (await openKeyboardDemo(id, request)).document });
    const { openModuleReviewDemo } = await import(`${demoRoot}demos/moduleReview.ts`);
    documents.push({ project: 'vik-module-review', document: (await openModuleReviewDemo(request)).document });
    documents.push({ project: 'reviung41', document: readArchiveProject(`${root}content/archives/reviung41-original.boardstudio`) });
  } finally {
    core.stop();
  }
  return documents;
}
