// ------------------------------------------------------------
// Module: socket
// Purpose: One websocket to the host. Join first, then keep
//          the clock honest with pings. A dropped cable
//          reconnects with the same seat.
// ------------------------------------------------------------

import type { ClientMsg } from './bindings/ClientMsg.ts';
import type { ServerMsg } from './bindings/ServerMsg.ts';
import { forgetSeat, table, type SeatRequest } from '../table.svelte';

let socket: WebSocket | null = null;
let seat: SeatRequest | null = null;
let pingTimer = 0;
let reconnectTimer = 0;
let attempt = 0;
let allowReconnect = false;

export function joinTable(next: SeatRequest) {
  seat = next;
  allowReconnect = true;
  attempt = 0;
  open();
}

export function leaveTable() {
  allowReconnect = false;
  seat = null;
  closeSocket();
  forgetSeat();
  table.reset();
}

export function send(message: ClientMsg) {
  if (socket?.readyState === WebSocket.OPEN) {
    socket.send(JSON.stringify(message));
  }
}

function open() {
  if (!seat) return;
  closeSocket();
  const protocol = location.protocol === 'https:' ? 'wss' : 'ws';
  socket = new WebSocket(`${protocol}://${location.host}/ws`);
  socket.addEventListener('open', () => {
    attempt = 0;
    table.connected = true;
    table.error = null;
    send(table.joinMessage(seat!));
    window.clearInterval(pingTimer);
    pingTimer = window.setInterval(ping, 4000);
    ping();
  });
  socket.addEventListener('message', (event) => {
    const message = parse(String(event.data));
    if (!message) return;
    if (message.type === 'pong') {
      const received = Date.now();
      table.notePing(message.clientTime, message.serverTime, received);
      return;
    }
    if (message.type === 'error' && !table.joined) {
      allowReconnect = false;
      table.error = message.message;
      return;
    }
    table.apply(message);
  });
  socket.addEventListener('close', () => {
    table.connected = false;
    window.clearInterval(pingTimer);
    if (!allowReconnect || !seat) return;
    const wait = Math.min(8000, 400 * 2 ** attempt);
    attempt += 1;
    reconnectTimer = window.setTimeout(open, wait);
  });
}

function ping() {
  send({ type: 'ping', clientTime: Date.now() });
}

function parse(raw: string): ServerMsg | null {
  try {
    const value = JSON.parse(raw) as ServerMsg;
    if (!value || typeof value !== 'object' || !('type' in value)) return null;
    return value;
  } catch {
    return null;
  }
}

function closeSocket() {
  window.clearTimeout(reconnectTimer);
  if (!socket) return;
  socket.onclose = null;
  socket.close();
  socket = null;
}
