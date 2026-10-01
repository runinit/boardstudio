import fs from "node:fs/promises";
import path from "node:path";

const [wasmPackDirectory, manifestPath, outputPath] = process.argv.slice(2);
if (!wasmPackDirectory || !manifestPath || !outputPath) {
  throw new Error(
    "usage: node scripts/web/embed-worker-wasm.mjs <wasm-pack-dir> <offline-manifest.json> <service-worker.js>",
  );
}

const manifest = JSON.parse(await fs.readFile(manifestPath, "utf8"));
if (
  typeof manifest.version !== "string" ||
  !/^[A-Za-z0-9._-]+$/.test(manifest.version) ||
  !Array.isArray(manifest.assets) ||
  manifest.assets.length === 0 ||
  !manifest.assets.includes("index.html") ||
  manifest.assets.some(
    (asset) =>
      typeof asset !== "string" ||
      !asset ||
      asset.startsWith("/") ||
      asset.includes("\\") ||
      asset.includes("%") ||
      /[?#:]/.test(asset) ||
      asset.split("/").some((part) => !part || part === "." || part === ".."),
  )
) {
  throw new Error("offline manifest is invalid or has no required assets");
}

const wasmName = "boardstudio_offline_worker_bg.wasm";
const jsName = "boardstudio_offline_worker.js";
const wasm = await fs.readFile(path.join(wasmPackDirectory, wasmName));
const js = await fs.readFile(path.join(wasmPackDirectory, jsName), "utf8");
const outputDirectory = path.dirname(outputPath);
await fs.mkdir(outputDirectory, { recursive: true });
await fs.writeFile(path.join(outputDirectory, jsName), js);

const encoded = wasm.toString("base64");
const bootstrap = `/* Generated synchronous WASM initializer; offline policy is Rust-owned. */
import { initSync } from "./${jsName}";
const wasm = Uint8Array.from(atob("${encoded}"), (character) => character.charCodeAt(0));
initSync({ module: wasm });
`;
await fs.writeFile(outputPath, bootstrap);
console.log(
  JSON.stringify({
    version: manifest.version,
    assets: manifest.assets.length,
    wasmBytes: wasm.byteLength,
    initializerBytes: Buffer.byteLength(bootstrap),
  }),
);
