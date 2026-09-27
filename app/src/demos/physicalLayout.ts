import type { Matrix, Part, ProjectDoc, Vec2 } from '@boardstudio/v2-contracts';
import type { MatrixPresetId } from '../ui/assemblyCatalog';
import { matrixWithPreset } from '../ui/matrixPresets';
import { importedPartDefinitions } from '../parts/catalogue';

type MeasuredKey = { reference: string; x: number; y: number; rotation: number; width: number };
type PhysicalKey = MeasuredKey & { row: number; column: number; cluster: string };
const number = (key: MeasuredKey) => Number(key.reference.match(/\d+$/)?.[0]);

/** Source references identify physical fingers/rows; scan coordinates are assigned by the core. */
export function physicalKeys(id: string, keys: readonly MeasuredKey[]): PhysicalKey[] {
  const regular = (key: MeasuredKey, columns: number, rows: number, first = 1): PhysicalKey => {
    const index = number(key) - first;
    return { ...key, cluster: 'keys', row: rows - 1 - Math.floor(index / columns), column: index % columns };
  };
  const thumb = (key: MeasuredKey): PhysicalKey => ({ ...key, cluster: 'thumbs', row: 0, column: 0 });
  let result: PhysicalKey[];
  if (['corne', 'lily58', 'sweep', 'chocofi'].includes(id)) {
    const rows = id === 'lily58' ? 4 : 3;
    result = keys.map(key => number(key) > rows * 6 ? thumb(key) : regular(key, 6, rows));
    if (id === 'sweep' || id === 'chocofi') result.forEach(key => { if (key.cluster === 'keys') key.column -= 1; });
  } else if (id === 'cantor') {
    result = keys.map(key => number(key) >= 30 ? thumb(key) : { ...key, cluster: 'keys', row: 2 - Math.floor(number(key) / 10), column: number(key) % 10 });
  } else if (id === 'totem') {
    result = keys.map(key => number(key) >= 17 ? thumb(key) : number(key) === 16 ? { ...key, cluster: 'outer', row: 0, column: 0 } : regular(key, 5, 3));
  } else if (id === 'klor') {
    result = keys.map(key => {
      const n = number(key);
      return n >= 19 ? thumb(key) : { ...key, cluster: 'keys', row: n <= 5 ? 2 : n <= 11 ? 1 : 0, column: n <= 5 ? n - 1 : n <= 11 ? n - 6 : n - 12 };
    });
  } else if (id === 'reviung41') {
    result = keys.map(key => number(key) >= 37 ? thumb(key) : { ...regular({ ...key, reference: `SW${(number(key) - 1) % 18 + 1}` }, 6, 3), reference: key.reference, cluster: number(key) <= 18 ? 'left-keys' : 'right-keys' });
  } else {
    const groups = new Map<string, MeasuredKey[]>();
    for (const key of keys) {
      let cluster = 'keys';
      if (id === 'lumberjack') cluster = key.x < 150 ? 'left-keys' : 'right-keys';
      if (id === 'mysterium' || id === 'voyager104' || id === 'voyager97') {
        const navX = id === 'mysterium' ? 330 : id === 'voyager104' ? 330 : 320;
        const numX = id === 'voyager104' ? 390 : id === 'voyager97' ? 350 : Infinity;
        const top = Math.min(...keys.map(key => key.y));
        const bottom = Math.max(...keys.map(key => key.y));
        cluster = key.x >= numX ? 'numpad' : key.x >= navX ? (key.y >= bottom - 20 ? 'arrows' : 'navigation') : key.y < top + 5 ? 'function' : 'keys';
        if (id === 'voyager97' && ['MX_LEFT1', 'MX_DOWN1', 'MX_RIGHT1', 'MX_UP1'].includes(key.reference)) cluster = 'arrows';
      }
      groups.set(cluster, [...groups.get(cluster) ?? [], key]);
    }
    result = [];
    for (const [cluster, members] of groups) {
      const levels: number[] = [];
      for (const key of [...members].sort((a,b)=>b.y-a.y)) if (!levels.some(y=>Math.abs(y-key.y)<0.5)) levels.push(key.y);
      const bottom = Math.max(...members.map(key=>key.y));
      const left = Math.min(...members.map(key=>key.x));
      for (const key of members) {
        const row = cluster === 'numpad' ? Math.round((bottom-key.y)/19.05) : levels.findIndex(y=>Math.abs(y-key.y)<0.5);
        const rowKeys = members.filter(other=>Math.abs(other.y-key.y)<0.5).sort((a,b)=>a.x-b.x);
        const column = cluster === 'numpad' || cluster === 'arrows' ? Math.round((key.x-left)/19.05) : rowKeys.indexOf(key);
        result.push({ ...key, cluster, row, column });
      }
    }
  }
  const thumbs = result.filter(key=>key.cluster==='thumbs').sort((a,b)=>a.x-b.x);
  thumbs.forEach((key,column)=>{key.column=column;});
  return result;
}

