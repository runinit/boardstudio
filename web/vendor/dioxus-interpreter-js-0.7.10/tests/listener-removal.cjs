// Exercise the shipped interpreter and binding statements; this is a Node unit test, not a DOM browser test.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const { test } = require('node:test');

const source = fs.readFileSync(path.join(__dirname, '../src/js/core.js'), 'utf8');
const bindings = fs.readFileSync(path.join(__dirname, '../src/unified_bindings.rs'), 'utf8');

function statement(name) {
  const match = bindings.match(new RegExp('fn ' + name + '[^\\{]*\\{\\s*(?:r#"([\\s\\S]*?)"#|("(?:[^"\\\\]|\\\\.)*"))'));
  assert.ok(match, 'binding statement exists: ' + name);
  return (match[1] ?? JSON.parse(match[2]))
    .replaceAll('$id$', 'id').replaceAll('$event_name$', 'name').replaceAll('$bubbles$', 'bubbles')
    .replaceAll('\\' + '$' + '{id}', '$' + '{id}');
}

class Element extends EventTarget {
  constructor() { super(); this.attributes = new Map(); }
  setAttribute(name, value) { this.attributes.set(name, String(value)); }
  getAttribute(name) { return this.attributes.get(name) ?? null; }
  removeAttribute(name) { this.attributes.delete(name); }
}

function fixture() {
  const exportPattern = /export\s*\{\s*BaseInterpreter\s*\};?\s*$/;
  assert.match(source, exportPattern);
  const context = vm.createContext({ Event, EventTarget });
  vm.runInContext(source.replace(exportPattern, 'this.BaseInterpreter = BaseInterpreter;'), context);
  const interpreter = new context.BaseInterpreter();
  const root = new Element();
  const seen = [];
  interpreter.initialize(root, event => seen.push([event.type, event.target.getAttribute('data-dioxus-id')]));
  const first = new Element();
  const second = new Element();
  interpreter.nodes[7] = first;
  interpreter.nodes[8] = second;
  const add = new Function('name', 'id', 'bubbles', statement('new_event_listener'));
  const remove = new Function('name', 'id', 'bubbles', statement('remove_event_listener'));
  return {
    interpreter, root, first, second, seen,
    add: (name, id = 7, bubbles = false) => add.call(interpreter, name, id, bubbles),
    remove: (name, id = 7, bubbles = false) => remove.call(interpreter, name, id, bubbles),
  };
}

test('removing one local callback preserves blur routing; final removal and re-add stay exact', () => {
  const f = fixture();
  f.add('focus'); f.add('blur');
  f.first.dispatchEvent(new Event('focus')); f.first.dispatchEvent(new Event('blur'));
  assert.deepEqual(f.seen, [['focus', '7'], ['blur', '7']]);
  f.remove('focus');
  assert.equal(f.first.listening, 1);
  assert.equal(f.first.getAttribute('data-dioxus-id'), '7');
  assert.deepEqual(Object.keys(f.interpreter.local['7']), ['blur']);
  f.first.dispatchEvent(new Event('focus')); f.first.dispatchEvent(new Event('blur'));
  assert.deepEqual(f.seen, [['focus', '7'], ['blur', '7'], ['blur', '7']]);
  f.remove('blur');
  assert.equal(f.first.listening, 0);
  assert.equal(f.first.getAttribute('data-dioxus-id'), null);
  assert.equal(f.interpreter.local['7'], undefined);
  f.add('focus'); f.first.dispatchEvent(new Event('focus'));
  assert.deepEqual(f.seen, [['focus', '7'], ['blur', '7'], ['blur', '7'], ['focus', '7']]);
  f.remove('focus'); f.first.dispatchEvent(new Event('focus'));
  assert.equal(f.seen.length, 4);
});

test('mixed local/global removal retains other node subscriptions and last-handler identity', () => {
  const f = fixture();
  f.add('focus'); f.add('blur'); f.add('click', 7, true); f.add('click', 8, true);
  assert.equal(f.interpreter.global.click.active, 2);
  f.remove('focus'); f.remove('blur');
  assert.equal(f.first.getAttribute('data-dioxus-id'), '7');
  assert.equal(f.first.listening, 1);
  assert.equal(f.interpreter.local['7'], undefined);
  f.remove('click', 7, true);
  assert.equal(f.first.getAttribute('data-dioxus-id'), null);
  assert.equal(f.second.getAttribute('data-dioxus-id'), '8');
  assert.equal(f.interpreter.global.click.active, 1);
  f.root.dispatchEvent(new Event('click'));
  assert.deepEqual(f.seen, [['click', '0']]);
  f.remove('click', 8, true); f.root.dispatchEvent(new Event('click'));
  assert.equal(f.interpreter.global.click, undefined);
  assert.equal(f.second.getAttribute('data-dioxus-id'), null);
  assert.equal(f.seen.length, 1);
});

test('hydration shares registration, partial removal and terminal cleanup invariants', () => {
  const f = fixture();
  f.first.setAttribute('data-node-hydration', '42,focus:0,blur:0,click:1');
  f.interpreter.hydrate_node(f.first, { 42: 7 });
  assert.equal(f.first.listening, 3);
  f.remove('focus'); f.first.dispatchEvent(new Event('blur'));
  assert.deepEqual(f.seen, [['blur', '7']]);
  assert.equal(f.first.getAttribute('data-dioxus-id'), '7');
  f.remove('blur'); f.remove('click', 7, true);
  assert.equal(f.first.listening, 0);
  assert.equal(f.first.getAttribute('data-dioxus-id'), null);
  assert.equal(f.interpreter.local['7'], undefined);
  assert.equal(f.interpreter.global.click, undefined);
});

test('existing element teardown clears its local bookkeeping without affecting another node', () => {
  const f = fixture();
  f.add('focus'); f.add('blur'); f.add('focus', 8);
  assert.deepEqual(Object.keys(f.interpreter.local['7']).sort(), ['blur', 'focus']);
  f.interpreter.removeAllNonBubblingListeners(f.first);
  assert.equal(f.interpreter.local['7'], undefined);
  assert.deepEqual(Object.keys(f.interpreter.local['8']), ['focus']);
});

test('initially absent callbacks install no listeners or routing id', () => {
  const f = fixture();
  f.first.dispatchEvent(new Event('focus')); f.first.dispatchEvent(new Event('blur'));
  assert.equal(f.first.getAttribute('data-dioxus-id'), null);
  assert.deepEqual(f.seen, []);
  assert.deepEqual(Object.keys(f.interpreter.local), []);
  assert.deepEqual(Object.keys(f.interpreter.global), []);
});
