import fs from "node:fs/promises";
import path from "node:path";

const [directory, version] = process.argv.slice(2);
if (!directory || !/^v[0-9]+$/.test(version ?? "")) {
  throw new Error("usage: node scripts/embed-worker-wasm.mjs <wasm-pack-dir> <vN>");
}

const wasmPath = path.join(directory, `p3_sw_${version}_bg.wasm`);
const wasm = await fs.readFile(wasmPath);
const encoded = wasm.toString("base64");
const bootstrap = `/* Generated synchronous WASM initializer; service-worker policy is Rust-owned. */
import { initSync } from "./p3_sw.js";
const wasm = Uint8Array.from(atob("${encoded}"), (character) => character.charCodeAt(0));
initSync({ module: wasm });
`;
await fs.writeFile(path.join(directory, "sw-bootstrap.mjs"), bootstrap);
console.log(JSON.stringify({ version, wasmBytes: wasm.byteLength, base64Bytes: encoded.length, bootstrapBytes: Buffer.byteLength(bootstrap) }));