const rotate = (point: Vec2, degrees: number, origin: Vec2): Vec2 => {
  const angle=degrees*Math.PI/180;
  const x=point.x-origin.x, y=point.y-origin.y;
  return {x:origin.x+x*Math.cos(angle)-y*Math.sin(angle),y:origin.y+x*Math.sin(angle)+y*Math.cos(angle)};
};

/** Build ordinary matrix parameters in board coordinates, with only residual per-key offsets. */
export function appendPhysicalMatrix(document: ProjectDoc, keys: PhysicalKey[], id: string, boardId: string, preset: MatrixPresetId, pitch: Vec2): Matrix {
  const rows=Math.max(...keys.map(key=>key.row))+1, columns=Math.max(...keys.map(key=>key.column))+1;
  const origin={x:Math.min(...keys.map(key=>key.x)),y:Math.min(...keys.map(key=>key.y))};
  const angles=Array.from({length:columns},(_,column)=>keys.find(key=>key.column===column)?.rotation??0);
  const splays=angles.map((angle,index)=>angle-(angles[index-1]??0));
  const origins: Vec2[]=[], offsets: Vec2[]=[], staggers: number[]=[];
  const inverse = (point: Vec2, through: number) => {
    let result=point;
    for(let col=0;col<=through;col++) result=rotate(result,-splays[col],origins[col]);
    return result;
  };
  for(let column=0;column<columns;column++) {
    const bottom=keys.filter(key=>key.column===column).sort((a,b)=>a.row-b.row)[0];
    let base=bottom?{x:bottom.x-origin.x,y:bottom.y-origin.y}:{x:column*pitch.x,y:0};
    // Remove the row rise in the measured column's frame before locating its pivot.
    if(bottom) { const rise=rotate({x:0,y:bottom.row*pitch.y},angles[column],{x:0,y:0});base={x:base.x-rise.x,y:base.y-rise.y}; }
    const local=inverse(base,column-1);
    origins.push(local);
    offsets.push({x:local.x-column*pitch.x,y:0});
    staggers.push(local.y-(origins[column-1]?.y??0));
  }
  const byCell=new Map(keys.map(key=>[`${key.row}:${key.column}`,key]));
  if(byCell.size!==keys.length) throw new Error(`Duplicate physical cell in ${id}`);
  const cells=Array.from({length:rows*columns},(_,index)=>{
    const row=Math.floor(index/columns),column=index%columns,key=byCell.get(`${row}:${column}`);
    const local=key?inverse({x:key.x-origin.x,y:key.y-origin.y},column):origins[column];
    return {row,column,enabled:Boolean(key),offset:{x:local.x-origins[column].x,y:local.y-origins[column].y-row*pitch.y},rotation:key?key.rotation-angles[column]:0};
  });
  const seed:Matrix={id,name:keys[0].cluster.replaceAll('-',' '),boardId,rows,columns,origin,pitch,definitionId:'',partIds:[],columnOffsets:offsets,columnStaggers:staggers,columnSplays:splays,columnOrigins:origins,cells,diodeDirection:'col2row'};
  const prepared=matrixWithPreset(seed,preset);
  document.definitions.push(...prepared.definitions);
  const matrix=prepared.matrix;
  const stabilizers=importedPartDefinitions();
  const parts:Part[]=[];
  for(const cell of matrix.cells.filter(cell=>cell.enabled)) {
    const key=byCell.get(`${cell.row}:${cell.column}`)!;
    const partId=`matrix/${id}/r${cell.row}c${cell.column}`;
    const at={x:key.x,y:key.y}, rotation=key.rotation;
    parts.push({id:partId,definitionId:cell.definitionId!,reference:`${boardId}-${key.cluster}-SW${cell.row*columns+cell.column+1}`,properties:{sourceReference:key.reference},pose:{at,rotation},side:'front',...(key.width!==1?{keycap:{x:18+(key.width-1)*19.05,y:18}}:{})});
    if(key.width>=2) {
      const stabilizer=stabilizers.find(definition=>definition.id.endsWith(key.width===6.25?'6.25u':'2u'))!;
      if(!document.definitions.some(definition=>definition.id===stabilizer.id)) document.definitions.push(stabilizer);
      cell.assemblies.push({id:'stabilizer',definitionId:stabilizer.id,offset:{x:0,y:0},rotation:0,side:'front'});
    }
    for(const member of cell.assemblies) {
      // Assembly rotations are relative to the complete physical key rotation.
      const target=rotate(member.offset,rotation,{x:0,y:0});
      parts.push({id:`${partId}/${member.id}`,definitionId:member.definitionId,reference:`${boardId}-${member.id}-${keys.indexOf(key)+1}-${keys[0].cluster}`,side:member.side,pose:{at:{x:at.x+target.x,y:at.y+target.y},rotation:rotation+member.rotation}});
    }
  }
  matrix.partIds=parts.map(part=>part.id);
  document.parts.push(...parts);
  document.matrices.push(matrix);
  document.layouts??=[];
  document.layouts.push({id:`${id}-layout`,name:matrix.name??'Keys',boardId,matrixId:id,partIds:[]});
  return matrix;
}
