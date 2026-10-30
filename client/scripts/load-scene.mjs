import { spawn } from 'node:child_process';
import { setTimeout as delay } from 'node:timers/promises';

const port = 5204;
const url = `http://127.0.0.1:${port}/load.html`;
const vite = spawn('npx', ['vite', '--host', '127.0.0.1', '--port', String(port), '--strictPort'], {
  cwd: new URL('..', import.meta.url),
  stdio: ['ignore', 'pipe', 'pipe'],
});

let ready = false;
vite.stdout.on('data', (chunk) => {
  if (String(chunk).includes(String(port))) ready = true;
});
vite.stderr.on('data', (chunk) => {
  if (String(chunk).includes(String(port))) ready = true;
});

try {
  for (let attempt = 0; attempt < 40 && !ready; attempt += 1) await delay(250);
  if (!ready) throw new Error('vite did not start');
  await delay(400);
  const line = await poll(url);
  console.log(line);
  if (!line.includes('LOAD_OK')) process.exitCode = 1;
} finally {
  vite.kill('SIGTERM');
}

async function poll(pageUrl) {
  const profile = `/tmp/longtable-load-${process.pid}`;
  const chrome = spawn('chromium', [
    '--headless=new',
    '--remote-debugging-port=9224',
    `--user-data-dir=${profile}`,
    '--no-sandbox',
    '--use-gl=angle',
    '--use-angle=swiftshader',
    '--enable-unsafe-swiftshader',
    '--enable-webgl',
    '--ignore-gpu-blocklist',
    '--window-size=960,540',
    'about:blank',
  ], { stdio: ['ignore', 'pipe', 'pipe'] });
  try {
    const browser = await debuggerSocket();
    const created = await browser.send('Target.createTarget', { url: pageUrl });
    const attached = await browser.send('Target.attachToTarget', { targetId: created.targetId, flatten: true });
    const session = attached.sessionId;
    await browser.send('Runtime.enable', {}, session);
    const deadline = Date.now() + 40000;
    let text = '';
    while (Date.now() < deadline) {
      const result = await browser.send('Runtime.evaluate', {
        expression: `document.querySelector('#out')?.textContent ?? ''`,
        returnByValue: true,
      }, session);
      text = result.result?.value ?? '';
      if (text.includes('LOAD_OK') || text.includes('LOAD_FAIL')) return text.trim();
      await delay(300);
    }
    throw new Error(`load page did not finish\n${text}`);
  } finally {
    chrome.kill('SIGTERM');
  }
}

async function debuggerSocket() {
  for (let attempt = 0; attempt < 40; attempt += 1) {
    try {
      const version = await fetch('http://127.0.0.1:9224/json/version').then((response) => response.json());
      return connect(version.webSocketDebuggerUrl);
    } catch {
      await delay(250);
    }
  }
  throw new Error('chromium did not open a debugging port');
}

function connect(socketUrl) {
  const ws = new WebSocket(socketUrl);
  const pending = new Map();
  let next = 0;
  const ready = new Promise((resolve, reject) => {
    ws.addEventListener('open', () => resolve());
    ws.addEventListener('error', () => reject(new Error('browser socket failed')));
  });
  ws.addEventListener('message', (event) => {
    const message = JSON.parse(event.data);
    if (message.id && pending.has(message.id)) pending.get(message.id)(message);
  });
  return ready.then(() => ({
    send(method, params = {}, sessionId) {
      const id = ++next;
      return new Promise((resolve, reject) => {
        pending.set(id, (message) => {
          pending.delete(id);
          if (message.error) reject(new Error(`${method}: ${JSON.stringify(message.error)}`));
          else resolve(message.result);
        });
        ws.send(JSON.stringify({ id, method, params, sessionId }));
      });
    },
  }));
}
