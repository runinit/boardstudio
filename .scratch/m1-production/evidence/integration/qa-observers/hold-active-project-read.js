// Test-only observer injected by serve-startup-restore-qa.mjs.
// Delays one real IndexedDB projects.get(activeProjectId) success callback
// until window.__m1Qa.releaseActiveProjectRead() is called.
(() => {
  const key = Object.keys(localStorage).find((name) =>
    name.startsWith("boardstudio-m1-active-project:boardstudio-m1-"),
  );
  const activeProjectId = key ? localStorage.getItem(key) : null;
  const state = {
    activeProjectId,
    preferenceKey: key,
    held: false,
    seen: 0,
    released: 0,
    unsupported: null,
    requestSource: null,
    requestResultId: null,
  };
  const descriptor = Object.getOwnPropertyDescriptor(IDBRequest.prototype, "onsuccess");
  let releaseHandler = null;

  if (!descriptor || typeof descriptor.set !== "function") {
    state.unsupported = "IDBRequest.onsuccess is not an interceptable IDL setter in this browser";
  } else {
    Object.defineProperty(IDBRequest.prototype, "onsuccess", {
      configurable: descriptor.configurable,
      enumerable: descriptor.enumerable,
      get: descriptor.get,
      set(handler) {
        if (typeof handler !== "function") {
          descriptor.set.call(this, handler);
          return;
        }
        const request = this;
        descriptor.set.call(this, function (event) {
          const resultId = request.result && request.result.id;
          if (
            !state.held &&
            state.released === 0 &&
            request.source?.name === "projects" &&
            activeProjectId &&
            resultId === activeProjectId
          ) {
            state.held = true;
            state.seen += 1;
            state.requestSource = request.source.name;
            state.requestResultId = resultId;
            releaseHandler = () => handler.call(request, event);
            return;
          }
          handler.call(this, event);
        });
      },
    });
  }

  state.releaseActiveProjectRead = () => {
    if (!releaseHandler) return false;
    const resume = releaseHandler;
    releaseHandler = null;
    state.held = false;
    state.released += 1;
    resume();
    return true;
  };
  window.__m1Qa = state;
})();
