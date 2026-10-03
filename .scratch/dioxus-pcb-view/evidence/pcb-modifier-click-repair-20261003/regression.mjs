// Requires the c6 fixture mounted in PCB with SW1 selected. agent-browser owns
// the isolated session; CDP provides its unsupported trusted modifier click.
import { execFileSync } from 'node:child_process';
import { writeFileSync } from 'node:fs';
import assert from 'node:assert/strict';

const [browserSession, urlPrefix, receipt] = process.argv.slice(2);
const browser = (...args) => execFileSync('agent-browser', ['--session', browserSession, ...args], { encoding: 'utf8' }).trim();
const read = () => JSON.parse(JSON.parse(browser('eval', `JSON.stringify({selected:[...document.querySelectorAll('.m1-pcb-part[aria-pressed=true]')].map(e=>e.dataset.partId).sort(),box:(()=>{const r=document.querySelector('[data-part-id="matrix/left-keys/r0c1"]').getBoundingClientRect();return {x:r.x+r.width/2,y:r.y+r.height/2}})()})`)));
const before = read();
assert.deepEqual(before.selected, ['matrix/left-keys/r0c0'], 'Start with only SW1 selected');
const ws = new WebSocket(browser('get', 'cdp-url'));
await new Promise((resolve, reject) => { ws.addEventListener('open', resolve, { once: true }); ws.addEventListener('error', reject, { once: true }); });
let sequence = 0;
const pending = new Map();
ws.addEventListener('message', event => { const result = JSON.parse(event.data); const request = pending.get(result.id); if (request) { pending.delete(result.id); result.error ? request.reject(result.error) : request.resolve(result.result); } });
const send = (method, params = {}, sessionId) => new Promise((resolve, reject) => { const id = ++sequence; pending.set(id, { resolve, reject }); ws.send(JSON.stringify({ id, method, params, ...(sessionId ? { sessionId } : {}) })); });
try {
    const { targetInfos } = await send('Target.getTargets');
    const pages = targetInfos.filter(target => target.type === 'page' && target.url.startsWith(urlPrefix));
    assert.equal(pages.length, 1, 'Use one isolated page');
    const { sessionId } = await send('Target.attachToTarget', { targetId: pages[0].targetId, flatten: true });
    const { x, y } = before.box;
    await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y, modifiers: 2 }, sessionId);
    await send('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'left', buttons: 1, clickCount: 1, modifiers: 2 }, sessionId);
    await send('Input.dispatchMouseEvent', { type: 'mouseReleased', x, y, button: 'left', buttons: 0, clickCount: 1, modifiers: 2 }, sessionId);
    // A frame barrier lets the release and compatibility click both settle.
    await send('Runtime.evaluate', { expression: 'new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)))', awaitPromise: true }, sessionId);
    const after = read();
    const result = { url: pages[0].url, input: 'trusted Control-click SW2', before, after, recordedAt: new Date().toISOString() };
    if (receipt) writeFileSync(receipt, `${JSON.stringify(result, null, 2)}\n`);
    assert.deepEqual(after.selected, ['matrix/left-keys/r0c0', 'matrix/left-keys/r0c1'], 'Control-click SW2 must retain SW1 and add SW2');
    console.log('PASS: Control-click SW2 retains SW1 and adds SW2');
    await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: after.box.x, y: after.box.y, modifiers: 2 }, sessionId);
    await send('Input.dispatchMouseEvent', { type: 'mousePressed', x: after.box.x, y: after.box.y, button: 'left', buttons: 1, clickCount: 1, modifiers: 2 }, sessionId);
    await send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: after.box.x, y: after.box.y, button: 'left', buttons: 0, clickCount: 1, modifiers: 2 }, sessionId);
    await send('Runtime.evaluate', { expression: 'new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)))', awaitPromise: true }, sessionId);
    const toggled = read();
    if (receipt) writeFileSync(receipt, `${JSON.stringify({ ...result, toggle: toggled }, null, 2)}\n`);
    assert.deepEqual(toggled.selected, ['matrix/left-keys/r0c0'], 'Second Control-click SW2 must remove only SW2');
    console.log('PASS: second Control-click SW2 removes only SW2');
} finally { ws.close(); }
