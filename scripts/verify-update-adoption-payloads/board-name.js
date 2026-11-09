(() => {
  // The real Board Inspector name field: the saved project's own content, not
  // just the generic demo board label.
  const field = document.querySelector('[aria-label="Board Inspector"] input[type=text]');
  return { boardName: field ? field.value : null,
           focusedInField: !!(field && document.activeElement === field),
           saved: Array.from(document.querySelectorAll('button')).some(
             (b) => /^Saved locally/.test((b.getAttribute('aria-label') || b.textContent || '').trim())) };
})()
