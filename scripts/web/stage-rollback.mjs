import { createHash } from "node:crypto";
import { constants } from "node:fs";
import fs from "node:fs/promises";
import path from "node:path";

const [sourceArgument, destinationArgument] = process.argv.slice(2);
if (!sourceArgument || !destinationArgument || process.argv.length !== 4) {
  throw new Error("usage: node scripts/web/stage-rollback.mjs <pinned-react-site> <new-output-site>");
}
const source = await fs.realpath(sourceArgument);
const requested = path.resolve(destinationArgument);
const parent = await fs.realpath(path.dirname(requested));
const destination = path.join(parent, path.basename(requested));
const relative = path.relative(source, destination);
if (!relative || (!relative.startsWith(`..${path.sep}`) && relative !== ".." && !path.isAbsolute(relative))) {
  throw new Error("rollback output must be outside the read-only reference artifact");
}

const hash = (bytes) => createHash("sha256").update(bytes).digest("hex");
async function inventory(directory, prefix = "") {
  const entries = await fs.readdir(path.join(directory, prefix), { withFileTypes: true });
  const files = {};
  for (const entry of entries.sort((a, b) => a.name.localeCompare(b.name))) {
    const name = path.posix.join(prefix, entry.name);
    if (entry.isDirectory()) Object.assign(files, await inventory(directory, name));
    else if (entry.isFile()) files[name] = hash(await fs.readFile(path.join(directory, name)));
    else throw new Error(`rollback source contains a non-file entry: ${name}`);
  }
  return files;
}
const sourceFiles = await inventory(source);
if (!sourceFiles["index.html"] || !sourceFiles["sw.js"]) {
  throw new Error("rollback requires a pinned React shell and classic sw.js");
}
if (sourceFiles["service-worker.js"] || sourceFiles["rollback-provenance.json"]) {
  throw new Error("rollback overlay would replace a reference artifact file");
}
// Intentionally not recursive: every existing target, even an empty directory,
// is rejected. Nothing in the reference tree is modified or removed.
await fs.mkdir(destination);
for (const [name, expected] of Object.entries(sourceFiles)) {
  const output = path.join(destination, name);
  await fs.mkdir(path.dirname(output), { recursive: true });
  await fs.copyFile(path.join(source, name), output, constants.COPYFILE_EXCL);
  if (hash(await fs.readFile(output)) !== expected) throw new Error(`reference changed during copy: ${name}`);
}
if (JSON.stringify(await inventory(source)) !== JSON.stringify(sourceFiles)) {
  throw new Error("reference changed during rollback staging; output is incomplete");
}
const handoff = await fs.readFile(new URL("./service-worker-handoff.js", import.meta.url));
await fs.writeFile(path.join(destination, "service-worker.js"), handoff, { flag: "wx" });
const provenance = {
  status: "staged-not-rehearsed",
  referenceDirectory: source,
  referenceFiles: sourceFiles,
  overlayFiles: { "service-worker.js": hash(handoff) },
  policy: "Byte-identical React artifact plus a module-compatible worker handoff; no project or cache storage deletion. Serve at the reference artifact's original deployment prefix.",
};
await fs.writeFile(path.join(destination, "rollback-provenance.json"), `${JSON.stringify(provenance, null, 2)}\n`, { flag: "wx" });
console.log(JSON.stringify({ destination, files: Object.keys(sourceFiles).length, handoffSha256: hash(handoff) }));
