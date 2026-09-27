import type { CoreReply, ProjectDoc, Vec2 } from '@boardstudio/v2-contracts';
import { CoreEngine } from '../../../../core/pkg/boardstudio_core';

export function unsupportedPads(document: ProjectDoc): string[] {
  const engine = new CoreEngine();
  try {
    const opened = JSON.parse(engine.request(JSON.stringify({ id: 'open', kind: 'open', document }))) as CoreReply;
    if (opened.kind !== 'scene') throw new Error(JSON.stringify(opened));
    const contains = (points: Vec2[], point: Vec2) => {
      let inside = false;
      for (let i = 0, j = points.length - 1; i < points.length; j = i++) {
        const a = points[i], b = points[j];
        if ((a.y > point.y) !== (b.y > point.y) && point.x < (b.x-a.x)*(point.y-a.y)/(b.y-a.y)+a.x) inside = !inside;
      }
      return inside;
    };
    return document.boards.flatMap(board => {
      const contours = opened.scene.boardContours.find(item => item.boardId === board.id)!.contours;
      return document.parts.filter(part => board.partIds.includes(part.id)).flatMap(part => {
        const definition = document.definitions.find(definition => definition.id === part.definitionId)!;
        const radians = part.pose.rotation * Math.PI / 180;
        return definition.pads.filter(pad => {
          // Sample pad bounds as well as centers; this also covers drilled pads.
          return [[0,0], [-1,-1],[-1,1],[1,-1],[1,1]].some(([dx,dy]) => {
            const angle = (pad.rotation ?? 0)*Math.PI/180;
            const u = dx*pad.size.x/2, v = dy*pad.size.y/2;
            const x = (pad.at.x+u*Math.cos(angle)-v*Math.sin(angle))*(part.side==='back'?-1:1);
            const y = pad.at.y+u*Math.sin(angle)+v*Math.cos(angle);
            const point = {x:part.pose.at.x+x*Math.cos(radians)-y*Math.sin(radians),y:part.pose.at.y+x*Math.sin(radians)+y*Math.cos(radians)};
            return !contours.some(contour=>!contour.hole && contains(contour.points,point)) || contours.some(contour=>contour.hole && contains(contour.points,point));
          });
        }).map(pad=>`${part.reference}/${pad.number}`);
      });
    });
  } finally { engine.free(); }
}
