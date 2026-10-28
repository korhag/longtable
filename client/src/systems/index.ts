// ------------------------------------------------------------
// Module: index
// Purpose: The one place the shared client asks a system for
//          extra dice controls. Other systems add a line here.
// ------------------------------------------------------------

import type { Component } from 'svelte';

import type { Character } from '../lib/net/bindings/Character.ts';
import type { CharacterKind } from '../lib/net/bindings/CharacterKind.ts';
import AdversaryCard from './daggerheart/AdversaryCard.svelte';
import { creation } from './daggerheart/creation.svelte';
import DualityButton from './daggerheart/DualityButton.svelte';
import Overlays from './daggerheart/Overlays.svelte';
import PcSheet from './daggerheart/PcSheet.svelte';

const extras: Record<string, Component> = {
  daggerheart: DualityButton,
};

type SheetView = Component<{ character: Character; preview?: boolean }>;

const sheets: Record<string, Partial<Record<CharacterKind, SheetView>>> = {
  daggerheart: { pc: PcSheet, adversary: AdversaryCard },
};

const overlays: Record<string, Component> = {
  daggerheart: Overlays,
};

export function extraFor(systemId: string | null): Component | null {
  if (!systemId) return null;
  return extras[systemId] ?? null;
}

export function sheetFor(systemId: string | null, kind: CharacterKind): SheetView | null {
  if (!systemId) return null;
  return sheets[systemId]?.[kind] ?? null;
}

export function overlaysFor(systemId: string | null): Component | null {
  if (!systemId) return null;
  return overlays[systemId] ?? null;
}

export function openCreation(kind: 'pc' | 'adversary') {
  creation.open(kind);
}
