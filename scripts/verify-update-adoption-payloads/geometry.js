(() => {
  // Layout, contrast and motion of the visible Update Now controls under the
  // current Theme, Appearance, viewport and motion preference.
  const toRgba = (c) => {
    const k = document.createElement('canvas'); k.width = k.height = 1;
    const g = k.getContext('2d', { willReadFrequently: true });
    g.clearRect(0, 0, 1, 1); g.fillStyle = c; g.fillRect(0, 0, 1, 1);
    return Array.from(g.getImageData(0, 0, 1, 1).data);
  };
  const lum = ([r, g, b]) => {
    const f = (v) => { v /= 255; return v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4); };
    return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b);
  };
  const background = (el) => {
    let node = el;
    while (node) {
      const c = toRgba(getComputedStyle(node).backgroundColor);
      if (c[3] === 255) return c;
      node = node.parentElement;
    }
    return toRgba('white');
  };
  const probe = (selector) => {
    const el = document.querySelector(selector);
    if (!el) return null;
    const r = el.getBoundingClientRect();
    const cs = getComputedStyle(el);
    const fg = toRgba(cs.color), bg = background(el);
    const hi = Math.max(lum(fg), lum(bg)), lo = Math.min(lum(fg), lum(bg));
    return { w: Math.round(r.width), h: Math.round(r.height),
             inViewport: r.left >= 0 && r.top >= 0 && r.right <= innerWidth && r.bottom <= innerHeight,
             contrast: Math.round(((hi + 0.05) / (lo + 0.05)) * 100) / 100,
             fontPx: parseFloat(cs.fontSize) };
  };
  return {
    theme: document.documentElement.getAttribute('data-theme'),
    appearance: document.documentElement.getAttribute('data-appearance'),
    viewport: [innerWidth, innerHeight],
    reducedMotion: matchMedia('(prefers-reduced-motion: reduce)').matches,
    runningAnimations: document.getAnimations().filter((a) => a.playState === 'running').length,
    toast: probe("[data-slot=toast-provider] [aria-label='Update Now']"),
    header: probe(".header-update [aria-label='Update Now']"),
  };
})()
