(() => {
  const viewer = document.querySelector('[role=img][aria-label^="Interactive 3D Case preview"]');
  if (!viewer) throw Error('missing viewer');
  const describe = e => {
    const r = e.getBoundingClientRect(), s = getComputedStyle(e);
    return { tag: e.tagName, class: e.className, role: e.getAttribute('role'), text: e.textContent.trim().slice(0, 160),
      rect: { x: r.x, y: r.y, width: r.width, height: r.height }, display: s.display, visibility: s.visibility };
  };
  const ancestors = [];
  for (let e = viewer; e && ancestors.length < 6; e = e.parentElement) {
    ancestors.push({ ...describe(e), children: [...e.children].map(describe) });
  }
  return { theme: document.documentElement.dataset.theme, dark: matchMedia('(prefers-color-scheme: dark)').matches,
    pointer: Object.fromEntries(['none', 'coarse', 'fine'].map(v => [v, matchMedia(`(pointer: ${v})`).matches])),
    hover: Object.fromEntries(['none', 'hover'].map(v => [v, matchMedia(`(hover: ${v})`).matches])),
    anyPointer: Object.fromEntries(['none', 'coarse', 'fine'].map(v => [v, matchMedia(`(any-pointer: ${v})`).matches])),
    anyHover: Object.fromEntries(['none', 'hover'].map(v => [v, matchMedia(`(any-hover: ${v})`).matches])),
    touchPoints: navigator.maxTouchPoints,
    readyState: document.readyState, fonts: document.fonts.status, viewport: [innerWidth, innerHeight],
    statuses: [...document.querySelectorAll('[role=status]')].map(describe), ancestors };
})()
