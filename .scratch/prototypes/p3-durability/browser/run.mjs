import assert from "node:assert/strict";
import { createRequire } from "node:module";
import fs from "node:fs/promises";
import http from "node:http";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, "..");
const appRequire = createRequire(path.join(root, "..", "..", "..", "app", "package.json"));
const { chromium } = appRequire("@playwright/test");
const siteRoot = path.join(root, "dist", "root");
const updateRoot = path.join(root, "dist", "update");
let updateMode = false;

const mime = (file) => ({
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".wasm": "application/wasm",
  ".boardstudio": "application/zip",
  ".txt": "text/plain; charset=utf-8",
}[path.extname(file)] ?? "application/octet-stream");

const server = http.createServer(async (request, response) => {
  const url = new URL(request.url, "http://127.0.0.1");
  const scopedPath = url.pathname.startsWith("/boardstudio/")
    ? url.pathname.slice("/boardstudio".length)
    : url.pathname;
  const relativePath = decodeURIComponent(scopedPath).replace(/^\/+/, "") || "index.html";
  let allowedRoot = siteRoot;
  if (updateMode && !url.pathname.startsWith("/boardstudio/") && ["p3_sw.js", "sw-bootstrap.mjs", "version.txt"].includes(relativePath)) {
    allowedRoot = updateRoot;
  }
  if (updateMode && !url.pathname.startsWith("/boardstudio/") && relativePath === "p3_sw_v2_bg.wasm") {
    allowedRoot = updateRoot;
  }
  const file = path.resolve(allowedRoot, relativePath);
  if (!file.startsWith(allowedRoot + path.sep)) {
    response.writeHead(400).end("bad path");
    return;
  }
  try {
    const body = await fs.readFile(file);
    response.writeHead(200, {
      "content-type": mime(file),
      "cache-control": "no-store",
      "cross-origin-opener-policy": "same-origin",
    });
    response.end(body);
  } catch {
    response.writeHead(404, { "content-type": "text/plain" }).end("not found");
  }
});

await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
const address = server.address();
const origin = `http://127.0.0.1:${address.port}`;
const browser = await chromium.launch({
  executablePath: process.env.P3_CHROMIUM ?? "/usr/bin/chromium",
  headless: true,
  args: ["--no-sandbox", "--disable-dev-shm-usage"],
});
const evidence = { browser: await browser.version(), origin, cases: [] };

async function cacheNames(page) {
  return page.evaluate(() => caches.keys());
}

async function waitControlled(page, scope) {
  await page.waitForFunction(
    (expected) => navigator.serviceWorker.controller?.scriptURL.includes(expected),
    scope === "/" ? "sw-bootstrap.mjs" : "sw-bootstrap.mjs",
    { timeout: 20_000 },
  );
  return page.evaluate(() => navigator.serviceWorker.controller.scriptURL);
}

async function checkOfflineEntry(page, url, expectedScope) {
  await page.context().setOffline(true);
  const result = await page.goto(url, { waitUntil: "load", timeout: 20_000 });
  assert.equal(result.status(), 200, `cached navigation should return 200 for ${url}`);
  await page.waitForFunction(() => document.documentElement.dataset.p3WasmReady === "true");
  assert.equal(await page.locator("h1").textContent(), "REVIUNG41 durability probe");
  assert.equal(
    await page.evaluate(() => document.documentElement.dataset.p3ServiceWorkerScope),
    expectedScope,
  );
  const missing = await page.evaluate(async () => {
    try {
      const response = await fetch("./lazy/case-step.chunk.js");
      return { kind: "http-response", status: response.status };
    } catch (error) {
      return { kind: "network-error", name: error.name };
    }
  });
  assert.deepEqual(missing, { kind: "network-error", name: "TypeError" });
  await page.context().setOffline(false);
  return { navigationStatus: result.status(), missingLazyAsset: missing };
}

