import type { PartDefinition } from '../../../contracts/src/index';
import { child, isErgogen, render, type Expression, value } from '@boardstudio/v2-ergogen';

const children = (node: Expression[], name: string): Expression[][] => node.filter((item): item is Expression[] => Array.isArray(item) && item[0] === name);
const scalar = (node: Expression[] | undefined, index: number): number => Number(node?.[index]);
const coords = (node: Expression[] | undefined): { x: number; y: number } | undefined => {
  if (!node) return undefined;
  const x = scalar(node, 1);
  const y = scalar(node, 2);
  // Match the Y-up coordinates used by compiled pads and courtyards. The SVG
  // renderer converts these back to screen coordinates together.
  return Number.isFinite(x) && Number.isFinite(y) ? { x, y: -y } : undefined;
};
const findForms = (node: Expression[], names: string[]): Expression[][] => node.flatMap((item) => Array.isArray(item) ? [ ...(names.includes(String(item[0])) ? [item] : []), ...findForms(item, names) ] : []);
const graphicKinds = ['fp_line', 'gr_line', 'fp_arc', 'gr_arc', 'fp_rect', 'gr_rect', 'fp_circle', 'gr_circle', 'fp_poly', 'gr_poly', 'fp_text', 'gr_text', 'zone'];
const graphicLayer = (form: Expression[]): string => {
  const layer = child(form, 'layer')?.[1];
  return typeof layer === 'string' ? value(layer) : 'Other graphics';
};
const previewDefinition = (definition: PartDefinition, hideKeycap: boolean): PartDefinition => {
  if (!hideKeycap || !definition.generator) return definition;
  const generator = definition.generator;
  return { ...definition, generator: { ...generator, parameters: { ...generator.parameters, include_keycap: false } } };
};
export const ergogenPreviewLayers = (definition: PartDefinition, hideKeycap = false): string[] => {
  if (!isErgogen(definition.generator?.source)) return [];
  try { return [...new Set(findForms(render(previewDefinition(definition, hideKeycap)), graphicKinds).map(graphicLayer))]; } catch { return []; }
};
const arcPoints = (form: Expression[]): [{ x: number; y: number }, { x: number; y: number }, { x: number; y: number }] | undefined => {
  const nativeStart = coords(child(form, 'start'));
  const nativeMid = coords(child(form, 'mid'));
  const nativeEnd = coords(child(form, 'end'));
  if (nativeStart && nativeMid && nativeEnd) return [nativeStart, nativeMid, nativeEnd];
  const center = nativeStart;
  const start = nativeEnd;
  const angleNode = child(form, 'angle');
  const angle = -Number(angleNode?.[1]) * Math.PI / 180;
  if (!center || !start || !Number.isFinite(angle)) return undefined;
  const dx = start.x - center.x;
  const dy = start.y - center.y;
  const point = (portion: number) => {
    const rotation = angle * portion;
    return { x: center.x + dx * Math.cos(rotation) - dy * Math.sin(rotation), y: center.y + dx * Math.sin(rotation) + dy * Math.cos(rotation) };
  };
  return [start, point(.5), point(1)];
};

export const ergogenPreviewPoints = (definition: PartDefinition): { x: number; y: number }[] => {
  if (!isErgogen(definition.generator?.source)) return [];
  try {
    const forms = render(definition);
    const plain = findForms(forms, ['start', 'end', 'center', 'xy', 'at']).map((form) => coords(form)).filter((point): point is { x: number; y: number } => Boolean(point));
    const arcs = findForms(forms, ['fp_arc', 'gr_arc']).flatMap((form) => arcPoints(form) ?? []);
    return [...plain, ...arcs];
  } catch { return []; }
};

export const Ergogen2DPreview = ({ definition, at = { x: 0, y: 0 }, hideKeycap = false, hiddenLayers }: { definition: PartDefinition; at?: { x: number; y: number }; hideKeycap?: boolean; hiddenLayers?: ReadonlySet<string> }) => {
  if (!isErgogen(definition.generator?.source)) return null;
  let forms: Expression[][];
  // The workspace draws its own keycap guide from the resolved envelope, including
  // authored overrides. Avoid a second, potentially different generator guide.
  try { forms = render(previewDefinition(definition, hideKeycap)); } catch { return null; }
  const graphics = findForms(forms, graphicKinds);
  const line = (form: Expression[], i: number) => {
    const start = coords(child(form, 'start'));
    const end = coords(child(form, 'end'));
    if (!start || !end) return null;
    return <line key={i} data-layer={graphicLayer(form)} x1={start.x + at.x} y1={-start.y - at.y} x2={end.x + at.x} y2={-end.y - at.y} className="wb-ergogen-line" />;
  };
  return <g className="wb-ergogen-drawing" aria-label={`${definition.name} graphics and text`}>
    {graphics.map((form, index) => {
      if (hiddenLayers?.has(`graphics:${graphicLayer(form)}`)) return null;
      const kind = String(form[0]);
      if (kind.endsWith('_line')) return line(form, index);
      if (kind.endsWith('_arc')) {
        const points = arcPoints(form);
        if (!points) return null;
        const [start, mid, end] = points;
        const controlX = 2 * mid.x - (start.x + end.x) / 2;
        const controlY = 2 * mid.y - (start.y + end.y) / 2;
        return <path key={index} data-layer={graphicLayer(form)} d={`M ${start.x + at.x} ${-start.y - at.y} Q ${controlX + at.x} ${-controlY - at.y} ${end.x + at.x} ${-end.y - at.y}`} className="wb-ergogen-line" />;
      }
      if (kind.endsWith('_rect')) {
        const start = coords(child(form, 'start'));
        const end = coords(child(form, 'end'));
        if (!start || !end) return null;
        return <rect key={index} data-layer={graphicLayer(form)} x={Math.min(start.x, end.x) + at.x} y={-Math.max(start.y, end.y) - at.y} width={Math.abs(end.x - start.x)} height={Math.abs(end.y - start.y)} className="wb-ergogen-line" />;
      }
      if (kind.endsWith('_circle')) {
        const center = coords(child(form, 'center'));
        const end = coords(child(form, 'end'));
        if (!center || !end) return null;
        return <circle key={index} data-layer={graphicLayer(form)} cx={center.x + at.x} cy={-center.y - at.y} r={Math.hypot(end.x - center.x, end.y - center.y)} className="wb-ergogen-line" />;
      }
      if (kind.endsWith('_poly') || kind === 'zone') {
        const pts = child(form, 'pts') ?? findForms([form], ['pts'])[0];
        const points = pts ? children(pts, 'xy').map((xy) => coords(xy)).filter((point): point is { x: number; y: number } => Boolean(point)) : [];
        if (points.length < 3) return null;
        const keepout = Boolean(child(form, 'keepout'));
        return <polygon key={index} data-layer={graphicLayer(form)} points={points.map((point) => `${point.x + at.x},${-point.y - at.y}`).join(' ')} className={keepout ? 'wb-ergogen-keepout' : 'wb-ergogen-zone'} />;
      }
      if (kind.endsWith('_text')) {
        const position = coords(child(form, 'at'));
        const textNode = form[kind === 'fp_text' ? 2 : 1];
        const text = typeof textNode === 'string' ? value(textNode) : '';
        if (form.includes('hide') || (kind === 'fp_text' && value(String(form[1])) === 'reference')) return null;
        if (!position || !text) return null;
        return <text key={index} data-layer={graphicLayer(form)} x={position.x + at.x} y={-position.y - at.y} className="wb-ergogen-text">{text}</text>;
      }
      return null;
    })}
  </g>;
};
