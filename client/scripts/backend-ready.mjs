import { setTimeout as pause } from 'node:timers/promises';

// Gate Vite on the actual API, not just an open TCP port or a fixed delay.
export async function waitForBackend(baseUrl, {
  timeoutMs = 300_000, intervalMs = 500, probe = fetch, signal,
} = {}) {
  const health = new URL(`${baseUrl.replace(/\/$/, '')}/api/health`);
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    signal?.throwIfAborted();
    try {
      const response = await probe(health, {
        signal: AbortSignal.any([AbortSignal.timeout(Math.min(2000, Math.max(1, deadline - Date.now()))), ...(signal ? [signal] : [])]),
      });
      if (response.ok && (await response.json()).ok === true) return;
    } catch {
      signal?.throwIfAborted();
      // Connection refusals and incomplete startup responses are retried.
    }
    await pause(Math.min(intervalMs, Math.max(0, deadline - Date.now())), undefined, { signal });
  }
  throw new Error(`Backend did not become ready at ${health.origin}. Check the Rust server terminal for build errors or a port conflict. Run cargo run --manifest-path server/Cargo.toml from the repository root. If it uses another address, set LONGTABLE_API to that address.`);
}
