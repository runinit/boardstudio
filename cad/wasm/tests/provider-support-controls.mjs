import { assert, runCases } from './provider-support.mjs';

// Independent witnesses must not depend on the assertion helper being tested.
function mustFail(fn, check = () => true) {
  let failed = false;
  try { fn(); } catch (error) { failed = true; if (!check(error)) throw error; }
  if (!failed) throw Error('negative control did not fail');
}

export async function run() {
  return runCases('provider-support-controls', [
    ['truth', () => { assert(true); mustFail(() => assert(false)); }],
    ['equal', () => {
      assert.equal(NaN, NaN);
      mustFail(() => assert.equal(0, -0), error => error.name === 'AssertionError' &&
        error.code === 'ERR_ASSERTION' && Object.is(error.actual, 0) && Object.is(error.expected, -0) &&
        error.operator === 'strictEqual' && error.message.includes('expected: -0'));
      const unprintable = Object.create(null);
      unprintable.self = unprintable;
      mustFail(() => assert.equal(unprintable, 1), error => error.name === 'AssertionError' &&
        error.message.includes('[unprintable]'));
      mustFail(() => assert.notEqual(1, 1));
    }],
    ['deepEqual', () => {
      assert.deepEqual(new Set([1, 2]), new Set([2, 1]));
      assert.deepEqual(new Uint8Array([1]), new Uint8Array([1]));
      mustFail(() => assert.deepEqual(new Uint8Array([1]), new Uint8Array([2])));
      mustFail(() => assert.deepEqual([,], [undefined]));
      mustFail(() => assert.deepEqual(Array(1), []));
      mustFail(() => assert.deepEqual(new Map(), new Map()));
      const map = new Map();
      mustFail(() => assert.deepEqual(map, map));
    }],
    ['match', () => {
      assert.match('abc', /b/);
      mustFail(() => assert.match('abc', /z/));
      mustFail(() => assert.match(123, /^\d+$/));
    }],
    ['throws', () => {
      let ran = false, validated = false;
      assert.throws(() => { ran = true; throw Error('boom'); }, error => { validated = true; return error.message === 'boom'; });
      if (!ran || !validated) throw Error('exception body or validator did not run');
      mustFail(() => assert.throws(() => {}));
      mustFail(() => assert.throws(() => { throw Error('wrong'); }, /boom/));
    }],
    ['rejects', async () => {
      let validated = false;
      await assert.rejects(() => { throw Error('boom'); }, error => { validated = true; return error.message === 'boom'; });
      if (!validated) throw Error('rejection validator did not run');
      let failed = false;
      try { await assert.rejects(Promise.resolve()); } catch (_) { failed = true; }
      if (!failed) throw Error('missing rejection was accepted');
      failed = false;
      try { await assert.rejects(Promise.reject(Error('wrong')), /boom/); } catch (_) { failed = true; }
      if (!failed) throw Error('mismatched rejection was accepted');
      let laterRan = false;
      const cases = await runCases('inner', [['failure', () => { throw Error('inner failure'); }],
        ['later', () => { laterRan = true; }]]);
      if (!laterRan || cases.length !== 2 || cases[0].ok !== false || cases[1].ok !== true) {
        throw Error('runCases did not continue after failure');
      }
    }],
  ]);
}
