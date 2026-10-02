const source = process.argv[2];
if (!source) throw new Error('usage: node snapshot-cdp-targets.mjs <agent-browser-cdp-url>');
const pageUrl = new URL(source);
const versionUrl = `http://127.0.0.1:${pageUrl.port}/json/version`;
const versionResponse = await fetch(versionUrl);
if (!versionResponse.ok) throw new Error(`Could not read Chrome CDP version at ${versionUrl}: ${versionResponse.status}`);
const version = await versionResponse.json();
const socket = new WebSocket(version.webSocketDebuggerUrl);
const result = await new Promise((resolve, reject) => {
  const timer = setTimeout(() => reject(new Error('CDP Target.getTargets timed out')), 5000);
  socket.addEventListener('open', () => socket.send(JSON.stringify({ id: 1, method: 'Target.getTargets' })));
  socket.addEventListener('message', (event) => {
    const message = JSON.parse(event.data);
    if (message.id !== 1) return;
    clearTimeout(timer);
    socket.close();
    if (message.error) reject(new Error(message.error.message));
    else resolve(message.result.targetInfos);
  });
  socket.addEventListener('error', () => reject(new Error('Chrome CDP websocket failed')));
});
console.log(JSON.stringify({
  browser: version.Browser,
  endpoint: versionUrl,
  targets: result
    .filter((target) => ['page', 'worker', 'service_worker', 'shared_worker'].includes(target.type))
    .map(({ targetId, type, url, title }) => ({ targetId, type, url, title })),
}, null, 2));
