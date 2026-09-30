import type { Part, PartDefinition, ProjectDoc } from '@boardstudio/v2-contracts';
import { defaultKeycapBoard, keyBinding } from '../keycapSettings';
import { bindingLabel } from './keyBindingChoices';

export function KeymapLayout({ document, boardId, parts, definitions, selected, onSelect }: {
  document: ProjectDoc; boardId: string; parts: Part[]; definitions: Map<string, PartDefinition>;
  selected: ReadonlySet<string>; onSelect: (id: string) => void;
}) {
  const colors = document.keycaps?.boards[boardId] ?? defaultKeycapBoard;
  return <g className="wb-keymap-layout">{parts.filter(part => definitions.get(part.definitionId)?.kind === 'switch').map(part => {
    const settings = document.keycaps?.keys[part.id];
    const size = settings?.units ? { x: settings.units.x * 19.05 - 0.85, y: settings.units.y * 19.05 - 0.85 } : part.keycap ?? definitions.get(part.definitionId)?.keycap ?? { x: 18.2, y: 18.2 };
    const color = settings?.color ?? colors.color;
    const rgb = parseInt(color.slice(1), 16); const foreground = (((rgb >> 16) & 255) * .299 + ((rgb >> 8) & 255) * .587 + (rgb & 255) * .114) > 150 ? '#182331' : '#ffffff';
    const legend = settings?.legend ?? bindingLabel(keyBinding(document, boardId, part.id));
    return <g key={part.id} transform={`translate(${part.pose.at.x} ${part.pose.at.y}) rotate(${part.pose.rotation})`} className={selected.has(part.id) ? 'is-selected' : ''} role="button" tabIndex={0} aria-label={`Edit key ${part.reference}`} onClick={event => { event.stopPropagation(); onSelect(part.id); }} onKeyDown={event => { if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); onSelect(part.id); } }}>
      <rect x={-size.x / 2} y={-size.y / 2} width={size.x} height={size.y} rx="1.3" fill={color} />
      <g transform="scale(1 -1)"><text textAnchor="middle" dominantBaseline="central" fill={foreground} fontSize={Math.min(4, 13 / Math.max(1, legend.length) * 1.3)}>{legend || '—'}</text><text textAnchor="middle" y="6" fontSize="2.1" fill={foreground}>{part.reference}</text></g>
    </g>;
  })}</g>;
}
