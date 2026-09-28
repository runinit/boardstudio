#!/usr/bin/env node
import { readFile, readdir, stat } from 'node:fs/promises';
import { createRequire } from 'node:module';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const rootDirectory = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const require = createRequire(path.join(rootDirectory, 'app/package.json'));
const ts = require('typescript/unstable/ast');
const { API } = require('typescript/unstable/sync');
const { createVirtualFileSystem } = require('typescript/unstable/fs');
const packages = ['app', 'cad', 'contracts', 'ergogen', 'kicad'];
const extensions = ['.ts', '.tsx', '.mts', '.mjs', '.js'];
const excludedDirectories = new Set(['node_modules', 'dist', 'pkg', 'target', '.git', '.impeccable', '.generated', 'library']);
const isTest = file => /\.(?:test|spec)\.[cm]?[jt]sx?$/.test(file) || /(?:^|\/)(?:test|e2e)\//.test(file);
const isGenerated = file => file.startsWith('contracts/src/generated/');
const entrypoints = [
  'app/src/main.tsx',
  'app/src/core.worker.ts', 'app/src/export.worker.ts', 'app/src/case.worker.ts', 'app/src/scene.worker.ts',
  // Separate HTML benchmark documents load these roots, not the app shell.
  'app/src/bench.ts', 'app/src/bench-workbench.tsx', 'app/src/bench-cad.ts',
  'cad/src/index.ts', 'contracts/src/index.ts', 'kicad/src/index.ts', 'ergogen/src/index.ts',
];
// These package facades also describe runtime/worker contracts whose complete
// surface is checked by native, contract, and package integration tests.
const publicFacades = new Map([
  ['contracts/src/index.ts', 'Rust-generated protocol plus shared document constructors'],
  ['cad/src/index.ts', 'CAD worker/runtime package boundary'],
]);

async function walk(directory, accept, prefix = '') {
  let entries;
  try { entries = await readdir(directory, { withFileTypes: true }); }
  catch (error) { if (error.code === 'ENOENT') return []; throw error; }
  const files = [];
  for (const entry of entries) {
    const relative = path.posix.join(prefix, entry.name);
    if (entry.isDirectory() && !excludedDirectories.has(entry.name)) {
      files.push(...await walk(path.join(directory, entry.name), accept, relative));
    } else if (entry.isFile() && accept(relative)) files.push(relative);
  }
  return files;
}

function analyze(ast) {
  const edges = [];
  const exported = new Set();
  function visit(node) {
    if (ts.isImportDeclaration(node) && ts.isStringLiteral(node.moduleSpecifier)) {
      const names = [];
      if (node.importClause?.name) names.push('default');
      const bindings = node.importClause?.namedBindings;
      if (bindings && ts.isNamespaceImport(bindings)) names.push('*');
      if (bindings && ts.isNamedImports(bindings)) {
        names.push(...bindings.elements.map(binding => (binding.propertyName ?? binding.name).text));
      }
      edges.push({ specifier: node.moduleSpecifier.text, names });
    }
    if (ts.isExportDeclaration(node)) {
      const names = node.exportClause && ts.isNamedExports(node.exportClause)
        ? node.exportClause.elements.map(binding => (binding.propertyName ?? binding.name).text) : ['*'];
      if (node.moduleSpecifier && ts.isStringLiteral(node.moduleSpecifier)) {
        edges.push({ specifier: node.moduleSpecifier.text, names });
      }
      if (node.exportClause && ts.isNamedExports(node.exportClause)) {
        node.exportClause.elements.forEach(binding => exported.add(binding.name.text));
      }
    }
    if (ts.isExportAssignment(node)) exported.add('default');
    const modifiers = node.modifiers ?? [];
    if (modifiers.some(modifier => modifier.kind === ts.SyntaxKind.ExportKeyword)) {
      if (modifiers.some(modifier => modifier.kind === ts.SyntaxKind.DefaultKeyword)) exported.add('default');
      else if (node.name && ts.isIdentifier(node.name)) exported.add(node.name.text);
      else if (ts.isVariableStatement(node)) {
        node.declarationList.declarations.forEach(declaration => {
          if (ts.isIdentifier(declaration.name)) exported.add(declaration.name.text);
        });
      }
    }
    if (ts.isCallExpression(node) && node.expression.kind === ts.SyntaxKind.ImportKeyword && ts.isStringLiteral(node.arguments[0])) {
      // Import promises expose module namespaces. Do not guess which members
      // survive callbacks/destructuring; this exception is local to this edge.
      edges.push({ specifier: node.arguments[0].text, names: ['*'] });
    }
    if (ts.isNewExpression(node) && ts.isIdentifier(node.expression) && node.expression.text === 'URL'
      && node.arguments?.length === 2 && ts.isStringLiteral(node.arguments[0])
      && node.arguments[1].getText(ast) === 'import.meta.url') {
      edges.push({ specifier: node.arguments[0].text, names: [] });
    }
    node.forEachChild(visit);
  }
  visit(ast);
  return { edges, exported };
}

