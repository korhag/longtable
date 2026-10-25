// ------------------------------------------------------------
// Module: index
// Purpose: The one place the shared client asks a system for
//          extra dice controls. Other systems add a line here.
// ------------------------------------------------------------

import type { Component } from 'svelte';

import DualityButton from './daggerheart/DualityButton.svelte';

const extras: Record<string, Component> = {
  daggerheart: DualityButton,
};

export function extraFor(systemId: string | null): Component | null {
  if (!systemId) return null;
  return extras[systemId] ?? null;
}
