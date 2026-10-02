// Test-only observer. Records browser-visible anchor download attempts without
// changing the click or its navigation/download behavior.
(() => {
  if (globalThis.__m1DownloadClickProbe) return;
  const events = [];
  const original = HTMLAnchorElement.prototype.click;
  HTMLAnchorElement.prototype.click = function(...args) {
    events.push({
      at: performance.now(),
      download: this.download || null,
      hrefKind: this.href.startsWith('blob:') ? 'blob' : 'other',
      href: this.href,
    });
    return original.apply(this, args);
  };
  globalThis.__m1DownloadClickProbe = { events };
})();
