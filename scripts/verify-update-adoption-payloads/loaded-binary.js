(async () => {
  // Identity of the application BINARY served to this page load: the packaged
  // wasm carries a trailing custom section "boardstudio-build"+<label>. Read it
  // through the controlling worker, and report whether this document's own
  // load requested that URL. (No web API exposes the executing module's bytes.)
  const reg = await navigator.serviceWorker.getRegistration();
  // The path placeholder is rendered driver-side (see loaded_binary_script).
  const url = reg.scope + '__BOARDSTUDIO_PAGE_WASM__';
  const requested = performance.getEntriesByType('resource').some((e) => e.name === url);
  const resp = await fetch(url);
  const bytes = new Uint8Array(await resp.arrayBuffer());
  const tail = new TextDecoder('latin1').decode(bytes.slice(Math.max(0, bytes.length - 40)));
  const m = tail.match(/boardstudio-build([A-Z])$/);
  return { url, requestedByThisDocument: requested, status: resp.status, size: bytes.length,
           wasmBuild: m ? m[1] : null };
})()
