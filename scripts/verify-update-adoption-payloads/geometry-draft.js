(() => {
  // Real Geometry scripts editor values: the saved project's own content.
  // Shape mirrors the 002 probe's geo_values (Name/Rhai/focus/invalid).
  const s = document.querySelector('[aria-label="Geometry scripts"]');
  if (!s) return {open:false};
  const name = s.querySelector('input[type=text]');
  const src = s.querySelector('textarea');
  const a = document.activeElement;
  const apply = Array.from(s.querySelectorAll('button')).find((b) => b.textContent.trim() === 'Apply script');
  return {
    open:true,
    name: name?name.value:null,
    source: src?src.value:null,
    active: a?(a.getAttribute('aria-label')||a.tagName):null,
    invalid: document.querySelectorAll('[aria-invalid="true"]').length,
    applyDisabled: apply?apply.disabled:null,
  };
})()
