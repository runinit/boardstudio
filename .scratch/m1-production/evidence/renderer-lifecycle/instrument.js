(() => {
  const out = window.__rendererRace = {
    getContextCalls: [], canvasListenerAdds: [], canvasListenerRemoves: [],
    windowResizeAdds: 0, windowResizeRemoves: 0, observers: [],
    rafScheduled: [], rafCancelled: [],
  };
  const caseCanvas = (target) => target instanceof HTMLCanvasElement &&
    target.getAttribute('aria-label') === 'Generated case assembly; use camera controls to inspect';
  const originalGetContext = HTMLCanvasElement.prototype.getContext;
  HTMLCanvasElement.prototype.getContext = function(...args) {
    if (caseCanvas(this)) out.getContextCalls.push({ type: args[0], connected: this.isConnected, at: performance.now() });
    return originalGetContext.apply(this, args);
  };
  const originalAdd = EventTarget.prototype.addEventListener;
  const originalRemove = EventTarget.prototype.removeEventListener;
  EventTarget.prototype.addEventListener = function(type, listener, options) {
    if (caseCanvas(this)) out.canvasListenerAdds.push({ type, at: performance.now() });
    if (this === window && type === 'resize') out.windowResizeAdds++;
    return originalAdd.call(this, type, listener, options);
  };
  EventTarget.prototype.removeEventListener = function(type, listener, options) {
    if (caseCanvas(this)) out.canvasListenerRemoves.push({ type, at: performance.now() });
    if (this === window && type === 'resize') out.windowResizeRemoves++;
    return originalRemove.call(this, type, listener, options);
  };
  const NativeRO = window.ResizeObserver;
  window.ResizeObserver = class extends NativeRO {
    constructor(callback) { super(callback); this.__rrid = out.observers.length; out.observers.push({ id: this.__rrid, observes: 0, disconnects: 0 }); }
    observe(target, options) { out.observers[this.__rrid].observes++; return super.observe(target, options); }
    disconnect() { out.observers[this.__rrid].disconnects++; return super.disconnect(); }
  };
  const originalRaf = window.requestAnimationFrame.bind(window);
  const originalCancel = window.cancelAnimationFrame.bind(window);
  window.requestAnimationFrame = (callback) => { const id = originalRaf(callback); out.rafScheduled.push({ id, at: performance.now() }); return id; };
  window.cancelAnimationFrame = (id) => { out.rafCancelled.push({ id, at: performance.now() }); return originalCancel(id); };
})();
