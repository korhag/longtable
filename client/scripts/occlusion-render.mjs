import { spawn } from 'node:child_process';
import { setTimeout as delay } from 'node:timers/promises';

const port = 5203;
const url = `http://127.0.0.1:${port}/occlusion.html`;
const vite = spawn('npx', ['vite', '--host', '127.0.0.1', '--port', String(port), '--strictPort'], {
  cwd: new URL('..', import.meta.url),
  stdio: ['ignore', 'pipe', 'pipe'],
});

let ready = false;
const note = (chunk) => {
  if (String(chunk).includes(String(port))) ready = true;
};
vite.stdout.on('data', note);
vite.stderr.on('data', note);

try {
  for (let attempt = 0; attempt < 40 && !ready; attempt += 1) await delay(250);
  if (!ready) throw new Error('vite did not start');
  await delay(400);
  const dump = await run('chromium', [
    '--headless=new',
    '--disable-gpu',
    '--use-gl=angle',
    '--use-angle=swiftshader',
    '--enable-webgl',
    '--ignore-gpu-blocklist',
    '--enable-unsafe-swiftshader',
    '--virtual-time-budget=45000',
    '--no-sandbox',
    '--dump-dom',
    '--window-size=640,480',
    url,
  ]);
  const line = dump.split('\n').map((item) => item.replace(/<[^>]+>/g, '').trim()).find((item) => item.includes('OCCLUSION_'));
  console.log(line || dump.slice(-1500));
  if (!dump.includes('OCCLUSION_OK')) process.exitCode = 1;
} finally {
  vite.kill('SIGTERM');
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
      else reject(new Error(`${command} exited ${code}\n${text.slice(-1500)}`));
    });
  });
}
