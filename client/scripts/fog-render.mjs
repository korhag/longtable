import { spawn } from 'node:child_process';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';

const port = 5211;
const debugPort = 9251;
const profile = await mkdtemp(join(tmpdir(), 'longtable-fog-'));
const vite = spawn(process.execPath, ['node_modules/vite/bin/vite.js', '--host', '127.0.0.1', '--port', String(port), '--strictPort'], {
  cwd: new URL('..', import.meta.url), stdio: 'ignore',
});
let chrome;
let socket;
try {
  await until(async () => (await fetch(`http://127.0.0.1:${port}/fog.html`)).ok);
  chrome = spawn('chromium', [
    '--headless=new', '--no-sandbox', '--use-gl=angle', '--use-angle=swiftshader',
    '--enable-webgl', '--ignore-gpu-blocklist', '--enable-unsafe-swiftshader',
    '--disable-dev-shm-usage', '--window-size=1000,700',
    `--user-data-dir=${profile}`, `--remote-debugging-port=${debugPort}`, 'about:blank',
  ], { stdio: 'ignore' });
  await until(async () => (await fetch(`http://127.0.0.1:${debugPort}/json/version`)).ok);
  const targets = await (await fetch(`http://127.0.0.1:${debugPort}/json/list`)).json();
  socket = new WebSocket(targets.find(target => target.type === 'page').webSocketDebuggerUrl);
  await new Promise((resolve, reject) => { socket.onopen = resolve; socket.onerror = reject; });
  let nextId = 0;
  const pending = new Map();
  socket.onmessage = ({ data }) => {
    const response = JSON.parse(data);
    const callback = pending.get(response.id);
    if (callback) { pending.delete(response.id); callback(response); }
  };
  const send = (method, params = {}) => new Promise((resolve, reject) => {
    const id = ++nextId;
    const timeout = setTimeout(() => { pending.delete(id); reject(new Error(`${method} timed out`)); }, 10000);
    pending.set(id, response => { clearTimeout(timeout); response.error ? reject(new Error(JSON.stringify(response.error))) : resolve(response.result); });
    socket.send(JSON.stringify({ id, method, params }));
  });
  await send('Page.navigate', { url: `http://127.0.0.1:${port}/fog.html` });
  let report = '';
  await until(async () => {
    const result = await send('Runtime.evaluate', { expression: 'document.querySelector("#out")?.textContent', returnByValue: true });
    report = result.result?.value ?? '';
    return report.includes('VISION_') || report.includes('FOG_FAIL');
  });
  const screenshot = await send('Page.captureScreenshot', { format: 'png' });
  await writeFile('/tmp/longtable-fog-vision.png', Buffer.from(screenshot.data, 'base64'));
  console.log(report);
  if (!report.includes('FOG_OK') || !report.includes('VISION_OK')) process.exitCode = 1;
} finally {
  socket?.close();
  chrome?.kill('SIGTERM');
  vite.kill('SIGTERM');
  await delay(250);
  await rm(profile, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 });
}

async function until(check) {
  for (let attempt = 0; attempt < 60; attempt++) {
    try { if (await check()) return; } catch { /* server/browser starting */ }
    await delay(250);
  }
  throw new Error('Browser fixture did not become ready within 15 seconds');
}
