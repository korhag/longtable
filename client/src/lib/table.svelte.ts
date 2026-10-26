// ------------------------------------------------------------
// Module: table
// Purpose: The seat's copy of the table. The server remains
//          the authority; this only remembers what it was sent.
// ------------------------------------------------------------

import { ClockSync } from './net/clock';
import type { Campaign } from './net/bindings/Campaign.ts';
import type { Listing } from './net/bindings/Listing.ts';
import type { Character } from './net/bindings/Character.ts';
import type { ClientMsg } from './net/bindings/ClientMsg.ts';
import type { DieMesh } from './net/bindings/DieMesh.ts';
import type { DoorView } from './net/bindings/DoorView.ts';
import type { FogView } from './net/bindings/FogView.ts';
import type { PlayerInfo } from './net/bindings/PlayerInfo.ts';
import type { Role } from './net/bindings/Role.ts';
import type { RollEvent } from './net/bindings/RollEvent.ts';
import type { RollLogEntry } from './net/bindings/RollLogEntry.ts';
import type { SceneView } from './net/bindings/SceneView.ts';
import type { ServerMsg } from './net/bindings/ServerMsg.ts';
import type { SystemDef } from './net/bindings/SystemDef.ts';
import type { TokenView } from './net/bindings/TokenView.ts';

export type Tool = 'select' | 'reveal' | 'hide';

export interface SeatRequest {
  name: string;
  passphrase: string;
  asGm: boolean;
  gmKey: string;
}

class TableStore {
  connected = $state(false);
  joined = $state(false);
  error = $state<string | null>(null);
  playerId = $state<string | null>(null);
  role = $state<Role>('player');
  sessionToken = $state<string | null>(null);
  players = $state<PlayerInfo[]>([]);
  scene = $state<SceneView | null>(null);
  tokens = $state<TokenView[]>([]);
  doors = $state<DoorView[]>([]);
  fog = $state<FogView | null>(null);
  rolls = $state<RollLogEntry[]>([]);
  systems = $state<SystemDef[]>([]);
  campaign = $state<Campaign>({ id: '', name: 'Campaign', systemId: null });
  campaigns = $state<Listing[]>([]);
  scenes = $state<Listing[]>([]);
  characters = $state<Character[]>([]);
  viewAs = $state<string | null>(null);
  diceMeshes = $state<DieMesh[]>([]);
  activeRoll = $state<RollEvent | null>(null);
  landedRollId = $state<string | null>(null);
  concealedNotice = $state<string | null>(null);
  selectedTokenId = $state<string | null>(null);
  tool = $state<Tool>('select');
  viewId = $state('');
  clock = new ClockSync();

  private queue: RollEvent[] = [];
  private framedScene = '';

  apply(message: ServerMsg) {
    switch (message.type) {
      case 'welcome':
        this.playerId = message.playerId;
        this.role = message.role;
        this.sessionToken = message.sessionToken;
        this.diceMeshes = message.dice;
        this.clock.push(message.serverTime - Date.now());
        this.joined = true;
        this.error = null;
        sessionStorage.setItem('battle-map-id', message.playerId);
        break;
      case 'systems':
        this.systems = message.systems;
        break;
      case 'pong':
        break;
      case 'snapshot':
        this.players = message.players;
        this.rememberView(message.scene);
        this.scene = message.scene;
        this.tokens = message.tokens;
        this.doors = message.doors;
        this.fog = message.fog;
        this.rolls = message.rolls;
        this.campaign = message.campaign;
        this.campaigns = message.campaigns;
        this.scenes = message.scenes;
        this.characters = message.characters;
        this.viewAs = message.viewAs;
        this.role = message.you.role;
        if (this.selectedTokenId && !message.tokens.some((token) => token.id === this.selectedTokenId)) {
          this.selectedTokenId = null;
        }
        break;
      case 'roll':
        this.enqueue(message);
        break;
      case 'concealed':
        this.concealedNotice = message.notice;
        break;
      case 'error':
        this.error = message.message;
        break;
    }
  }

  notePing(sent: number, serverTime: number, received: number) {
    const rtt = Math.max(0, received - sent);
    this.clock.push(serverTime - (sent + rtt / 2));
  }

  enqueue(roll: RollEvent) {
    if (this.activeRoll) this.queue.push(roll);
    else this.activeRoll = roll;
  }

  finishRoll(id: string) {
    if (this.activeRoll?.id === id) this.landedRollId = id;
  }

  dismissRoll() {
    const next = this.queue.shift() ?? null;
    this.activeRoll = next;
    if (next) this.landedRollId = null;
  }

  clearNotice() {
    this.concealedNotice = null;
  }

  turnView(step: number) {
    const order = this.scene?.presentation?.viewOrder ?? [];
    if (order.length < 2) return;
    const index = order.indexOf(this.viewId);
    const start = index < 0 ? 0 : index;
    const count = order.length;
    this.viewId = order[((start + step) % count + count) % count] ?? order[0] ?? '';
  }

  private rememberView(scene: SceneView | null) {
    const presentation = scene?.presentation;
    if (!scene || !presentation) {
      this.viewId = '';
      this.framedScene = scene?.id ?? '';
      return;
    }
    if (scene.id !== this.framedScene || !presentation.viewOrder.includes(this.viewId)) {
      this.framedScene = scene.id;
      this.viewId = presentation.defaultView;
    }
  }

  reset() {
    this.joined = false;
    this.connected = false;
    this.playerId = null;
    this.sessionToken = null;
    this.role = 'player';
    this.players = [];
    this.scene = null;
    this.tokens = [];
    this.doors = [];
    this.fog = null;
    this.rolls = [];
    this.systems = [];
    this.campaign = { id: '', name: 'Campaign', systemId: null };
    this.campaigns = [];
    this.scenes = [];
    this.characters = [];
    this.viewAs = null;
    this.activeRoll = null;
    this.landedRollId = null;
    this.concealedNotice = null;
    this.selectedTokenId = null;
    this.viewId = '';
    this.framedScene = '';
    this.queue = [];
  }

  joinMessage(seat: SeatRequest): ClientMsg {
    const resumeId = sessionStorage.getItem('battle-map-id');
    return {
      type: 'join',
      name: seat.name,
      passphrase: seat.passphrase,
      asGm: seat.asGm,
      gmKey: seat.asGm ? seat.gmKey : null,
      resumeId,
    };
  }
}

export const table = new TableStore();

const SEAT_KEY = 'battle-map-seat';

export function saveSeat(seat: SeatRequest) {
  sessionStorage.setItem(SEAT_KEY, JSON.stringify(seat));
}

export function loadSeat(): SeatRequest | null {
  const raw = sessionStorage.getItem(SEAT_KEY);
  if (!raw) return null;
  try {
    const seat = JSON.parse(raw) as SeatRequest;
    if (!seat.name || !seat.passphrase) return null;
    return seat;
  } catch {
    return null;
  }
}

export function forgetSeat() {
  sessionStorage.removeItem(SEAT_KEY);
  sessionStorage.removeItem('battle-map-id');
}
