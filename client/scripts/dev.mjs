import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { waitForBackend } from './backend-ready.mjs';

const controller = new AbortController();
const cancel = () => controller.abort();
process.once('SIGINT', cancel);
process.once('SIGTERM', cancel);
const api = process.env.LONGTABLE_API ?? 'http://127.0.0.1:8080';
console.log(`Waiting for the Rust backend at ${api}. First-time compilation can take several minutes.`);
try {
  await waitForBackend(api, { signal: controller.signal });
} catch (error) {
  console.error(controller.signal.aborted ? 'Startup cancelled.' : error.message);
  process.exit(controller.signal.aborted ? 130 : 1);
}
process.removeListener('SIGINT', cancel);
process.removeListener('SIGTERM', cancel);
console.log('Backend ready. Starting Vite.');
const child = spawn(process.execPath, [fileURLToPath(new URL('../node_modules/vite/bin/vite.js', import.meta.url)), ...process.argv.slice(2)], { stdio: 'inherit' });
process.on('SIGINT', () => child.kill('SIGINT'));
process.on('SIGTERM', () => child.kill('SIGTERM'));
child.on('error', error => { console.error(error.message); process.exitCode = 1; });
child.on('exit', (code, signal) => { process.exitCode = code ?? (signal ? 130 : 1); });
