import { afterEach, expect, it, vi } from 'vitest';

const state = vi.hoisted(() => ({ connected:false, viewAs:null, error:null, joined:false, apply:vi.fn(),
  joinMessage:vi.fn(() => ({ type:'join' })), notePing:vi.fn(), reset:vi.fn() }));
vi.mock('../table.svelte', () => ({ table:state, forgetSeat:vi.fn() }));
class FakeSocket extends EventTarget {
  static OPEN = 1;
  static sockets:FakeSocket[] = [];
  readyState = 1;
  onclose = null;
  send = vi.fn();
  constructor(_url:string) { super(); FakeSocket.sockets.push(this); }
  close() { this.readyState = 3; }
}
afterEach(() => { vi.useRealTimers(); vi.unstubAllGlobals(); vi.resetModules(); FakeSocket.sockets = []; });

it('ignores open, message and close events from a replaced connection', async () => {
  vi.useFakeTimers();
  vi.stubGlobal('WebSocket', FakeSocket);
  vi.stubGlobal('window', globalThis);
  vi.stubGlobal('location', { protocol:'http:', host:'localhost' });
  const { joinTable } = await import('./socket');
  const seat = { name:'GM', passphrase:'local', asGm:true, gmKey:'' };
  joinTable(seat); joinTable(seat);
  const [old, current] = FakeSocket.sockets;
  current!.dispatchEvent(new Event('open'));
  expect(state.connected).toBe(true);
  const sent = current!.send.mock.calls.length;
  old!.dispatchEvent(new Event('open'));
  old!.dispatchEvent(new MessageEvent('message', { data:JSON.stringify({ type:'snapshot' }) }));
  old!.dispatchEvent(new Event('close'));
  vi.advanceTimersByTime(500);
  expect(state.connected).toBe(true);
  expect(state.apply).not.toHaveBeenCalled();
  expect(current!.send).toHaveBeenCalledTimes(sent);
  expect(FakeSocket.sockets).toHaveLength(2);
});
