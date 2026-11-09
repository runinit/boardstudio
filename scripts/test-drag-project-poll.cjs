const assert = require('node:assert/strict');
const { test } = require('node:test');
const { performance } = require('node:perf_hooks');
const { readProjects, waitForProjects } = require('./drag-project-poll.cjs');

test('a stalled page read cannot exceed the polling deadline', async () => {
  const page = { evaluate: () => new Promise(() => {}) };
  const start = performance.now();
  await assert.rejects(waitForProjects(ms => readProjects(page, 'projects', ms), () => false, 40, 'no match'),
    /no match: IndexedDB project read timed out/);
  assert.ok(performance.now() - start < 1000);
});

test('timed-out reads close a database that opens later', async () => {
  let request;
  let closed = 0;
  const previous = global.indexedDB;
  global.indexedDB = { open: () => (request = {}) };
  try {
    const page = { evaluate: (fn, arg) => fn(arg) };
    await assert.rejects(readProjects(page, 'projects', 20), /timed out/);
    request.result = { close: () => closed++ };
    request.onsuccess();
    assert.equal(closed, 1);
  } finally {
    global.indexedDB = previous;
  }
});

test('polling finds delayed changes and times out without a match', async () => {
  let count = 0;
  await waitForProjects(async () => [{ revision: ++count }], projects => projects[0].revision === 2, 1000, 'no match');
  assert.equal(count, 2);
  const start = performance.now();
  await assert.rejects(waitForProjects(async () => [], () => false, 40, 'no match'), /no match/);
  assert.ok(performance.now() - start < 1000);
});

test('blocked opens reject and close a database that opens later', async () => {
  let request;
  let closed = 0;
  const previous = global.indexedDB;
  global.indexedDB = { open: () => (request = {}) };
  try {
    const page = { evaluate: (fn, arg) => fn(arg) };
    const pending = readProjects(page, 'projects', 1000);
    request.onblocked();
    await assert.rejects(pending, /blocked/);
    request.result = { close: () => closed++ };
    request.onsuccess();
    assert.equal(closed, 1);
  } finally {
    global.indexedDB = previous;
  }
});
