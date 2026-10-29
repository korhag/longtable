// ------------------------------------------------------------
// Module: account
// Purpose: The signed-in person, their tables, and their friends.
//          The token stays in this browser.
// ------------------------------------------------------------

import { prefs, type AccountSettings } from './prefs.svelte';

const TOKEN_KEY = 'game-table.token';

export interface Person {
  id: string;
  username: string;
}

export interface TableCard {
  id: string;
  name: string;
  yours: boolean;
  standing: string;
}

export interface SeatRequestRow {
  tableId: string;
  tableName: string;
  accountId: string;
  username: string;
}

export interface OpenTable {
  id: string;
  name: string;
  ownerName: string;
}

export interface Lobby {
  account: { id: string; username: string; color: string; settings: AccountSettings };
  tables: TableCard[];
  requests: SeatRequestRow[];
  friends: Person[];
  incoming: Person[];
  friendTables: OpenTable[];
  openId: string | null;
}

function readToken(): string | null {
  return localStorage.getItem(TOKEN_KEY);
}

async function send(path: string, method: string, body?: unknown): Promise<unknown> {
  const token = readToken();
  const headers: Record<string, string> = { 'Content-Type': 'application/json' };
  if (token) headers.Authorization = `Bearer ${token}`;
  const response = await fetch(path, {
    method,
    headers,
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const payload = (await response.json().catch(() => ({}))) as { message?: string };
  if (!response.ok) throw new Error(payload.message || 'that did not work');
  return payload;
}

class AccountStore {
  token = $state<string | null>(readToken());
  lobby = $state<Lobby | null>(null);
  error = $state<string | null>(null);

  remember(token: string, lobby: Lobby) {
    localStorage.setItem(TOKEN_KEY, token);
    this.token = token;
    this.lobby = lobby;
    this.error = null;
    prefs.adopt(lobby.account.settings);
  }

  take(lobby: Lobby) {
    this.lobby = lobby;
    this.error = null;
    prefs.adopt(lobby.account.settings);
  }

  async login(username: string, password: string) {
    const payload = (await send('/api/login', 'POST', { username, password })) as { token: string; lobby: Lobby };
    this.remember(payload.token, payload.lobby);
  }

  async register(username: string, password: string) {
    const payload = (await send('/api/register', 'POST', { username, password })) as { token: string; lobby: Lobby };
    this.remember(payload.token, payload.lobby);
  }

  async refresh() {
    if (!this.token) return;
    try {
      this.take((await send('/api/me', 'GET')) as Lobby);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'sign in again';
      if (message === 'sign in again') this.clear();
      else this.error = message;
    }
  }

  async update(body: { username?: string; color?: string; settings?: AccountSettings }) {
    const payload = (await send('/api/me', 'PATCH', body)) as { lobby: Lobby };
    this.take(payload.lobby);
  }

  async createTable(name: string) {
    this.take((await send('/api/tables', 'POST', { name })) as Lobby);
  }

  async openTable(id: string) {
    this.take((await send(`/api/tables/${id}/open`, 'POST')) as Lobby);
  }

  async requestSeat(id: string) {
    this.take((await send(`/api/tables/${id}/request`, 'POST')) as Lobby);
  }

  async approve(tableId: string, accountId: string, always: boolean) {
    this.take((await send(`/api/tables/${tableId}/approve`, 'POST', { accountId, always })) as Lobby);
  }

  async addFriend(username: string) {
    this.take((await send('/api/friends', 'POST', { name: username })) as Lobby);
  }

  async acceptFriend(id: string) {
    this.take((await send(`/api/friends/${id}/accept`, 'POST')) as Lobby);
  }

  clear() {
    localStorage.removeItem(TOKEN_KEY);
    this.token = null;
    this.lobby = null;
  }
}

export const account = new AccountStore();
