// The browser page helpers the packaged-provider regressions use: the `node:assert/strict`
// subset, the `node:fs` fixture reads and the `node:crypto` SHA-256 digest they used.
//
// The assert subset keeps Node's strict semantics: `equal` compares with `Object.is`, `deepEqual`
// is `deepStrictEqual`, `throws` catches a synchronous throw and `rejects` a rejected promise.
// `deepEqual` compares plain objects, arrays, typed arrays and sets; any other shape is refused
// instead of being matched against an arbitrary object.

class AssertionError extends Error {
  constructor(message, actual, expected, operator = 'assert') {
    super(`${message ?? 'assertion failed'}\ncode: ERR_ASSERTION\nactual: ${inspect(actual)}\nexpected: ${inspect(expected)}\noperator: ${operator}`);
    this.name = 'AssertionError';
    Object.assign(this, { code: 'ERR_ASSERTION', actual, expected, operator });
  }
}

// `ignoreBOM` keeps a byte order mark in the text, as Node's `Buffer.toString('utf8')` did.
const decoder = new TextDecoder('utf-8', { ignoreBOM: true });
const encoder = new TextEncoder();

/** The text of `bytes`, decoded as UTF-8. */
export const decodeText = bytes => decoder.decode(bytes);

/** The UTF-8 bytes of `text`. */
export const encodeText = text => encoder.encode(text);

/** The bytes of `value`, as `Buffer.from` produced them for a string or a typed array. */
export const toBytes = value => (typeof value === 'string' ? encodeText(value) : new Uint8Array(value));

function inspect(value) {
  try {
    if (Object.is(value, -0)) return '-0';
    if (typeof value === 'object' || typeof value === 'string') {
      try { return (JSON.stringify(value) ?? String(value)).slice(0, 2048); } catch { /* fall back */ }
    }
    return String(value).slice(0, 2048);
  } catch {
    return '[unprintable]';
  }
}

/** Node's `deepStrictEqual`, for the values the provider returns. */
function deepStrictEqual(actual, expected) {
  if (actual instanceof Map || expected instanceof Map) {
    throw new AssertionError('deepEqual compares plain objects, arrays, typed arrays and sets only');
  }
  if (Object.is(actual, expected)) return true;
  if (actual === null || expected === null) return false;
  if (typeof actual !== 'object' || typeof expected !== 'object') return false;
  if (Object.getPrototypeOf(actual) !== Object.getPrototypeOf(expected)) return false;
  if (ArrayBuffer.isView(actual) || ArrayBuffer.isView(expected)) return viewsEqual(actual, expected);
  if (Array.isArray(actual) !== Array.isArray(expected)) return false;
  if (Array.isArray(actual)) {
    // The length is part of the value: `Array(1)` is not `[]`, and its hole is not a key.
    if (actual.length !== expected.length) return false;
    return ownKeysEqual(actual, expected);
  }
  if (actual instanceof Set || expected instanceof Set) return setsEqual(actual, expected);
  if (!isPlainObject(actual)) {
    // A shape the helper cannot compare is refused, never matched against an arbitrary object.
    throw new AssertionError('deepEqual compares plain objects, arrays, typed arrays and sets only');
  }
  return ownKeysEqual(actual, expected);
}

/** Whether two typed arrays hold the same values, of the same kind. */
function viewsEqual(actual, expected) {
  if (!ArrayBuffer.isView(actual) || !ArrayBuffer.isView(expected)) return false;
  if (actual.constructor !== expected.constructor) return false;
  if (actual instanceof DataView || expected instanceof DataView) {
    if (actual.byteLength !== expected.byteLength) return false;
    const left = new Uint8Array(actual.buffer, actual.byteOffset, actual.byteLength);
    const right = new Uint8Array(expected.buffer, expected.byteOffset, expected.byteLength);
    return left.every((byte, index) => byte === right[index]);
  }
  if (actual.length !== expected.length) return false;
  for (let index = 0; index < actual.length; index++) {
    if (!Object.is(actual[index], expected[index])) return false;
  }
  return true;
}

/** Whether two sets hold the same values, in any order. */
function setsEqual(actual, expected) {
  if (!(actual instanceof Set) || !(expected instanceof Set)) return false;
  if (actual.size !== expected.size) return false;
  for (const value of actual) {
    if (![...expected].some(other => deepStrictEqual(value, other))) return false;
  }
  return true;
}

/** Whether two objects hold the same own enumerable keys and values. */
function ownKeysEqual(actual, expected) {
  const actualKeys = Object.keys(actual);
  const expectedKeys = Object.keys(expected);
  if (actualKeys.length !== expectedKeys.length) return false;
  for (const key of actualKeys) {
    if (!Object.prototype.hasOwnProperty.call(expected, key)) return false;
    if (!deepStrictEqual(actual[key], expected[key])) return false;
  }
  return true;
}

