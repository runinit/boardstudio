// QA-only page init script. Pass with agent-browser open --init-script <path>
// before the first page navigation. It captures public page lifecycle APIs and
// writes no files, fetches nothing, and changes no application state.
(() => {
  if (globalThis.__m1ResourceProbe) return;

  const data = {
    workers: { created: 0, terminated: 0, nextId: 1, active: new Map(), byScript: {} },
    animationFrames: { requested: 0, fired: 0, cancelled: 0, active: new Set() },
    listeners: { added: 0, removed: 0, active: new Map() },
    observers: {},
    objectUrls: { created: 0, revoked: 0, active: new Map() },
    workerReplies: [],
    events: []
  };
  const bucket = (map, key) => {
    const current = map.get(key) ?? 0;
    map.set(key, current);
    return current;
  };
  const bump = (map, key, delta) => {
    const next = bucket(map, key) + delta;
    if (next) map.set(key, next);
    else map.delete(key);
  };
  const targetKind = target => {
    if (target === globalThis) return "window";
    if (target === document) return "document";
    if (target instanceof Worker) return "worker";
    if (target instanceof MediaQueryList) return "media-query";
    if (target instanceof Node) return target.nodeName.toLowerCase();
    return target?.constructor?.name ?? "event-target";
  };

  // Count effective registrations and explicit removals. Weak collections avoid
  // retaining detached nodes or callback functions solely for this probe.
  const targetListeners = new WeakMap();
  const listenerIds = new WeakMap();
  let nextListenerId = 1;
  const nativeAdd = EventTarget.prototype.addEventListener;
  const nativeRemove = EventTarget.prototype.removeEventListener;
  EventTarget.prototype.addEventListener = function(type, listener, options) {
    if (listener) {
      const capture = typeof options === "boolean" ? options : !!options?.capture;
      let byKey = targetListeners.get(this);
      if (!byKey) targetListeners.set(this, byKey = new Map());
      const key = `${type}:${capture}`;
      let seen = byKey.get(key);
      if (!seen) byKey.set(key, seen = new WeakSet());
      if (!seen.has(listener)) {
        seen.add(listener);
        if (!listenerIds.has(listener)) listenerIds.set(listener, nextListenerId++);
        data.listeners.added++;
        bump(data.listeners.active, `${targetKind(this)}:${type}`, 1);
      }
    }
    return nativeAdd.call(this, type, listener, options);
  };
  EventTarget.prototype.removeEventListener = function(type, listener, options) {
    if (listener) {
      const capture = typeof options === "boolean" ? options : !!options?.capture;
      const key = `${type}:${capture}`;
      const seen = targetListeners.get(this)?.get(key);
      if (seen?.has(listener)) {
        seen.delete(listener);
        data.listeners.removed++;
        bump(data.listeners.active, `${targetKind(this)}:${type}`, -1);
      }
    }
    return nativeRemove.call(this, type, listener, options);
  };

  // Instrument page-owned Worker constructors and explicit termination.
  const NativeWorker = globalThis.Worker;
  const workerIds = new WeakMap();
  const nativeTerminate = NativeWorker.prototype.terminate;
  NativeWorker.prototype.terminate = function(...args) {
    const id = workerIds.get(this);
    if (id && data.workers.active.has(id)) {
      const entry = data.workers.active.get(id);
      entry.terminateCalls++;
      if (entry.terminateCalls === 1) data.workers.terminated++;
      data.workers.active.delete(id);
    }
    return nativeTerminate.apply(this, args);
  };
  globalThis.Worker = new Proxy(NativeWorker, {
    construct(target, args, newTarget) {
      const worker = Reflect.construct(target, args, newTarget);
      const id = data.workers.nextId++;
      const script = String(args[0] ?? "");
      const type = args[1]?.type ?? "classic";
      workerIds.set(worker, id);
      data.workers.created++;
      data.workers.active.set(id, { id, script, type, terminateCalls: 0 });
      data.workers.byScript[script] = (data.workers.byScript[script] ?? 0) + 1;
      return worker;
    }
  });

  // Count pending one-shot animation frames and their drain/cancel outcomes.
  const nativeRaf = globalThis.requestAnimationFrame.bind(globalThis);
  const nativeCancelRaf = globalThis.cancelAnimationFrame.bind(globalThis);
  const rafIds = new WeakMap();
  globalThis.requestAnimationFrame = callback => {
    const holder = {};
    const id = nativeRaf(time => {
      data.animationFrames.active.delete(holder.id);
      data.animationFrames.fired++;
      callback(time);
    });
    holder.id = id;
    data.animationFrames.active.add(id);
    data.animationFrames.requested++;
    return id;
  };
  globalThis.cancelAnimationFrame = id => {
    if (data.animationFrames.active.delete(id)) data.animationFrames.cancelled++;
    return nativeCancelRaf(id);
  };

  // Track observer constructors and their observe/unobserve/disconnect lifecycle.
  const observerState = new WeakMap();
  for (const name of ["ResizeObserver", "MutationObserver", "IntersectionObserver", "PerformanceObserver"]) {
    const Native = globalThis[name];
    if (!Native) continue;
    data.observers[name] = { created: 0, observeCalls: 0, unobserveCalls: 0, disconnectCalls: 0, activeTargets: 0 };
    for (const method of ["observe", "unobserve", "disconnect"]) {
      const original = Native.prototype[method];
      if (typeof original !== "function") continue;
      Native.prototype[method] = function(target, ...args) {
        const state = observerState.get(this);
        const stats = data.observers[name];
        if (method === "observe") {
          stats.observeCalls++;
          if (target && !state.targets.has(target)) {
            state.targets.add(target);
            stats.activeTargets++;
          }
        } else if (method === "unobserve") {
          stats.unobserveCalls++;
          if (target && state.targets.delete(target)) stats.activeTargets--;
        } else {
          stats.disconnectCalls++;
          stats.activeTargets -= state.targets.size;
          state.targets.clear();
        }
        return original.call(this, target, ...args);
      };
    }
    globalThis[name] = new Proxy(Native, {
      construct(target, args, newTarget) {
        const instance = Reflect.construct(target, args, newTarget);
        observerState.set(instance, { targets: new Set() });
        data.observers[name].created++;
        return instance;
      }
    });
  }

  // Track temporary object URLs without retaining their Blob objects.
  const nativeCreateObjectURL = URL.createObjectURL.bind(URL);
  const nativeRevokeObjectURL = URL.revokeObjectURL.bind(URL);
  URL.createObjectURL = blob => {
    const url = nativeCreateObjectURL(blob);
    data.objectUrls.created++;
    data.objectUrls.active.set(url, { bytes: blob?.size ?? null, type: blob?.type ?? "" });
    return url;
  };
  URL.revokeObjectURL = url => {
    if (data.objectUrls.active.delete(String(url))) data.objectUrls.revoked++;
    return nativeRevokeObjectURL(url);
  };

  // Observe page-worker replies, including compact body mesh summaries for an
  // independent STEP readback comparison. No raw vertex arrays are retained.
  const onmessage = Object.getOwnPropertyDescriptor(NativeWorker.prototype, "onmessage");
  if (onmessage?.set && onmessage?.get) {
    Object.defineProperty(NativeWorker.prototype, "onmessage", {
      configurable: true,
      enumerable: onmessage.enumerable,
      get() { return onmessage.get.call(this); },
      set(handler) {
        if (typeof handler !== "function") return onmessage.set.call(this, handler);
        const wrapped = function(event) {
          const message = event.data;
          if (message?.kind === "reply") {
            let frame = {};
            try { frame = JSON.parse(message.frame ?? "{}"); } catch {}
            const result = message.result ?? {};
            const meshVolume = positions => {
              if (!(positions instanceof Float32Array) || positions.length % 9 !== 0) return null;
              let volume = 0;
              for (let i = 0; i < positions.length; i += 9) {
                const ax = positions[i], ay = positions[i + 1], az = positions[i + 2];
                const bx = positions[i + 3], by = positions[i + 4], bz = positions[i + 5];
                const cx = positions[i + 6], cy = positions[i + 7], cz = positions[i + 8];
                volume += (ax * (by * cz - bz * cy) + ay * (bz * cx - bx * cz) + az * (bx * cy - by * cx)) / 6;
              }
              return Math.abs(volume);
            };
            const bodies = Array.isArray(result.bodies) ? result.bodies.map(body => ({
              id: body.id ?? null,
              positionsFloat32: body.positions instanceof Float32Array ? body.positions.length : null,
              meshVolumeMm3: meshVolume(body.positions)
            })) : null;
            data.workerReplies.push({
              at: performance.now(), jobId: frame.jobId ?? null, requestId: frame.requestId ?? null,
              operation: frame.operation ?? null, outcome: frame.outcome ?? null,
              resultKeys: Object.keys(result), stepBytes: result.step?.byteLength ?? null,
              bodyCount: bodies?.length ?? null, bodies
            });
          }
          return handler.call(this, event);
        };
        return onmessage.set.call(this, wrapped);
      }
    });
  }

  globalThis.__m1ResourceProbe = {
    mark(label) {
      const listeners = Object.fromEntries(data.listeners.active);
      const observers = Object.fromEntries(Object.entries(data.observers).map(([name, stats]) => [name, { ...stats }]));
      return {
        label,
        at: performance.now(),
        workers: {
          created: data.workers.created,
          terminated: data.workers.terminated,
          live: [...data.workers.active.values()].map(worker => ({ ...worker })),
          createdByScript: { ...data.workers.byScript }
        },
        animationFrames: {
          requested: data.animationFrames.requested,
          fired: data.animationFrames.fired,
          cancelled: data.animationFrames.cancelled,
          pending: [...data.animationFrames.active]
        },
        listeners: {
          added: data.listeners.added,
          removed: data.listeners.removed,
          trackedActive: listeners
        },
        observers,
        objectUrls: {
          created: data.objectUrls.created,
          revoked: data.objectUrls.revoked,
          active: [...data.objectUrls.active].map(([url, detail]) => ({ url, ...detail }))
        },
        workerReplies: data.workerReplies.slice(),
        events: data.events.slice()
      };
    },
    events: data.events
  };
})();
