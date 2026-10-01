import { moduleReviewDemoId, type DemoId } from './keyboards';
import keyboards from './keyboard-layouts.json';
import sofle from './sofle-layouts.json';

/** Use the measured layouts directly; browsing must not construct 18 complete projects. */
export function keyboardPreview(id: DemoId) {
  if (id === moduleReviewDemoId) {
    return { boardCount: 1, keys: Array.from({ length: 15 }, (_, index) => ({
      id: `review-key-${index}`, x: (index % 5) * 19.05, y: -Math.floor(index / 5) * 19.05,
      angle: 0, width: 18, height: 18,
    })) };
  }
  if (id === 'v2' || id === 'rgb' || id === 'choc') {
    const layout = sofle.layouts[id];
    const minX = Math.min(...layout.outline.map(point => point.x));
    const maxX = Math.max(...layout.outline.map(point => point.x));
    const minY = Math.min(...layout.outline.map(point => point.y));
    const measuredKeys = layout.components.filter(part => /^SW\d+$/.test(part.reference) && part.reference !== 'SW25');
    return { boardCount: 2, keys: ['left', 'right'].flatMap(half => measuredKeys.map(key => ({
      id: `${half}-${key.reference}`, x: half === 'left' ? maxX - key.x : key.x - minX + maxX - minX + 30,
      y: key.y - minY, angle: (half === 'left' ? 1 : -1) * (key.rotation - (id === 'v2' ? 0 : 180)), width: 18, height: 18,
    }))) };
  }

  const layout = keyboards[id];
  const minX = Math.min(...layout.keys.map(key => key.x - key.width * 9.525));
  const maxX = Math.max(...layout.keys.map(key => key.x + key.width * 9.525));
  const minY = Math.min(...layout.keys.map(key => key.y));
  const width = maxX - minX + 44;
  return { boardCount: layout.split ? 2 : 1, keys: (layout.split ? ['left', 'right'] : ['main']).flatMap(board => {
    const mirror = layout.split && ((board === 'right') !== (id === 'lily58' || id === 'klor'));
    return layout.keys.map(key => ({
      id: `${board}-${key.reference}`, x: (mirror ? maxX - key.x : key.x - minX) + 6 + (board === 'right' ? width + 25 : 0),
      y: key.y - minY, angle: mirror ? key.rotation : -key.rotation, width: 18 + (key.width - 1) * 19.05, height: 18,
    }));
  }) };
}
