import { spawn, execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { setTimeout as delay } from 'node:timers/promises';

const port = 5202;
const url = `http://127.0.0.1:${port}/perf.html`;
const swift = process.argv.includes('--swiftshader');
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
  const line = swift ? await swiftDump(url) : await gpuPoll(url);
  console.log(hostLine());
  console.log(line);
  if (!line.includes('PERF_OK')) process.exitCode = 1;
} finally {
  vite.kill('SIGTERM');
}

function hostLine() {
  return `host: ${runSync('uname', ['-sr'])}; chromium ${runSync('chromium', ['--version'])}; cpu ${runSync('nproc', [])} threads; mem ${memTotal()}`;
}

function memTotal() {
  try {
    const text = readFileSync('/proc/meminfo', 'utf8');
    return text.split('\n').find((line) => line.startsWith('MemTotal:'))?.replace(/\s+/g, ' ') ?? 'unknown';
  } catch {
    return 'unknown';
  }
}

function runSync(command, args) {
  return execFileSync(command, args, { encoding: 'utf8' }).trim();
}

async function swiftDump(pageUrl) {
  const dump = await run('chromium', [
    '--headless=new',
    '--disable-gpu',
    '--use-gl=angle',
    '--use-angle=swiftshader',
    '--enable-webgl',
    '--ignore-gpu-blocklist',
    '--enable-unsafe-swiftshader',
    '--no-sandbox',
    '--virtual-time-budget=45000',
    '--dump-dom',
    '--window-size=1920,1080',
    pageUrl,
  ]);
  return markerLine(dump);
}

async function gpuPoll(pageUrl) {
  const profile = `/tmp/longtable-perf-${process.pid}`;
  const chrome = spawn('chromium', [
    '--headless=new',
    '--remote-debugging-port=9223',
    `--user-data-dir=${profile}`,
    '--no-sandbox',
    '--use-gl=angle',
    '--use-angle=gl',
    '--ignore-gpu-blocklist',
    '--enable-gpu',
    '--enable-webgl',
    '--window-size=1920,1080',
    'about:blank',
  ], { stdio: ['ignore', 'pipe', 'pipe'] });
  try {
    const browser = await debuggerSocket();
    const created = await browser.send('Target.createTarget', { url: pageUrl });
    const attached = await browser.send('Target.attachToTarget', { targetId: created.targetId, flatten: true });
    const session = attached.sessionId;
    await browser.send('Runtime.enable', {}, session);
    const deadline = Date.now() + 90000;
    let text = '';
    while (Date.now() < deadline) {
      const result = await browser.send('Runtime.evaluate', {
        expression: `document.querySelector('#out')?.textContent ?? ''`,
        returnByValue: true,
      }, session);
      text = result.result?.value ?? '';
      if (text.includes('PERF_OK') || text.includes('PERF_FAIL')) return text.trim();
      await delay(400);
    }
    throw new Error(`performance page did not finish\n${text}`);
  } finally {
    chrome.kill('SIGTERM');
  }
}

async function debuggerSocket() {
  for (let attempt = 0; attempt < 40; attempt += 1) {
    try {
      const version = await fetch('http://127.0.0.1:9223/json/version').then((response) => response.json());
      return connect(version.webSocketDebuggerUrl);
    } catch {
      await delay(250);
    }
  }
  throw new Error('chromium did not open a debugging port');
}

function connect(url) {
  const ws = new WebSocket(url);
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

function markerLine(dump) {
  const marker = dump.includes('PERF_OK') ? 'PERF_OK' : dump.includes('PERF_FAIL') ? 'PERF_FAIL' : '';
  if (!marker) throw new Error(`performance page did not finish\n${dump.slice(-2000)}`);
  const line = dump.split('\n').find((item) => item.includes(marker)) ?? dump;
  return line.replace(/<[^>]+>/g, '').trim();
}

function run(command, args) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { stdio: ['ignore', 'pipe', 'pipe'] });
    let text = '';
    child.stdout.on('data', (chunk) => {
      text += chunk;
    });
    child.stderr.on('data', (chunk) => {
      text += chunk;
    });
    child.on('error', reject);
    child.on('close', (code) => {
      if (code === 0) resolve(text);
      else reject(new Error(`${command} exited ${code}\n${text.slice(-2000)}`));
    });
  });
}
