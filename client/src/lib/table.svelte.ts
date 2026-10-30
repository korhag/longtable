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
import type { Figure } from './net/bindings/Figure.ts';
import type { FogView } from './net/bindings/FogView.ts';
import type { PlayerInfo } from './net/bindings/PlayerInfo.ts';
import type { Role } from './net/bindings/Role.ts';
import type { RollEvent } from './net/bindings/RollEvent.ts';
import type { RollLogEntry } from './net/bindings/RollLogEntry.ts';
import type { SceneView } from './net/bindings/SceneView.ts';
import type { StrikeOffer } from './net/bindings/StrikeOffer.ts';
import type { TalkLine } from './net/bindings/TalkLine.ts';
import type { ServerMsg } from './net/bindings/ServerMsg.ts';
import type { SystemDef } from './net/bindings/SystemDef.ts';
import type { TokenView } from './net/bindings/TokenView.ts';

export type Tool = 'select' | 'reveal' | 'hide';

export interface SeatRequest {
  name: string;
  passphrase: string;
  asGm: boolean;
  gmKey: string;
  accountToken?: string;
  tableId?: string;
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
  targetTokenId = $state<string | null>(null);
  hoverTokenId = $state<string | null>(null);
  menu = $state<{ tokenId: string; x: number; y: number } | null>(null);
  fear = $state(0);
  catalogRevision = $state('');
  figures = $state<Figure[]>([]);
  canUndo = $state(false);
  strike = $state<StrikeOffer | null>(null);
  talk = $state<TalkLine[]>([]);
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
        this.fear = message.fear;
        this.figures = message.figures;
        this.canUndo = message.canUndo;
        this.strike = message.strike;
        this.talk = message.talk;
        this.catalogRevision = String(message.catalogRevision ?? '');
        if (this.selectedTokenId && !message.tokens.some((token) => token.id === this.selectedTokenId)) {
          this.selectedTokenId = null;
        }
        if (this.targetTokenId && !message.tokens.some((token) => token.id === this.targetTokenId)) {
          this.targetTokenId = null;
        }
        if (this.hoverTokenId && !message.tokens.some((token) => token.id === this.hoverTokenId)) {
          this.hoverTokenId = null;
        }
        const menu = this.menu;
        if (menu && !message.tokens.some((token) => token.id === menu.tokenId)) {
          this.menu = null;
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

  turnView(_step: number) {
    return;
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
    this.targetTokenId = null;
    this.hoverTokenId = null;
    this.menu = null;
    this.fear = 0;
    this.catalogRevision = '';
    this.figures = [];
    this.canUndo = false;
    this.strike = null;
    this.talk = [];
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
      accountToken: seat.accountToken || null,
      tableId: seat.tableId || null,
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
    if (!seat.name || (!seat.passphrase && !seat.accountToken)) return null;
    return seat;
  } catch {
    return null;
  }
}

export function forgetSeat() {
  sessionStorage.removeItem(SEAT_KEY);
  sessionStorage.removeItem('battle-map-id');
}
