import { CanvasLayers } from './CanvasLayers';
import type { Part, PartDefinition } from '../../../contracts/src/index';
import { Ergogen2DPreview, ergogenPreviewLayers } from './Ergogen2DPreview';

export const boardLayer = (layer: string, side: Part['side']) => side === 'back'
  ? layer.startsWith('F.') ? `B.${layer.slice(2)}` : layer.startsWith('B.') ? `F.${layer.slice(2)}` : layer
  : layer;
export function footprintLayers(definition: PartDefinition, side: Part['side']): string[] {
  return [...new Set([
    ...ergogenPreviewLayers(definition, true).map((layer) => boardLayer(layer, side)),
    ...definition.pads.filter((pad) => pad.plated !== false).flatMap((pad) => pad.drill ? ['F.Cu', 'B.Cu'] : [boardLayer(pad.side === 'back' ? 'B.Cu' : 'F.Cu', side)]),
  ])].sort();
}
export function WorkbenchLayers({ layers, hidden, onToggle }: { layers: string[]; hidden: ReadonlySet<string>; onToggle: (layer: string) => void }) {
  const label = (layer: string) => layer === 'F.Cu' ? 'Front copper' : layer === 'B.Cu' ? 'Back copper' : layer === 'Edge.Cuts' ? 'Board outline' : layer;
  const groups = layers.includes('Edge.Cuts') ? [
    { title: 'Copper', layers: layers.filter((layer) => layer.endsWith('.Cu')) },
    { title: 'Technical', layers: layers.filter((layer) => layer.includes('.') && !layer.endsWith('.Cu') && layer !== 'Edge.Cuts') },
    { title: 'Objects', layers: layers.filter((layer) => !layer.includes('.') || layer === 'Edge.Cuts') },
  ] : [{ title: '', layers }];
  return <CanvasLayers groups={groups.map(group => ({ title: group.title, layers: group.layers.map(layer => ({ id: layer, label: label(layer), accessibilityLabel: layer })) }))} hidden={hidden} onToggle={onToggle} />;
}

export function SceneFootprint({ definition, part, hidden }: { definition: PartDefinition; part: Part; hidden: ReadonlySet<string> }) {
  const drawing = definition.generator && part.generatorParameters ? { ...definition, generator: { ...definition.generator, parameters: { ...definition.generator.parameters, ...part.generatorParameters } } } : definition;
  const hiddenGraphics = new Set(ergogenPreviewLayers(definition, true).filter((layer) => hidden.has(boardLayer(layer, part.side))).map((layer) => `graphics:${layer}`));
  return <g className="wb-scene-footprint">
    <g transform="scale(1,-1)"><Ergogen2DPreview definition={drawing} hideKeycap hiddenLayers={hiddenGraphics} /></g>
    {definition.pads.map((pad) => {
      const copperVisible = pad.plated !== false && !hidden.has('Pads') && (pad.drill
        ? !hidden.has('F.Cu') || !hidden.has('B.Cu')
        : !hidden.has(boardLayer(pad.side === 'back' ? 'B.Cu' : 'F.Cu', part.side)));
      return <g key={pad.id} transform={`translate(${pad.at.x} ${pad.at.y}) rotate(${pad.rotation ?? 0})`}>
        {copperVisible && <rect className="wb-part-pad" x={-pad.size.x / 2} y={-pad.size.y / 2} width={pad.size.x} height={pad.size.y} rx={pad.shape === 'circle' || pad.shape === 'oval' ? Math.min(pad.size.x, pad.size.y) / 2 : pad.shape === 'roundrect' ? Math.min(pad.size.x, pad.size.y) / 4 : 0} />}
        {pad.drill && !hidden.has('Holes') && <circle className="wb-part-drill" r={pad.drill / 2} />}
      </g>;
    })}
  </g>;
}
export function KeycapOverlay({ size }: { size: { x: number; y: number } }) {
  const inset = Math.min(1.5, size.x / 6, size.y / 6);
  return <g className="wb-keycap-overlay" aria-hidden="true">
    <rect x={-size.x / 2} y={-size.y / 2} width={size.x} height={size.y} rx=".9" />
    <rect className="wb-keycap-top" x={-size.x / 2 + inset} y={-size.y / 2 + inset} width={size.x - inset * 2} height={size.y - inset * 2} rx=".7" />
  </g>;
}
