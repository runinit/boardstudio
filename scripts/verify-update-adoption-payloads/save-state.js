(() => {
  // Save status via the production header labels (state.js only sees Local-save
  // prefixed labels, missing Saved/Saving). Never clicks, only reports.
  return {
    saved: !!document.querySelector('[aria-label="Saved locally"]'),
    saving: !!document.querySelector('[aria-label="Saving locally"]'),
    failed: !!document.querySelector('[aria-label="Local save failed"]'),
    unavailable: !!document.querySelector('[aria-label="Local save status unavailable"]'),
  };
})()
