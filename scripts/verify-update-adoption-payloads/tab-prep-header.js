(() => {
  // Real Tab order for the HEADER Update Now: focus its predecessor so ONE
  // trusted Tab must land on it. Mirrors tab-prep.js (toast) for the header
  // entry point after toast dismissal.
  const sel = 'a[href],button:not([disabled]),input:not([disabled]):not([type=hidden]),select:not([disabled]),textarea:not([disabled]),[tabindex]:not([tabindex="-1"])';
  const visible = (e) => {
    const r = e.getBoundingClientRect();
    const cs = getComputedStyle(e);
    return r.width > 0 && r.height > 0 && cs.visibility !== 'hidden' && cs.display !== 'none';
  };
  const list = Array.from(document.querySelectorAll(sel)).filter(visible);
  const positive = list.filter((e) => e.tabIndex > 0).length;
  const target = document.querySelector(".header-update [aria-label='Update Now']");
  const index = list.indexOf(target);
  if (!target || index < 1) return { found: false, index, total: list.length, positive };
  const before = list[index - 1];
  before.focus();
  return { found: true, index, total: list.length, positive,
           predecessor: (before.getAttribute('aria-label') || before.textContent || before.tagName).trim().slice(0, 60) };
})()
