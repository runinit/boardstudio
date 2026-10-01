import type { ModuleDefinition } from '../../../contracts/src/index';
import './module-workspace.css';

/** This view shows the authored board frame; assembly placement is resolved in Rust. */
export function ModulePreview({ definition }: { definition: ModuleDefinition }) {
  const points = definition.board.contours.flatMap(contour=>contour.points);
  const minX=Math.min(0,...points.map(point=>point.x)),maxX=Math.max(0,...points.map(point=>point.x));
  const minY=Math.min(0,...points.map(point=>point.y)),maxY=Math.max(0,...points.map(point=>point.y));
  const boardPath=definition.board.contours.map(contour=>`M${contour.points.map(point=>`${point.x},${point.y}`).join('L')}Z`).join(' ');
  return <section className="wb-module-preview" aria-label="Module source preview">
    <header><h2>{definition.name}</h2><small>VIK module · {definition.variant}</small></header>
    <svg role="img" aria-label={`${definition.name} source board and components`} viewBox={`${minX-5} ${-maxY-5} ${Math.max(10,maxX-minX+10)} ${Math.max(10,maxY-minY+10)}`}>
      <g transform="scale(1,-1)"><path className="module-board" d={boardPath} fillRule="evenodd"/>
        {definition.board.holes?.map((points,index)=><polygon key={`hole/${index}`} className="module-hole" points={points.map(point=>`${point.x},${point.y}`).join(' ')}/>)}
        {definition.circuit?.parts.map(part=>{const source=definition.circuit?.definitions.find(item=>item.id===part.definitionId);return <g key={part.id} transform={`translate(${part.pose.at.x} ${part.pose.at.y}) rotate(${part.pose.rotation}) scale(${part.side==='back'?-1:1},1)`}><title>{part.reference} · {source?.name}</title><polygon className="module-component" points={source?.courtyard.map(point=>`${point.x},${point.y}`).join(' ')}/>{source?.pads.filter(pad=>pad.drill).map(pad=><circle className="module-hole" key={pad.id} cx={pad.at.x} cy={pad.at.y} r={(pad.drill ?? 0)/2}/>)}</g>;})}
      </g>
    </svg>
    <p>{definition.board.thickness===undefined?'PCB thickness needs review':`${definition.board.thickness} mm PCB`} · {definition.constituents.length} source components · {definition.mounts.length} mounting holes. Attach above or below the host PCB, or copy the circuit into your board.</p>
  </section>;
}
