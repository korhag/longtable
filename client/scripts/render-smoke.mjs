import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { setTimeout as delay } from 'node:timers/promises';

const port = 5199;
const url = `http://127.0.0.1:${port}/smoke.html`;
const vite = spawn('npx', ['vite', '--host', '127.0.0.1', '--port', String(port), '--strictPort'], {
  cwd: new URL('..', import.meta.url),
  stdio: ['ignore', 'pipe', 'pipe'],
});

let ready = false;
vite.stdout.on('data', (chunk) => {
  const text = String(chunk);
  if (text.includes('Local') || text.includes(`${port}`)) ready = true;
});
vite.stderr.on('data', (chunk) => {
  const text = String(chunk);
  if (text.includes('Local') || text.includes(`${port}`)) ready = true;
});

try {
  for (let attempt = 0; attempt < 40 && !ready; attempt += 1) await delay(250);
  if (!ready) throw new Error('vite did not start');
  await delay(400);
  const shot = '/tmp/longtable-g0-smoke.png';
  const dump = await run('chromium', [
    '--headless=new',
    '--disable-gpu',
    '--use-gl=angle',
    '--use-angle=swiftshader',
    '--enable-webgl',
    '--ignore-gpu-blocklist',
    '--enable-unsafe-swiftshader',
    '--virtual-time-budget=4000',
    '--no-sandbox',
    '--dump-dom',
    `--screenshot=${shot}`,
    '--window-size=640,400',
    url,
  ]);
  if (!dump.includes('SMOKE_OK')) {
    console.error(dump);
    throw new Error('smoke page did not report SMOKE_OK');
  }
  const { readFile } = await import('node:fs/promises');
  const file = await readFile(shot);
  await mkdir(new URL('../../docs/milestones/', import.meta.url), { recursive: true });
  await writeFile(new URL('../../docs/milestones/g0-smoke.png', import.meta.url), file);
  console.log('SMOKE_OK');
} finally {
  vite.kill('SIGTERM');
}

function run(command, args) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { stdio: ['ignore', 'pipe', 'pipe'] });
    let text = '';
    child.stdout.on('data', (chunk) => { text += chunk; });
    child.stderr.on('data', (chunk) => { text += chunk; });
    child.on('error', reject);
    child.on('close', (code) => {
      if (code === 0) resolve(text);
      else reject(new Error(`${command} exited ${code}\n${text}`));
    });
  });
}
