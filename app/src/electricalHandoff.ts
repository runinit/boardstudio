import type { ElectricalPlan, JumperRecipe, ProjectDoc } from '@boardstudio/v2-contracts';

export function existingConnectionReview(document: ProjectDoc, plan: ElectricalPlan) {
  const conflicts = new Set(plan.diagnostics.filter(finding => finding.code === 'manual-net-conflict').map(finding => finding.keyId));
  const proposed = new Set(plan.nets.flatMap(net => net.pins.map(pin => JSON.stringify([pin.partId, pin.padId]))));
  const matrices = document.matrices.filter(matrix => matrix.boardId === plan.boardId);
  return document.nets.filter(net => !net.id.startsWith(`generated/electrical/${plan.boardId}/`)
    && !matrices.some(matrix => net.id.startsWith(`matrix/${matrix.id}/net/`)))
    .map(net => ({ id: net.id, name: net.name, pins: net.pins.filter(pin => conflicts.has(pin.partId) && proposed.has(JSON.stringify([pin.partId, pin.padId]))) }))
    .filter(net => net.pins.length > 0);
}

export function releaseReviewedConnections(document: ProjectDoc, plan: ElectricalPlan): ProjectDoc {
  const review = existingConnectionReview(document, plan);
  return { ...document, nets: document.nets.map(net => {
    const entry = review.find(entry => entry.id === net.id);
    return entry ? { ...net, pins: net.pins.filter(pin => !entry.pins.some(removed => removed.partId === pin.partId && removed.padId === pin.padId)) } : net;
  }) };
}

export function pcbAssemblyFiles(plan: ElectricalPlan, draft: boolean, populations: { name: string; plan: ElectricalPlan }[] = []): Record<string, string> {
  const assemblies = populations.length ? populations : [{ name: 'PCB assembly', plan }];
  const files: Record<string, string> = {
    'wiring-report.json': JSON.stringify({ draft, plan, populations }, null, 2),
    'ASSEMBLY.md': assemblies.map(assembly => `# ${assembly.name}\n\n${assemblyInstructions(assembly.plan, draft)}`).join('\n\n---\n\n'),
  };
  for (const [populationIndex, assembly] of assemblies.entries()) {
    for (const [recipeIndex, recipe] of assembly.plan.jumpers.entries()) {
      files[`jumpers/assembly-${populationIndex + 1}/part-${recipeIndex + 1}.svg`] = jumperDiagram(recipe);
    }
  }
  return files;
}

function assemblyInstructions(plan: ElectricalPlan, draft: boolean): string {
  const lines = [
    '# PCB wiring and assembly', '',
    draft ? 'DRAFT: review the unresolved findings below before fabrication.' : 'Ready for routing. This package does not certify routed-board DRC or fabrication readiness.', '',
    `PCB: ${plan.boardId}`, `Wiring revision: ${plan.revision}`, `Controller: ${plan.controllerPartId ?? 'Unresolved'}`, '',
    'The PCB file retains separate local nets on the two sides of every open solder gap. Close only the bridges listed below for this population. Leave the opposite-face bridges open.', '',
  ];
  for (const recipe of plan.jumpers) {
    lines.push(`## ${recipe.partId}`, '', `Footprint: ${recipe.source}`, '', '| Face | Local socket net | Signal | Action |', '| --- | --- | --- | --- |');
    for (const site of recipe.sites) lines.push(`| ${site.face} | ${site.localNetId} | ${site.signalTerminal} | ${site.close ? 'Bridge' : 'Leave open'}${site.routingRequired ? '; route local connection first' : ''} |`);
    lines.push('');
  }
  if (plan.diagnostics.length) {
    lines.push('## Findings', '');
    for (const finding of plan.diagnostics) lines.push(`- ${finding.severity}: ${finding.message}`);
  }
  lines.push('', 'For a wired split, use local power on each half and a straight TRRS cable: tip carries central TX / peripheral RX; ring 2 carries peripheral TX / central RX; sleeve is ground; ring 1 is unused. Disconnect power before plugging or unplugging.');
  return lines.join('\n');
}

const escape = (text: string) => text.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll('"', '&quot;');

/** Face diagrams use component-local coordinates, viewed from that face. */
function jumperDiagram(recipe: JumperRecipe): string {
  const ys = recipe.sites.map(site => site.y);
  const minY = Math.min(...ys, 0);
  const maxY = Math.max(...ys, 1);
  const height = Math.max(200, (maxY - minY) * 18 + 110);
  const sites = recipe.sites.map(site => {
    const back = site.face === 'back';
    const x = (back ? 460 : 160) + (back ? -site.x : site.x) * 12;
    const y = (site.y - minY) * 18 + 70;
    const color = site.close ? '#126b4b' : '#64748b';
    return `<g><circle cx="${x}" cy="${y}" r="5" fill="${site.close ? color : 'white'}" stroke="${color}"/><text x="${x}" y="${y - 10}" text-anchor="middle" font-size="10">${escape(site.signalTerminal)}</text></g>`;
  }).join('');
  return `<svg xmlns="http://www.w3.org/2000/svg" width="620" height="${height}" viewBox="0 0 620 ${height}"><rect width="620" height="${height}" fill="white"/><g font-family="sans-serif" fill="#172033"><text x="160" y="25" text-anchor="middle">Front · viewed from front</text><text x="460" y="25" text-anchor="middle">Back · viewed from back</text>${sites}<text x="20" y="${height - 20}" font-size="12">Filled: bridge · Hollow: leave open · Route flagged local connections first</text></g></svg>`;
}
