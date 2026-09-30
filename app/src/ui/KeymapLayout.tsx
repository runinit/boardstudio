import type { KeymapView } from './createKeymapWorkspace';

export function KeymapLayout({ keys, selected, onSelect }: {
  keys: KeymapView['keys'];
  selected: ReadonlySet<string>; onSelect: (id: string) => void;
}) {
  return <g className="wb-keymap-layout">{keys.map(({ part, color, size, legend }) => {
    const rgb = parseInt(color.slice(1), 16); const foreground = (((rgb >> 16) & 255) * .299 + ((rgb >> 8) & 255) * .587 + (rgb & 255) * .114) > 150 ? '#182331' : '#ffffff';
    return <g key={part.id} transform={`translate(${part.pose.at.x} ${part.pose.at.y}) rotate(${part.pose.rotation})`} className={selected.has(part.id) ? 'is-selected' : ''} role="button" tabIndex={0} aria-label={`Edit key ${part.reference}`} onClick={event => { event.stopPropagation(); onSelect(part.id); }} onKeyDown={event => { if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); onSelect(part.id); } }}>
      <rect x={-size.x / 2} y={-size.y / 2} width={size.x} height={size.y} rx="1.3" fill={color} />
      <g transform="scale(1 -1)"><text textAnchor="middle" dominantBaseline="central" fill={foreground} fontSize={Math.min(4, 13 / Math.max(1, legend.length) * 1.3)}>{legend || '—'}</text><text textAnchor="middle" y="6" fontSize="2.1" fill={foreground}>{part.reference}</text></g>
    </g>;
  })}</g>;
}