/** Whether `value` is a plain object, the only object shape compared by its own keys. */
function isPlainObject(value) {
  const prototype = Object.getPrototypeOf(value);
  return prototype === Object.prototype || prototype === null;
}

/** Checks the thrown value against the expectation Node's `assert` accepted. */
function checkException(actual, expected, message) {
  if (expected === undefined) return;
  if (typeof expected === 'function') {
    if (expected.prototype !== undefined && actual instanceof expected) return;
    if (expected.call({}, actual) !== true) {
      throw new AssertionError(message ?? 'The validation function is expected to return `true`.');
    }
    return;
  }
  if (expected instanceof RegExp) {
    if (!expected.test(String(actual))) {
      throw new AssertionError(message
        ?? `The input did not match the regular expression ${expected}. Input:\n\n${inspect(actual)}\n`);
    }
    return;
  }
  if (typeof expected === 'object' && expected !== null) {
    for (const key of Object.keys(expected)) {
      if (!deepStrictEqual(actual?.[key], expected[key])) {
        throw new AssertionError(message ?? `The error's ${key} did not match the expectation.`);
      }
    }
    return;
  }
  throw new AssertionError(message ?? 'The expectation is not a validation function, a regular expression or an object.');
}

/** The assert subset `node:assert/strict` provided, with the same strict and deep semantics. */
export function assert(value, message) {
  if (!value) {
    throw new AssertionError(message, value, true, '==');
  }
}

assert.ok = assert;

assert.equal = (actual, expected, message) => {
  if (!Object.is(actual, expected)) {
    throw new AssertionError(message, actual, expected, 'strictEqual');
  }
};

assert.notEqual = (actual, expected, message) => {
  if (Object.is(actual, expected)) {
    throw new AssertionError(message, actual, expected, 'notStrictEqual');
  }
};

assert.deepEqual = (actual, expected, message) => {
  if (!deepStrictEqual(actual, expected)) {
    throw new AssertionError(message, actual, expected, 'deepStrictEqual');
  }
};

assert.match = (value, regexp, message) => {
  if (typeof value !== 'string' || !regexp.test(value)) {
    throw new AssertionError(message
      ?? `The input did not match the regular expression ${regexp}.`, value, regexp, 'match');
  }
};

assert.throws = (body, expected, message) => {
  let thrown = false;
  let actual;
  try {
    body();
  } catch (error) {
    thrown = true;
    actual = error;
  }
  if (!thrown) throw new AssertionError(message ?? 'Missing expected exception.', actual, expected, 'throws');
  checkException(actual, expected, message);
};

assert.rejects = async (body, expected, message) => {
  let rejected = false;
  let actual;
  try {
    await (typeof body === 'function' ? body() : body);
  } catch (error) {
    rejected = true;
    actual = error;
  }
  if (!rejected) throw new AssertionError(message ?? 'Missing expected rejection.', actual, expected, 'rejects');
  checkException(actual, expected, message);
};

/** The bytes of the file at `path`, as `node:fs`'s `readFileSync` returned them. */
export async function readBytes(path) {
  const response = await fetch('/fixture/' + path);
  if (!response.ok) throw new Error(`fixture read failed: ${path} (${response.status}): ${await response.text()}`);
  const bytes = new Uint8Array(await response.arrayBuffer());
  assert.equal(await sha256Hex(bytes), response.headers.get('X-Fixture-SHA256'), 'fixture SHA-256 mismatch');
  return bytes;
}

/** The UTF-8 text of the file at `path`. */
export async function readText(path) {
  return decodeText(await readBytes(path));
}

/** The lowercase hex SHA-256 of `bytes`, as `crypto.createHash('sha256')` produced it. */
export async function sha256Hex(bytes) {
  const digest = await crypto.subtle.digest('SHA-256', bytes);
  return Array.from(new Uint8Array(digest), byte => byte.toString(16).padStart(2, '0')).join('');
}

/** Run every named case, collecting failures without stopping the remaining cases. */
export async function runCases(suite, cases) {
  const results = [];
  for (const [name, fn] of cases) {
    try {
      await fn();
      results.push({ suite, case: name, ok: true });
    } catch (error) {
      let diagnostic;
      try { diagnostic = String(error?.stack ?? error); } catch { diagnostic = '[unprintable thrown value]'; }
      results.push({ suite, case: name, ok: false, error: diagnostic });
    }
  }
  return results;
}
