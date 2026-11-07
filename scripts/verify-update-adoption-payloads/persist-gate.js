(() => {
  // Diagnostic gate snapshot (diagnostic builds only). Missing optional fields
  // (armed/document/revision when None) are absent from the JSON, not null.
  const g = window.__boardstudioPersistGate;
  if (!g) return {present:false};
  try {
    const s = g.snapshot();
    return {present:true, ...s};
  } catch (e) {
    return {present:true, error:String(e).slice(0,200)};
  }
})()
