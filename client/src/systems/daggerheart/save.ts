// ------------------------------------------------------------
// Module: save
// Purpose: Create a character, then write the sheet once the
//          table has given it an id. Place it on the open scene.
// ------------------------------------------------------------

import type { Character } from '../../lib/net/bindings/Character.ts';
import type { CharacterKind } from '../../lib/net/bindings/CharacterKind.ts';
import { send } from '../../lib/net/socket';
import { systems } from '../../lib/systems/store.svelte';
import { table } from '../../lib/table.svelte';
import type { CharacterPayload } from './draft';

const COLORS = ['#d4a656', '#c45c4a', '#6e9a72', '#6f8fbf', '#8d5b86', '#e7d7b6'];

export function writeCharacter(id: string, numbers: Record<string, number> = {}, texts: Record<string, string> = {}, name: string | null = null) {
  send({
    type: 'updateCharacter',
    characterId: id,
    name,
    kind: null,
    ownerId: null,
    clearOwner: false,
    numbers,
    texts,
  });
}

export function adjust(id: string, field: string, delta: number) {
  if (!delta) return;
  send({ type: 'adjustResource', characterId: id, field, delta });
}

export async function createFrom(kind: CharacterKind, body: CharacterPayload): Promise<string> {
  const before = new Set(table.characters.map((character) => character.id));
  const prior = table.error;
  send({ type: 'createCharacter', name: body.name, kind });
  const created = await waitFor(before, kind, body.name, prior);
  writeCharacter(created.id, body.numbers, body.texts);
  systems.selectedId = created.id;
  return created.id;
}

export function placeCharacter(characterId: string, name: string, kind: CharacterKind) {
  const scene = table.scene;
  const index = table.tokens.length;
  const originX = scene ? scene.width / 2 : 2.5;
  const originY = scene ? scene.height / 2 : 2.5;
  send({
    type: 'createToken',
    name,
    x: originX + (index % 5) - 2,
    y: originY + Math.floor(index / 5),
    ownerId: null,
    hidden: kind !== 'pc',
    visionRadius: kind === 'pc' ? 12 : 8,
    grantsVision: kind === 'pc',
    color: kind === 'adversary' ? '#c45c4a' : (COLORS[index % COLORS.length] ?? '#d4a656'),
    characterId,
  });
}

function waitFor(before: Set<string>, kind: CharacterKind, name: string, prior: string | null): Promise<Character> {
  return new Promise((resolve, reject) => {
    const started = Date.now();
    const timer = window.setInterval(() => {
      const found = table.characters.find((character) => !before.has(character.id) && character.kind === kind && character.name === name);
      if (found) {
        window.clearInterval(timer);
        resolve(found);
        return;
      }
      if (table.error && table.error !== prior) {
        window.clearInterval(timer);
        reject(new Error(table.error));
        return;
      }
      if (Date.now() - started > 5000) {
        window.clearInterval(timer);
        reject(new Error('The table did not add that character.'));
      }
    }, 40);
  });
}
