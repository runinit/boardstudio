const { performance } = require('node:perf_hooks');

async function readProjects(page, database, timeout) {
  let timer;
  try {
    return await Promise.race([
      page.evaluate(({ database, timeout }) => new Promise((resolve, reject) => {
        let settled = false;
        let db;
        const finish = (error, value) => {
          if (settled) return;
          settled = true;
          clearTimeout(timer);
          db?.close();
          if (error) reject(error); else resolve(value);
        };
        const timer = setTimeout(() => finish(Error('IndexedDB project read timed out')), timeout);
        let request;
        try { request = indexedDB.open(database); } catch (error) { finish(error); return; }
        request.onerror = () => finish(request.error);
        request.onblocked = () => finish(Error('IndexedDB project read was blocked'));
        request.onsuccess = () => {
          db = request.result;
          if (settled) { db.close(); return; }
          try {
            const query = db.transaction('projects').objectStore('projects').getAll();
            query.onerror = () => finish(query.error);
            query.onsuccess = () => finish(null, query.result);
          } catch (error) { finish(error); }
        };
      }), { database, timeout }),
      new Promise((_, reject) => {
        timer = setTimeout(() => reject(Error('IndexedDB project read timed out')), timeout);
      }),
    ]);
  } finally {
    clearTimeout(timer);
  }
}

async function waitForProjects(read, matches, timeout, message) {
  const deadline = performance.now() + timeout;
  for (;;) {
    const remaining = deadline - performance.now();
    if (remaining <= 0) throw Error(message);
    let projects;
    try {
      projects = await read(remaining);
    } catch (error) {
      if (error.message === 'IndexedDB project read timed out') throw Error(`${message}: ${error.message}`);
      throw error;
    }
    if (performance.now() >= deadline) throw Error(message);
    if (matches(projects)) return;
    await new Promise(resolve => setTimeout(resolve, Math.min(200, Math.max(0, deadline - performance.now()))));
  }
}

module.exports = { readProjects, waitForProjects };
