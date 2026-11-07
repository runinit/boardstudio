(() => {
  // Read the real standalone-part inspector, including its keep-mounted form.
  // Never constructs UI, changes values, or seeds a registry owner.
  const scope = document.querySelector('[aria-label="Component inspector"]');
  const trigger = scope?.querySelector('[aria-label="Layout constraint"]');
  if (!trigger) return {present: false};
  const value = label => scope.querySelector(`[aria-label="${label}"]`)?.value;
  return {
    present: true,
    expanded: trigger.getAttribute('aria-expanded') === 'true',
    x: value('Offset X (mm)'),
    y: value('Offset Y (mm)'),
    invalid: scope.querySelectorAll('[aria-invalid="true"], :invalid').length,
    saveLabel: Array.from(scope.querySelectorAll('button'))
      .map(b => b.textContent.trim()).find(t => /^(Save|Add) constraint$/.test(t)) || null,
  };
})()