try {
  const context = await browser.newContext({ serviceWorkers: "allow" });
  const page = await context.newPage();
  await page.goto(`${origin}/cold.html`);
  assert.deepEqual(await cacheNames(page), [], "cold network-only entry starts without a cache");
  await page.evaluate(async () => {
    const unrelated = await caches.open("unrelated-stale-cache");
    await unrelated.put(
      new Request(new URL("./version.txt", location.href)),
      new Response("stale-other-registration", { headers: { "content-type": "text/plain" } }),
    );
  });

  await page.goto(`${origin}/index.html?skip-sw=1`, { waitUntil: "load" });
  await page.waitForFunction(() => document.documentElement.dataset.p3WasmReady === "true");
  const fixture = await page.evaluate(() => p3.importFixture());
  assert.deepEqual(fixture, {
    id: "edbec892-9952-4b6b-a7ba-cb6b0a514ac4",
    name: "REVIUNG41",
    partCount: 85,
    assetCount: 4,
  });
  const initialRoundtrip = await page.evaluate(() => p3.archiveStoredProjectRoundtrip());
  assert.equal(initialRoundtrip.assetCount, 4);
  const initialStored = await page.evaluate(() => p3.loadStoredProject());
  const committedRevision = initialStored.document.revision + 1;
  const storageShape = await page.evaluate(async () => {
    const request = indexedDB.open("boardstudio-p3-durability", 1);
    const db = await new Promise((resolve, reject) => {
      request.onsuccess = () => resolve(request.result);
      request.onerror = () => reject(request.error);
    });
    const tx = db.transaction(["projects", "assets"], "readonly");
    const result = {
      projectKeyPath: tx.objectStore("projects").keyPath,
      assetKeyPath: tx.objectStore("assets").keyPath,
      storeNames: [...db.objectStoreNames].sort(),
      activeProjectId: localStorage.getItem("boardstudio-p3-durability-active-project"),
    };
    db.close();
    return result;
  });
  assert.deepEqual(storageShape, {
    projectKeyPath: "id",
    assetKeyPath: null,
    storeNames: ["assets", "projects"],
    activeProjectId: fixture.id,
  });

  const committed = await page.evaluate(() => p3.commitOneEdit());
  assert.equal(committed.partCount, 85);
  const forcedAbort = await page.evaluate(async () => {
    try {
      return { saved: await p3.savePending(true) };
    } catch (error) {
      return {
        name: error.name,
        transactionEvent: error.transactionEvent,
        requestsSucceeded: error.requestsSucceeded,
      };
    }
  });
  assert.equal(forcedAbort.transactionEvent, "abort");
  assert.equal(forcedAbort.requestsSucceeded, 1, "one IDB request succeeded before the real transaction abort");
  const afterAbort = await page.evaluate(() => p3.loadStoredProject());
  assert.equal(afterAbort.document.revision, initialStored.document.revision, "aborted transaction leaves old committed document visible");
  assert.equal(afterAbort.assetCount, 4, "aborted asset writes leave the original asset set intact");
  const saved = await page.evaluate(() => p3.retryPending());
  assert.deepEqual(saved, {
    operationId: "p3-durability-edit-1",
    saveAttempt: 2,
    revision: committedRevision,
    engineCommitCount: 1,
    transactionEvent: "complete",
    requestsSucceeded: 5,
  });
  const afterRetry = await page.evaluate(() => p3.loadStoredProject());
  assert.equal(afterRetry.document.revision, committedRevision);
  assert.equal(afterRetry.assetCount, 4);
  const undo = await page.evaluate(() => p3.undoOnce());
  assert.equal(undo.revision, committedRevision + 1, "one Undo undoes the sole engine edit after storage retry");
  assert.equal(undo.partX, initialStored.document.parts[0].pose.at.x, "Undo restores the original part position");

  const archived = await page.evaluate(() => p3.archiveStoredProjectRoundtrip());
  assert.equal(archived.assetCount, 4);
  assert.equal(archived.projectId, fixture.id);
  evidence.cases.push({
    name: "IndexedDB completion, abort rollback, same-snapshot retry and archive/assets",
    initialRevision: initialStored.document.revision,
    abortedRevisionRemained: afterAbort.document.revision,
    abortTerminal: forcedAbort,
    retryReceipt: saved,
    undo,
    archiveRoundtrip: archived,
    storageShape,
  });
  await fs.writeFile(path.join(root, "evidence", "browser-storage-results.json"), `${JSON.stringify({ browser: await browser.version(), origin, cases: evidence.cases }, null, 2)}\n`);

  await page.goto(`${origin}/leave.html`);
  await page.goto(`${origin}/index.html?skip-sw=1`, { waitUntil: "load" });
  await page.waitForFunction(() => document.documentElement.dataset.p3WasmReady === "true");
  const reopened = await page.evaluate(() => p3.loadStoredProject());
  assert.equal(reopened.document.revision, committedRevision, "saved snapshot survives navigation and reload");
  assert.equal(reopened.document.id, fixture.id);
  assert.equal(reopened.assetCount, 4);
  evidence.cases.push({ name: "root navigation away and saved project reopen", revision: reopened.document.revision, assetCount: reopened.assetCount });
  await fs.writeFile(path.join(root, "evidence", "browser-storage-results.json"), `${JSON.stringify({ browser: await browser.version(), origin, cases: evidence.cases }, null, 2)}\n`);

  await page.goto(`${origin}/index.html`, { waitUntil: "load" });
  await page.waitForFunction(() => document.documentElement.dataset.p3WasmReady === "true");
  await page.waitForFunction(() => document.documentElement.dataset.p3ServiceWorkerScope === `${location.origin}/`);
  const rootController = await waitControlled(page, "/");
  const coldAssetWasNetworkFetched = await page.evaluate(async () => {
    const response = await fetch("./version.txt");
    return response.ok && (await response.text()).trim() === "p3-cache-v1";
  });
  assert.equal(coldAssetWasNetworkFetched, true);
  const scopedCacheProbe = await page.evaluate(async () => {
    const response = await fetch("./version.txt");
    return (await response.text()).trim();
  });
  assert.equal(scopedCacheProbe, "p3-cache-v1", "worker ignores matching stale data in an unrelated CacheStorage cache");
  evidence.cases.push({
    name: "root cold registration and first-evaluation Rust listener capture",
    scope: await page.evaluate(() => document.documentElement.dataset.p3ServiceWorkerScope),
    controller: rootController,
    caches: await cacheNames(page),
    unrelatedStaleCacheDidNotLeak: scopedCacheProbe,
  });

  const rootOffline = await checkOfflineEntry(page, `${origin}/index.html`, `${origin}/`);
  evidence.cases.push({ name: "root cached offline reopen and missing lazy asset", ...rootOffline });

  await page.goto(`${origin}/boardstudio/cold.html`);
  await page.goto(`${origin}/boardstudio/index.html`, { waitUntil: "load" });
  await page.waitForFunction(() => document.documentElement.dataset.p3WasmReady === "true");
  await page.waitForFunction(() => document.documentElement.dataset.p3ServiceWorkerScope === `${location.origin}/boardstudio/`);
  const subpathController = await waitControlled(page, "/boardstudio/");
  const scopesBeforeUpdate = await page.evaluate(async () => navigator.serviceWorker.getRegistrations().then((regs) => regs.map((reg) => reg.scope).sort()));
  const cachesBeforeUpdate = await cacheNames(page);
  assert(scopesBeforeUpdate.includes(`${origin}/`));
  assert(scopesBeforeUpdate.includes(`${origin}/boardstudio/`));
  assert(cachesBeforeUpdate.includes("boardstudio-p3-durability-path-2f-v1"));
  assert(cachesBeforeUpdate.includes("boardstudio-p3-durability-path-2f626f61726473747564696f2f-v1"));
  evidence.cases.push({ name: "subpath module registration retains both independent cache scopes", scope: `${origin}/boardstudio/`, controller: subpathController, scopes: scopesBeforeUpdate, caches: cachesBeforeUpdate });

  updateMode = true;
  const update = await page.evaluate(async () => {
    const registrations = await navigator.serviceWorker.getRegistrations();
    const rootRegistration = registrations.find((registration) => new URL(registration.scope).pathname === "/");
    if (!rootRegistration) throw new Error("Root service worker registration missing");
    await rootRegistration.update();
    for (let attempt = 0; attempt < 100; attempt += 1) {
      const names = await caches.keys();
      if (names.includes("boardstudio-p3-durability-path-2f-v2") && !names.includes("boardstudio-p3-durability-path-2f-v1")) {
        return { names, activeScript: rootRegistration.active?.scriptURL };
      }
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
    return { names: await caches.keys(), activeScript: rootRegistration.active?.scriptURL };
  });
  assert(update.names.includes("boardstudio-p3-durability-path-2f-v2"), "updated Rust policy installs v2 cache");
  assert(!update.names.includes("boardstudio-p3-durability-path-2f-v1"), "activation removes previous root cache");
  assert(update.names.includes("boardstudio-p3-durability-path-2f626f61726473747564696f2f-v1"), "root update preserves subpath cache");
  assert(update.names.includes("unrelated-stale-cache"), "root update preserves unrelated caches");
  evidence.cases.push({ name: "root worker update replaces only root cache", ...update });

  const rootOfflineV2 = await checkOfflineEntry(page, `${origin}/index.html`, `${origin}/`);
  const rootVersion = await page.evaluate(async () => {
    const response = await fetch("./version.txt");
    return (await response.text()).trim();
  });
  assert.equal(rootVersion, "p3-cache-v2", "updated root worker serves its own v2 cached asset");
  const subpathOffline = await checkOfflineEntry(page, `${origin}/boardstudio/index.html`, `${origin}/boardstudio/`);
  evidence.cases.push({ name: "root v2 cached offline reopen", ...rootOfflineV2 });
  evidence.cases.push({ name: "root update serves v2 independently", version: rootVersion });
  evidence.cases.push({ name: "subpath cached offline reopen", ...subpathOffline });

  const versionProbe = await page.evaluate(async () => {
    const response = await fetch("./version.txt");
    return (await response.text()).trim();
  });
  assert.equal(versionProbe, "p3-cache-v1", "subpath cache remains on its own version after root update");
  evidence.cases.push({ name: "subpath cache retains its independent asset version", version: versionProbe });

  const coldContext = await browser.newContext({ serviceWorkers: "allow" });
  const coldPage = await coldContext.newPage();
  await coldPage.goto(`${origin}/cold.html`, { waitUntil: "load" });
  const coldState = await coldPage.evaluate(async () => ({ caches: await caches.keys(), registrations: await navigator.serviceWorker.getRegistrations() }));
  assert.deepEqual(coldState, { caches: [], registrations: [] });
  await coldContext.setOffline(true);
  let coldFailure;
  try {
    await coldPage.goto(`${origin}/index.html`, { waitUntil: "load", timeout: 10_000 });
  } catch (error) {
    coldFailure = error.message;
  }
  assert.match(coldFailure ?? "", /ERR_INTERNET_DISCONNECTED|offline/i);
  evidence.cases.push({ name: "fresh context with no worker or cache fails offline honestly", ...coldState, navigationFailure: coldFailure });
  await coldContext.close();

  await fs.mkdir(path.join(root, "evidence"), { recursive: true });
  await fs.writeFile(path.join(root, "evidence", "browser-results.json"), `${JSON.stringify(evidence, null, 2)}\n`);
  await context.close();
  console.log(JSON.stringify(evidence, null, 2));
} finally {
  await browser.close();
  await new Promise((resolve) => server.close(resolve));
}
