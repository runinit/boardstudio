// Retain measured key positions, not upstream footprints, circuits, or artwork.
import { readFileSync, writeFileSync, mkdirSync, copyFileSync, existsSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { resolve } from 'node:path';
import { parseForms, child, value } from '../ergogen/src/index.ts';
const root = process.argv[2];
if (!root) throw new Error('Usage: node scripts/extract-keyboard-layouts.mjs /path/to/checkouts');
const numbered = (prefix, count) => reference => new RegExp(`^${prefix}\\d+$`).test(reference) && Number(reference.slice(prefix.length)) <= count;
const configurations = [
  ['corne', 'Corne', 'corne-v3', 'corne-cherry/pcb/corne-cherry.kicad_pcb', true, false, 21, numbered('SW', 21)],
  ['lily58', 'Lily58', 'Lily58', 'Pro/PCB/Lily58_Pro.kicad_pcb', true, false, 29, numbered('SW', 29)],
  ['sweep', 'Ferris Sweep', 'Sweep', 'Sweep v2.2/sweepv2.kicad_pcb', true, true, 17, r => /^SW\d+$/.test(r)],
  ['chocofi', 'Chocofi', 'chocofi', 'pcb/chocofi.kicad_pcb', true, true, 18, r => /^SW\d+$/.test(r)],
  ['reviung41', 'REVIUNG41', 'reviung', 'reviung41/pcb/ver1.3/reviung41.kicad_pcb', false, false, 41, numbered('SW', 41)],
  ['totem', 'TOTEM', 'TOTEM', 'PCB/totem_0-3/totem_0_3.kicad_pcb', true, true, 19, r => /^SWL\d+$/.test(r)],
  ['klor', 'KLOR', 'KLOR', 'PCB/klor1_3/klor1_3.kicad_pcb', true, false, 21, () => true],
  ['cantor', 'Cantor', 'cantor', 'Cantor_Classic/keyboard_pcb.kicad_pcb', true, true, 21, () => true],
  ['gh60', 'GH60 · ANSI', 'gh60', 'keyboard.kicad_pcb', false, false, 61, (_, name) => name === 'mx1a:MX1A'],
  ['discipline', 'Discipline · ANSI', 'discipline', 'discipline-pcb.kicad_pcb', false, false, 68, numbered('SW', 68)],
  ['mysterium', 'Mysterium · ANSI', 'mysterium', 'mysterium-pcb.kicad_pcb', false, false, 87, r => /^SW\d+$/.test(r) && ![51,77,80,81,84,86,89,90].includes(Number(r.slice(2)))],
  ['voyager97', 'Voyager97 · 103-key layout', 'Voyager97', 'Voyager97.kicad_pcb', false, false, 103, r => !['BS2','BS1-2','CL1-2','LM1-2','LM3-2','RM1-2','RM3-2','SP2'].includes(r) && !/^MX_NUM(11|15|19|20|21|23)$/.test(r)],
  ['voyager104', 'Voyager104 · ANSI', 'Voyager104', 'Voyager104.kicad_pcb', false, false, 104, r => r !== 'MX_\\2' && !/^MX_(BACK[23]|CLOCK2|LCTRL2|LSHIFT[23]|RETURN2|RSHIFT[23]|SP2|LALT2|LGUI2|RALT2|RCTRL2|RGUI2|NUM(1|2|3|4|12|16|20|21|22|24))$/.test(r)],
  ['plaid', 'Plaid', 'plaid', 'pcb/plaid.kicad_pcb', false, false, 48, numbered('SW',48)],
  ['lumberjack', 'Lumberjack', 'lumberjack-keyboard', 'lumberjack.kicad_pcb', false, false, 60, numbered('MX',60)],
];
const layouts = {};
const licenses = new URL('../content/demos/licenses/', import.meta.url);
mkdirSync(licenses, { recursive: true });
for (const [id, name, folder, path, split, choc, count, include] of configurations) {
  const checkout = resolve(root, folder);
  const revision = execFileSync('git', ['-C', checkout, 'rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
  const repository = execFileSync('git', ['-C', checkout, 'remote', 'get-url', 'origin'], { encoding: 'utf8' }).trim().replace(/\.git$/, '');
  const source = readFileSync(resolve(checkout, path), 'utf8');
  const board = parseForms(source)[0];
  const keys = board.filter(n => Array.isArray(n) && ['module', 'footprint'].includes(n[0])).flatMap(module => {
    const footprint = value(module[1]);
    if (!/MX|PG1350|choc-v1|SK6812MINI_and_cherry/i.test(footprint) || /(?:^|:)(?:Stab|MXST)|(?:_|-)Stabilizer$|wire|led/i.test(footprint)) return [];
    const ref = module.find(n => Array.isArray(n) && ((n[0] === 'fp_text' && n[1] === 'reference') || (n[0] === 'property' && value(n[1]) === 'Reference')));
    const reference = value(ref?.[2] ?? '');
    if (!include(reference, footprint)) return [];
    const at = child(module, 'at');
    const width = Number(footprint.match(/(?:_|-)([\d.]+)[uU]/)?.[1] ?? 1);
    return [{ reference, x: Number(value(at[1])), y: Number(value(at[2])), rotation: ((Number(value(at[3] ?? '0')) + 90) % 180 + 180) % 180 - 90, width }];
  }).sort((a,b) => a.y-b.y || a.x-b.x);
  if (keys.length !== count) throw new Error(`${id}: expected ${count}, got ${keys.length}: ${keys.map(k=>k.reference).join(',')}`);
  // Source switches with a separately drawn stabilizer have no key size metadata.
  const widths = id === 'gh60' ? { S66:2,S02:1.5,S67:1.5,S03:1.75,S68:2.25,S04:2.25,S69:2.75,S30:6.25,S05:1.25,S10:1.25,S15:1.25,S55:1.25,S60:1.25,S65:1.25,S70:1.25 } : id === 'discipline' ? {SW62:6.25,SW63:1,SW64:1} : id === 'mysterium' ? {SW83:6.25} : {};
  for (const key of keys) key.width = widths[key.reference] ?? key.width;
  const license = ['LICENSE','LICENSE.md','LICENSE.txt'].find(file => existsSync(resolve(checkout, file)));
  if (license) copyFileSync(resolve(checkout, license), new URL(`${id}.txt`, licenses));
  layouts[id] = { name, split, choc, repository, revision, path, sha256: createHash('sha256').update(source).digest('hex'), licenseFile: license ? `licenses/${id}.txt` : null, keys };
}
writeFileSync(new URL('../tooling/demo-projects/src/keyboard-layouts.json', import.meta.url), JSON.stringify(layouts, null, 2)+'\n');
console.log(`Measured ${Object.keys(layouts).length} layouts`);
