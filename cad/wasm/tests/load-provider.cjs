const fs = require('node:fs');
const path = require('node:path');
const { pathToFileURL } = require('node:url');

// Share qualified-analysis's actual WEB binding initialization; Node builds remain usable manually.
module.exports = async function loadProvider(packageArgument, webBinding = process.argv.includes('--web')) {
  const packagePath = path.resolve(packageArgument);
  if (!webBinding) return require(packagePath);
  const cad = await import(pathToFileURL(packagePath));
  await cad.default({ module_or_path: fs.readFileSync(packagePath.replace(/\.js$/, '_bg.wasm')) });
  return cad;
};
