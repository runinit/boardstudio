import {execFileSync} from 'node:child_process';
import {writeFileSync} from 'node:fs';
const [session,out]=process.argv.slice(2);
const url=execFileSync('agent-browser',['--session',session,'get','cdp-url'],{encoding:'utf8'}).trim();
const ws=new WebSocket(url); await new Promise(r=>ws.onopen=r);
let seq=0; const pending=new Map(), events=[];
function send(method,params={},sessionId){return new Promise((resolve,reject)=>{let id=++seq; pending.set(id,{resolve,reject}); ws.send(JSON.stringify({id,method,params,...(sessionId?{sessionId}:{})}));});}
ws.onmessage=async e=>{const d=JSON.parse(e.data); if(d.id){const p=pending.get(d.id); pending.delete(d.id); if(d.error)p?.reject(d.error);else p?.resolve(d.result);return;}
 if(d.method==='Target.attachedToTarget') {const s=d.params.sessionId; events.push(d); await send('Runtime.enable',{},s).catch(()=>{}); await send('Log.enable',{},s).catch(()=>{}); await send('Network.enable',{},s).catch(()=>{}); await send('Runtime.runIfWaitingForDebugger',{},s).catch(()=>{});}
 if(/ServiceWorker|exceptionThrown|entryAdded|consoleAPICalled|loadingFailed|responseReceived|requestWillBeSent/.test(d.method||'')){events.push(d);console.log(JSON.stringify(d));}
};
await send('Target.setAutoAttach',{autoAttach:true,waitForDebuggerOnStart:true,flatten:true});
const targets=await send('Target.getTargets');
const page=targets.targetInfos.find(x=>x.type==='page'&&x.url.includes('127.0.0.1'));
const {sessionId}=await send('Target.attachToTarget',{targetId:page.targetId,flatten:true});
await send('ServiceWorker.enable',{},sessionId); await send('Network.enable',{},sessionId); await send('Runtime.enable',{},sessionId);
await send('Page.reload',{},sessionId);
await new Promise(r=>setTimeout(r,18000));
const status=await send('Runtime.evaluate',{expression:'(async()=>({url:location.href,registrations:(await navigator.serviceWorker.getRegistrations()).map(r=>({scope:r.scope,active:r.active?.state})),controller:!!navigator.serviceWorker.controller,caches:await caches.keys()}))()',awaitPromise:true,returnByValue:true},sessionId);
writeFileSync(out,JSON.stringify({events,status},null,2));console.log(JSON.stringify(status)); ws.close();
