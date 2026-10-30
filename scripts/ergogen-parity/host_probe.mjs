// Experimental CommonJS subset for this lock; this is not a security sandbox.
const manifest = await (await fetch('/manifest')).json();
const files = new Set(manifest.files);
const packages = new Set(manifest.packages);
const cache = new Map();

function normalize(parts, floor) {
  const result = [];
  for (const part of parts) {
    if (!part || part === '.') continue;
    if (part === '..') {
      if (result.length <= floor) throw Error('module path escapes its root');
      result.pop();
    } else {
      if (!/^[a-zA-Z0-9_@+.-]+$/.test(part)) throw Error('invalid module path');
      result.push(part);
    }
  }
  return result.join('/');
}

function text(path) {
  const request = new XMLHttpRequest();
  request.open('GET', '/files/' + encodeURIComponent(path), false);
  request.send();
  if (request.status !== 200) throw Error('module route failed: ' + path);
  return request.responseText;
}

function resolve(spec, from) {
  if (typeof spec !== 'string' || !spec || spec.startsWith('/') || spec.includes('\\')) {
    throw Error('invalid module specifier');
  }
  let base;
  if (spec.startsWith('./') || spec.startsWith('../')) {
    const parts = from.split('/');
    parts.pop();
    const floor = parts[0] === 'upstream' ? 1 : (parts[1].startsWith('@') ? 3 : 2);
    base = normalize([...parts, ...spec.split('/')], floor);
  } else {
    const parts = spec.split('/');
    const count = spec.startsWith('@') ? 2 : 1;
    const name = parts.slice(0, count).join('/');
    if (!packages.has(name) || parts.some(p => !p || p === '.' || p === '..')) {
      throw Error('unlocked or invalid module: ' + spec);
    }
    base = normalize(['node_modules', ...parts], count + 1);
  }
  return resolveFile(base, new Set());
}

function resolveFile(base, visited) {
  if (visited.has(base)) throw Error('recursive package main: ' + base);
  visited.add(base);
  for (const path of [base, base + '.js', base + '.json']) {
    if (files.has(path)) return path;
  }
  const packageFile = base + '/package.json';
  if (files.has(packageFile)) {
    const main = JSON.parse(text(packageFile)).main;
    if (main !== undefined) {
      if (typeof main !== 'string' || !main || main.startsWith('/') || main.includes('\\')) {
        throw Error('invalid package main: ' + base);
      }
      const target = normalize([...base.split('/'), ...main.split('/')], base.split('/').length);
      return resolveFile(target, visited);
    }
  }
  if (files.has(base + '/index.js')) return base + '/index.js';
  if (files.has(base + '/index.json')) return base + '/index.json';
  throw Error('missing module: ' + base);
}

function load(spec, from = 'upstream/src/entry.js') {
  const path = resolve(spec, from);
  if (cache.has(path)) return cache.get(path).exports;
  const module = { exports: {} };
  cache.set(path, module);
  try {
    const source = text(path);
    if (path.endsWith('.json')) module.exports = JSON.parse(source);
    else new Function('require', 'module', 'exports', '__filename', '__dirname', source)(
      name => load(name, path), module, module.exports, path, path.slice(0, path.lastIndexOf('/')));
  } catch (error) {
    cache.delete(path);
    throw error;
  }
  return module.exports;
}

// Repository-authored data with no imports; it is evaluated exactly once per page.
function authored() {
  const module = { exports: {} };
  new Function('require', 'module', 'exports', text('boardstudio/cases.cjs'))(
    name => { throw Error('cases.cjs cannot require ' + name); }, module, module.exports);
  const { cases, boardStudioOnly } = module.exports;
  return { cases, boardStudioOnly };
}

export async function run(suite) {
  if (suite !== 'ergogen-host-probe') throw Error('unknown suite');
  if (manifest.mode === 'source') return [{ suite, case: 'source', ok: true, json: JSON.stringify(authored()) }];
  if (manifest.mode === 'missing') load('./does-not-exist.js');
  if (manifest.mode === 'builtin') load('node:fs');
  if (manifest.mode === 'traversal') load('../../outside.js');
  if (manifest.mode === 'retry') {
    let failed = false;
    try { load('./units.js'); } catch (_) { failed = true; }
    if (!failed) throw Error('expected initial module failure');
  }
  const points = load('./points.js');
  const units = load('./units.js');
  if (manifest.mode === 'generate') {
    // As the original oracle: configurations cross JSON, points.parse gets a clone, points cross JSON.
    const source = authored();
    const configs = JSON.parse(JSON.stringify(source.cases.map(item => item.ergogenConfig)));
    const actual = configs.map(config => Object.fromEntries(Object.entries(
      points.parse(structuredClone(config.points), units.parse(config.units || {})),
    ).map(([name, point]) => [name, { x: point.x, y: point.y, r: point.r }])));
    return [{ suite, case: 'generate', ok: true, json: JSON.stringify({ ...source, actual }) }];
  }
  if (manifest.mode === 'cases' || manifest.mode === 'invalid-case') {
    return manifest.cases.map(({ case: id, config }, index) => {
      const input = manifest.mode === 'invalid-case' && index === 0 ? null : structuredClone(config.points);
      const result = Object.fromEntries(Object.entries(points.parse(input, units.parse(config.units || {})))
        .map(([name, point]) => {
          if (!name || ![point.x, point.y, point.r].every(Number.isFinite)) throw Error('invalid point');
          return [name, { x: point.x, y: point.y, r: point.r }];
        }));
      if (!Object.keys(result).length) throw Error('empty points');
      return { suite, case: id, ok: true, result };
    });
  }
  const config = manifest.mode === 'invalid' ? null : {
    zones: { m: { anchor: { shift: [10, 20], rotate: 30 }, columns: { a: {} }, rows: { r0: {} } } },
  };
  const result = Object.fromEntries(Object.entries(points.parse(config, units.parse({})))
    .map(([name, point]) => {
      if (![point.x, point.y, point.r].every(Number.isFinite)) throw Error('nonfinite point');
      return [name, { x: point.x, y: point.y, r: point.r }];
    }));
  return [{ suite, case: 'pinned-parser', ok: true, result }];
}