async function check(root = rootDirectory) {
  const files = new Set();
  const manifests = new Map();
  const packageExports = new Map();
  const isSource = file => extensions.includes(path.extname(file)) && !file.endsWith('.d.ts');
  for (const directory of ['', ...packages]) {
    const manifest = path.posix.join(directory, 'package.json');
    try {
      const json = JSON.parse(await readFile(path.join(root, manifest), 'utf8'));
      manifests.set(manifest, json);
      const exports = typeof json.exports === 'string' ? { '.': json.exports } : json.exports ?? {};
      for (const [subpath, target] of Object.entries(exports)) {
        if (typeof target === 'string') packageExports.set(`${json.name}${subpath === '.' ? '' : subpath.slice(1)}`, path.posix.join(directory, target));
      }
    } catch (error) { if (error.code !== 'ENOENT') throw error; }
    if (!directory) continue;
    for (const subdirectory of ['src', 'scripts', 'test', 'e2e']) {
      for (const file of await walk(path.join(root, directory, subdirectory), isSource)) files.add(path.posix.join(directory, subdirectory, file));
    }
  }
  // Minimal fixtures need not reproduce every workspace manifest.
  for (const directory of packages.filter(directory => directory !== 'app')) {
    if (!packageExports.has(`@boardstudio/v2-${directory}`)) packageExports.set(`@boardstudio/v2-${directory}`, `${directory}/src/index.ts`);
  }
  const infos = new Map();
  // TypeScript 7 parses through its native API; one virtual project keeps this
  // syntax-only check independent of application typechecking and module resolution.
  const configFile = path.join(root, '__repo_check__.json');
  const sources = Object.fromEntries(await Promise.all([...files].map(async file =>
    [path.join(root, file), await readFile(path.join(root, file), 'utf8')])));
  sources[configFile] = JSON.stringify({
    compilerOptions: { allowJs: true, noLib: true, noResolve: true, noCheck: true, types: [] },
    files: [...files],
  });
  const api = new API({ cwd: root, fs: createVirtualFileSystem(sources) });
  try {
    const snapshot = api.updateSnapshot({ openProject: configFile });
    const project = snapshot.getProject(configFile);
    if (!project) throw new Error('Could not initialize repository syntax check');
    for (const file of files) {
      const ast = project.program.getSourceFile(path.join(root, file));
      if (!ast) throw new Error(`Could not parse ${file}`);
      infos.set(file, analyze(ast));
    }
  } finally {
    api.close();
  }
  function resolve(from, specifier) {
    const clean = specifier.split(/[?#]/)[0];
    const base = clean.startsWith('.') ? path.posix.normalize(path.posix.join(path.posix.dirname(from), clean)) : packageExports.get(clean);
    if (!base) return undefined;
    return [base, ...extensions.map(extension => `${base}${extension}`), ...extensions.map(extension => `${base}/index${extension}`)].find(candidate => files.has(candidate));
  }
  const reachable = new Set();
  function visit(file) {
    if (reachable.has(file) || !infos.has(file)) return;
    reachable.add(file);
    for (const edge of infos.get(file).edges) {
      const target = resolve(file, edge.specifier);
      if (target) visit(target);
    }
  }
  entrypoints.forEach(visit);
  const issues = entrypoints.filter(file => !files.has(file)).map(file => ({ kind: 'missing-entrypoint', file, message: 'Explicit entrypoint does not exist' }));
  const used = new Map();
  for (const [file, info] of infos) {
    for (const edge of info.edges) {
      const target = resolve(file, edge.specifier);
      if (!target) continue;
      const names = used.get(target) ?? new Set();
      edge.names.forEach(name => names.add(name));
      used.set(target, names);
    }
  }
  for (const [file, info] of infos) {
    if (isTest(file) || !file.includes('/src/') || isGenerated(file)) continue;
    if (!reachable.has(file)) issues.push({ kind: 'unreachable-module', file, message: 'No production entrypoint reaches this module (test imports do not make it production code)' });
    if (publicFacades.has(file)) continue;
    for (const name of info.exported) {
      if (!used.get(file)?.has(name) && !used.get(file)?.has('*')) {
        issues.push({ kind: 'unused-export', file, message: `Export ${name} has no import or re-export consumer` });
      }
    }
  }
  for (const [manifest, json] of manifests) {
    const prefix = path.posix.dirname(manifest);
    const specifiers = [...infos].filter(([file]) => prefix !== '.' && file.startsWith(`${prefix}/`) || prefix === '.' && !file.includes('/'))
      .flatMap(([, info]) => info.edges.map(edge => edge.specifier));
    for (const dependency of Object.keys(json.dependencies ?? {})) {
      if (!specifiers.some(specifier => specifier === dependency || specifier.startsWith(`${dependency}/`))) {
        issues.push({ kind: 'unused-dependency', file: manifest, message: `Runtime dependency ${dependency} has no import in its package` });
      }
    }
  }
  const markdown = ['README.md', 'PRODUCT.md', 'DESIGN.md', ...packages.map(directory => `${directory}/README.md`), ...(await walk(path.join(root, 'docs'), file => file.endsWith('.md'))).map(file => `docs/${file}`)];
  for (const file of markdown) {
    let source;
    try { source = await readFile(path.join(root, file), 'utf8'); }
    catch (error) { if (error.code === 'ENOENT') continue; throw error; }
    // Check file destinations, not remote availability or heading anchors.
    for (const match of source.replace(/```[\s\S]*?```/g, '').matchAll(/\[[^\]]*\]\((?:<([^>]+)>|([^\s)]+))(?:\s+"[^"]*")?\)/g)) {
      const link = match[1] ?? match[2];
      if (/^(?:[a-z][a-z\d+.-]*:|#)/i.test(link)) continue;
      const target = decodeURIComponent(link.split('#')[0]);
      try { await stat(path.resolve(root, path.dirname(file), target)); }
      catch (error) {
        if (error.code !== 'ENOENT' && error.code !== 'ENOTDIR') throw error;
        issues.push({ kind: 'broken-markdown-link', file, message: `Local link does not resolve: ${link}` });
      }
    }
  }
  return { issues, reachable: [...reachable].sort(), tracked: [...files].sort() };
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const result = await check(process.argv[2] ? path.resolve(process.argv[2]) : rootDirectory);
  for (const issue of result.issues) console.error(`${issue.kind}: ${issue.file} — ${issue.message}`);
  if (result.issues.length) process.exitCode = 1;
  else console.log(`Repository check passed: ${result.tracked.length} authored modules, ${result.reachable.length} production-reachable.`);
}
export { check };
