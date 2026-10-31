import assert from 'node:assert/strict';
import { test } from 'node:test';
import { waitForBackend } from './backend-ready.mjs';

test('waits through compilation, errors and unrelated responses until the API is healthy', async () => {
  let calls = 0;
  await waitForBackend('http://localhost:8080/', {
    intervalMs: 1, timeoutMs: 1000,
    probe: async url => {
      assert.equal(url.pathname, '/api/health');
      calls++;
      if (calls === 1) throw new Error('ECONNREFUSED');
      if (calls === 2) return new Response('unavailable', { status: 503 });
      if (calls === 3) return new Response('<html>wrong service</html>');
      return Response.json({ ok: true });
    },
  });
  assert.equal(calls, 4);
});

test('reports the backend address and recovery instructions when it never starts', async () => {
  await assert.rejects(waitForBackend('http://localhost:8099', {
    intervalMs: 1, timeoutMs: 15, probe: async () => Response.json({ ok: false }),
  }), /localhost:8099.*Rust server terminal.*LONGTABLE_API/);
});

test('cancels startup without waiting for the full timeout', async () => {
  const controller = new AbortController();
  controller.abort();
  await assert.rejects(waitForBackend('http://localhost:8080', { signal: controller.signal }), { name: 'AbortError' });
});
