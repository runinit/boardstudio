(() => {
  // Layout constraint form presence and values (minimal; refined once the
  // packaged demo selection yielding the form is confirmed).
  const trigger = document.querySelector('[aria-label="Layout constraint"]');
  if (!trigger) return {present:false};
  const root = trigger.closest('[data-slot]')?.parentElement || trigger.parentElement || document;
  const scope = root.parentElement || document;
  const inputs = Array.from(scope.querySelectorAll('input')).map((i) => ({
    label: i.getAttribute('aria-label'), value: i.value,
  })).filter((e) => e.label && /Offset|Mirror|Angle|Margin|X \(mm\)|Y \(mm\)/i.test(e.label));
  const buttons = Array.from(scope.querySelectorAll('button'))
    .map((b) => (b.getAttribute('aria-label') || b.textContent.trim()).slice(0, 40))
    .filter((t) => /Save constraint|Add constraint|Remove/i.test(t));
  return {present:true, inputs, buttons,
          saveLabel: buttons.find((t) => /Save|Add/.test(t)) || null};
})()
