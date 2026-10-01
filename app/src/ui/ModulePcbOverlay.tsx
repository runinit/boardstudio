import type { ResolvedModule } from '../../../contracts/src/index';
import type { KeyboardEvent, MouseEvent } from 'react';
import { componentPoseSvgTransform } from './componentPreview';

export function ModulePcbOverlay({ module, hidden, hostHidden, onSelect }: {
  module: ResolvedModule;
  hidden: ReadonlySet<string>;
  hostHidden: ReadonlySet<string>;
  onSelect: () => void;
}) {
  const chooseModule = (event: MouseEvent<SVGElement> | KeyboardEvent<SVGElement>) => {
    event.stopPropagation();
    onSelect();
  };
  const keyChooseModule = (event: KeyboardEvent<SVGElement>) => {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      chooseModule(event);
    }
  };
  const selectable = {
    role: 'button' as const,
    tabIndex: 0,
    onClick: chooseModule,
    onKeyDown: keyChooseModule,
    'aria-label': `Select mounted module ${module.id}`,
  };
  return <g className="wb-module-pcb-overlay" data-module-id={module.id}>
    {!hidden.has('module-outlines') && module.board.map((outline, index) => <polygon key={`outline-${index}`} className="wb-module-board-outline" points={outline.points.map(point => `${point.x},${point.y}`).join(' ')} {...selectable} />)}
    {!hidden.has('module-clearances') && [...module.volumes, ...module.openings].map(volume => <polygon key={`clearance-${volume.id}`} className="wb-module-clearance" points={volume.geometry.points.map(point => `${point.x},${point.y}`).join(' ')} data-qualified={volume.qualified} {...selectable} />)}
    {!hidden.has('module-holes') && (module.mounts ?? []).map(mount => <g key={`hole-${mount.sourceId}`} className="wb-module-mount-hole" {...selectable}>
      <title>{`Module mounting hole ${mount.sourceId} · source drill ${mount.diameter} mm`}</title>
      <circle cx={mount.at.x} cy={mount.at.y} r={mount.diameter / 2} />
      <path d={`M${mount.at.x - mount.diameter / 2} ${mount.at.y}h${mount.diameter}M${mount.at.x} ${mount.at.y - mount.diameter / 2}v${mount.diameter}`} />
    </g>)}
    {!hidden.has('module-standoffs') && (module.mountSupports ?? []).map((support, index) => <g key={`support-${support.mountId}-${index}`} className="wb-module-standoff" {...selectable}>
      <title>{`Designer-selected support at ${support.mountId} · outer diameter ${support.outerDiameter} mm · hole ${support.holeDiameter} mm · Z ${support.z} mm · height ${support.height} mm`}</title>
      <circle cx={support.at.x} cy={support.at.y} r={support.outerDiameter / 2} />
      <circle className="wb-module-standoff-hole" cx={support.at.x} cy={support.at.y} r={support.holeDiameter / 2} />
    </g>)}
    {!hidden.has('module-footprints') && (module.footprints ?? []).map(footprint => <g key={footprint.id} className={`wb-module-source-footprint is-${footprint.side}`} transform={componentPoseSvgTransform(footprint.pose.at, footprint.pose.rotation, footprint.side)} data-source-part={footprint.sourcePartId} {...selectable}>
      <polygon className={`wb-module-footprint-courtyard${hostHidden.has('Courtyards') ? ' is-host-hidden' : ''}`} points={footprint.courtyard.map(point => `${point.x},${point.y}`).join(' ')} />
      {(footprint.surfaces ?? []).map((surface, index) => {
        const layer = surface.layer;
        const front = layer.startsWith('F.');
        const silk = layer.endsWith('.SilkS');
        const visibilityId = silk ? front ? 'module-silkscreen-front' : 'module-silkscreen-back' : front ? 'module-fab-front' : 'module-fab-back';
        const isReference = Boolean(surface.text && surface.text === footprint.reference);
        if (hidden.has(visibilityId) || hostHidden.has(layer) || isReference && hostHidden.has('References')) return null;
        const points = surface.points.map(point => `${point.x},${point.y}`).join(' ');
        const d = `${surface.points.map((point, pointIndex) => `${pointIndex ? 'L' : 'M'}${point.x} ${point.y}`).join(' ')}${surface.filled ? 'Z' : ''}`;
        return surface.text
          ? <text key={`surface-${index}`} className={`wb-module-footprint-art is-text ${silk ? 'is-silkscreen' : 'is-fabrication'}`} data-source-layer={layer} x={surface.points[0]?.x ?? 0} y={surface.points[0]?.y ?? 0} fontSize={surface.textSize || 1} transform={`rotate(${surface.rotation ?? 0} ${surface.points[0]?.x ?? 0} ${surface.points[0]?.y ?? 0}) scale(1,-1)`}>{surface.text}</text>
          : surface.filled
            ? <polygon key={`surface-${index}`} className={`wb-module-footprint-art is-filled ${silk ? 'is-silkscreen' : 'is-fabrication'}`} data-source-layer={layer} points={points} />
            : <path key={`surface-${index}`} className={`wb-module-footprint-art ${silk ? 'is-silkscreen' : 'is-fabrication'}`} data-source-layer={layer} d={d} strokeWidth={surface.width || 0.15} />;
      })}
      {footprint.pads.map(pad => {
        const copperLayer = footprint.side === 'back' ? 'B.Cu' : 'F.Cu';
        const copperVisible = !hostHidden.has('Pads') && (!hostHidden.has(copperLayer) || Boolean(pad.drill && !hostHidden.has(footprint.side === 'back' ? 'F.Cu' : 'B.Cu')));
        return <g key={pad.id} className="wb-module-source-pad" transform={`translate(${pad.at.x} ${pad.at.y}) rotate(${pad.rotation ?? 0})`} data-pad-number={pad.number}>
          {copperVisible && <rect x={-pad.size.x / 2} y={-pad.size.y / 2} width={pad.size.x} height={pad.size.y} rx={pad.shape === 'circle' || pad.shape === 'oval' ? Math.min(pad.size.x, pad.size.y) / 2 : pad.shape === 'roundrect' ? Math.min(pad.size.x, pad.size.y) / 4 : 0} />}
          {pad.drill && !hostHidden.has('Holes') && <circle className="wb-module-source-drill" r={pad.drill / 2} />}
        </g>;
      })}
      {!hostHidden.has('References') && !(footprint.surfaces ?? []).some(surface => surface.text === footprint.reference) && <text className="wb-module-source-reference" x="0" y="-2">{footprint.reference}</text>}
      <title>{`${footprint.reference} · ${footprint.name} · source module footprint`}</title>
    </g>)}
  </g>;
}
