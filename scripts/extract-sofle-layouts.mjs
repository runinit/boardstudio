// Extract measurements only; demo footprints come from the bundled catalogue.
import { readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { resolve } from 'node:path';
import { parseForms, child, value } from './kicad-forms.mjs';

const checkout = process.argv[2];
if (!checkout) throw new Error('Usage: node scripts/extract-sofle-layouts.mjs /path/to/SofleKeyboard');
const revision = execFileSync('git', ['-C', checkout, 'rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
const expectedRevision = 'fb294f7c58d0f91c379a9ef339159d4d056643e9';
if (revision !== expectedRevision) throw new Error(`Expected upstream revision ${expectedRevision}`);
const children = (node, head) => node.filter(item => Array.isArray(item) && item[0] === head);
const point = node => ({ x: Number(value(node[1])), y: Number(value(node[2])) });
const close = (a, b) => Math.hypot(a.x - b.x, a.y - b.y) < 0.002;
const layouts = {};
for (const variant of ['v2', 'RGB', 'Choc']) {
  const path = `Sofle_${variant}/PCB/SofleKeyboard.kicad_pcb`;
  const source = readFileSync(resolve(checkout, path), 'utf8');
  const board = parseForms(source)[0];
  const components = children(board, 'module').flatMap(module => {
    const reference = value(child(module, 'fp_text')[2]);
    if (!/^(SW\d+|U1|J2|J3|RSW1|TH\d+|D3[1-7])$/.test(reference)) return [];
    const at = child(module, 'at');
    return [{ reference, ...point(at), rotation: Number(value(at[3] ?? '0')) }];
  }).sort((a, b) => a.reference.localeCompare(b.reference, 'en', { numeric: true }));
  const edges = board.filter(node => Array.isArray(node) && child(node, 'layer')?.[1] === 'Edge.Cuts');
  if (edges.some(edge => edge[0] !== 'gr_line')) throw new Error('Expected polygonal source edges');
  const segments = edges.map(edge => [point(child(edge, 'start')), point(child(edge, 'end'))]);
  const contours = [];
  while (segments.length) {
    const [start, end] = segments.shift();
    const outline = [start, end];
    while (!close(outline[0], outline.at(-1))) {
      const tail = outline.at(-1);
      const index = segments.findIndex(([a, b]) => close(tail, a) || close(tail, b));
      if (index < 0) throw new Error(`${variant}: disconnected outline at ${JSON.stringify(tail)}`);
      const [a, b] = segments.splice(index, 1)[0];
      outline.push(close(tail, a) ? b : a);
    }
    outline.pop();
    contours.push(outline);
  }
  const area = points => Math.abs(points.reduce((sum, a, i) => {
    const b = points[(i + 1) % points.length];
    return sum + a.x * b.y - b.x * a.y;
  }, 0));
  contours.sort((a, b) => area(b) - area(a));
  const [outline, ...cutouts] = contours;
  layouts[variant.toLowerCase()] = { path, sha256: createHash('sha256').update(source).digest('hex'), components, outline, sourceCutoutCount: cutouts.length };
}
writeFileSync(new URL('../content/layouts/sofle-layouts.json', import.meta.url), JSON.stringify({ repository: 'https://github.com/josefadamcik/SofleKeyboard', revision, layouts }, null, 2) + '\n');
