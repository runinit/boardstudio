// Coordinate-specific native wheel probe for an agent-browser-owned session.
// Usage: node wheel-probe.mjs <CDP port> <blocked|scroll> [output.json]
import {writeFileSync} from 'node:fs';
const [port, expected, output] = process.argv.slice(2);
const tabs = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
const tab = tabs.find(tab => tab.type === 'page' && tab.url.includes('/boardstudio/'));
if (!tab) throw new Error('Open the BoardStudio fixture in the owned browser session first');
const socket = new WebSocket(tab.webSocketDebuggerUrl);
await new Promise(resolve => socket.addEventListener('open', resolve, {once:true}));
let sequence = 0;
const send = (method, params = {}) => new Promise((resolve,reject) => {
  const id = ++sequence;
  const onMessage = event => {
    const message = JSON.parse(event.data);
    if (message.id !== id) return;
    socket.removeEventListener('message', onMessage);
    if (message.error) reject(new Error(JSON.stringify(message.error))); else resolve(message.result);
  };
  socket.addEventListener('message', onMessage);
  socket.send(JSON.stringify({id,method,params}));
});
const measure = async () => {
  const result = await send('Runtime.evaluate',{expression:`JSON.stringify({url:location.href,viewport:[innerWidth,innerHeight],elements:[...document.querySelectorAll('.m1-editor-body,.m1-inspector-slot .m1-panel-content,.m1-inspector-body,.m1-keycaps-inspector,.m1-workspace-content,.m1-object-slot')].map(e=>({selector:e.className,height:e.clientHeight,scrollHeight:e.scrollHeight,top:e.scrollTop,overflow:getComputedStyle(e).overflowY,rect:e.getBoundingClientRect().toJSON()}))})`,returnByValue:true});
  return JSON.parse(result.result.value);
};
const before = await measure();
const panel = before.elements.find(e=>e.selector==='m1-panel-content').rect;
const x = panel.left + panel.width/2, y = Math.min(panel.bottom-30,panel.top+200);
await send('Input.dispatchMouseEvent',{type:'mouseWheel',x,y,deltaY:1400,deltaX:0});
await new Promise(resolve=>setTimeout(resolve,250));
const after = await measure();
const moved = after.elements.some((e,i)=>e.top>before.elements[i].top && (e.selector==='m1-inspector-body'||e.selector==='m1-keycaps-inspector'));
const report={expected,before,after,nativeWheel:{x,y,deltaY:1400},moved};
if(output) writeFileSync(output,JSON.stringify(report,null,2)+'\n');
console.log(JSON.stringify(report));
socket.close();
if ((expected==='scroll')!==moved) process.exitCode=1;
