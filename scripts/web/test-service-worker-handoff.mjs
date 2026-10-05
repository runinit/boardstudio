import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import vm from "node:vm";
import test from "node:test";

const scripts = path.dirname(fileURLToPath(import.meta.url));
const run = (name, args) => spawnSync(process.execPath, [path.join(scripts, name), ...args], { encoding: "utf8" });

async function temporary(t) {
  const directory = await mkdtemp(path.join(tmpdir(), "boardstudio-worker-handoff-"));
  t.after(() => rm(directory, { recursive: true, force: true }));
  return directory;
}

async function assertHandoff(source, scope) {
  const handlers = new Map();
  const calls = [];
  let claimed = false;
  const inside = `${scope}?project=saved#layout`;
  const outside = "https://other.test/boardstudio/";
  const sibling = scope.endsWith("/boardstudio/") ? "https://example.test/boardstudio-other/" : outside;
  const client = (url) => ({ url, navigate: async (to) => { calls.push(["navigate", to]); return {}; } });
  const context = vm.createContext({
    URL,
    self: {
      registration: { scope },
      addEventListener: (type, handler) => handlers.set(type, handler),
      skipWaiting: async () => calls.push(["skipWaiting"]),
      clients: {
        claim: async () => { calls.push(["claim"]); claimed = true; },
        matchAll: async (options) => {
          assert.ok(claimed, "claim must finish before controlled clients are enumerated");
          assert.equal(options.type, "window");
          assert.equal(options.includeUncontrolled ?? false, false);
          return [client(inside), client(outside), client(sibling)];
        },
      },
    },
  });
  for (const name of ["caches", "indexedDB", "localStorage", "fetch"]) {
    Object.defineProperty(context, name, { get() { throw new Error(`handoff must not access ${name}`); } });
  }
  new vm.Script(source, { filename: "sw.js" }).runInContext(context);
  assert.deepEqual([...handlers.keys()], ["install", "activate"]);
  for (const type of ["install", "activate"]) {
    let work;
    handlers.get(type)({ waitUntil: (promise) => { work = promise; } });
    assert.ok(work, `${type} must retain its async work`);
    await work;
  }
  assert.deepEqual(calls, [["skipWaiting"], ["claim"], ["navigate", inside]]);
  // The same handoff must parse for both the old classic registration and the
  // candidate's module registration during a reverse deployment.
  const moduleCheck = spawnSync(process.execPath, ["--input-type=module", "--check"], { input: source, encoding: "utf8" });
  assert.equal(moduleCheck.status, 0, moduleCheck.stderr);
}

for (const prefix of ["/", "/boardstudio/"]) {
  test(`candidate packaging supplies a classic handoff for ${prefix}`, async (t) => {
    const directory = await temporary(t);
    const wasm = path.join(directory, "worker");
    const output = path.join(directory, "site", prefix.slice(1));
    await mkdir(wasm, { recursive: true });
    await writeFile(path.join(wasm, "boardstudio_offline_worker_bg.wasm"), Buffer.from([0, 97, 115, 109, 1, 0, 0, 0]));
    await writeFile(path.join(wasm, "boardstudio_offline_worker.js"), "export function initSync() {}\n");
    const manifest = path.join(directory, "manifest.json");
    await writeFile(manifest, JSON.stringify({ version: "handoff-test", assets: ["index.html", "sw.js", "service-worker.js", "boardstudio_offline_worker.js"] }));
    const embedded = run("embed-worker-wasm.mjs", [wasm, manifest, path.join(output, "service-worker.js")]);
    assert.equal(embedded.status, 0, embedded.stderr);
    const handoff = await readFile(path.join(output, "sw.js"), "utf8");
    await assertHandoff(handoff, `https://example.test${prefix}`);
    assert.match(await readFile(path.join(output, "service-worker.js"), "utf8"), /import \{ initSync \}/);
  });
}

test("rollback staging preserves reference bytes and adds only the reverse handoff", async (t) => {
  const directory = await temporary(t);
  const source = path.join(directory, "pinned-react");
  const destination = path.join(directory, "rollback");
  const files = {
    "index.html": '<base href="/boardstudio/">pinned shell',
    "sw.js": "/* pinned classic worker */",
    "assets/app.js": "/* pinned application */",
  };
  for (const [name, bytes] of Object.entries(files)) {
    await mkdir(path.dirname(path.join(source, name)), { recursive: true });
    await writeFile(path.join(source, name), bytes);
  }
  const staged = run("stage-rollback.mjs", [source, destination]);
  assert.equal(staged.status, 0, staged.stderr);
  const receipt = JSON.parse(await readFile(path.join(destination, "rollback-provenance.json"), "utf8"));
  assert.equal(receipt.status, "staged-not-rehearsed");
  assert.equal(receipt.referenceDirectory, source);
  for (const [name, bytes] of Object.entries(files)) {
    assert.equal(await readFile(path.join(source, name), "utf8"), bytes, "original remains unchanged");
    assert.equal(await readFile(path.join(destination, name), "utf8"), bytes, "copy is byte-identical");
    assert.equal(receipt.referenceFiles[name], createHash("sha256").update(bytes).digest("hex"));
  }
  assert.deepEqual(Object.keys(receipt.referenceFiles).sort(), Object.keys(files).sort());
  const handoff = await readFile(path.join(destination, "service-worker.js"), "utf8");
  assert.deepEqual(receipt.overlayFiles, {
    "service-worker.js": createHash("sha256").update(handoff).digest("hex"),
  });
  await assertHandoff(handoff, "https://example.test/boardstudio/");

  for (const target of [destination, path.join(directory, "empty-existing"), path.join(source, "nested")]) {
    if (target.endsWith("empty-existing")) await mkdir(target);
    const rejected = run("stage-rollback.mjs", [source, target]);
    assert.notEqual(rejected.status, 0, `must refuse unsafe output ${target}`);
    assert.match(rejected.stderr, /EEXIST|outside the read-only reference artifact/);
  }
  await writeFile(path.join(source, "service-worker.js"), "existing reference file");
  const collision = run("stage-rollback.mjs", [source, path.join(directory, "collision")]);
  assert.notEqual(collision.status, 0);
  assert.match(collision.stderr, /overlay would replace a reference artifact file/);
});
