import { beforeEach, describe, expect, it } from 'vitest';
import type { RollEvent } from './net/bindings/RollEvent';
import { table } from './table.svelte';

const unrelated: RollEvent = { id: 'skill', rollerId: 'alice', rollerName: 'Alice', formula: '1d6', label: null, visibility: 'public', modifier: 0, total: 4, playAt: 0, keyframeDt: 1 / 30, dice: [], outcome: null, contest: null };
const hit: RollEvent = { ...unrelated, id: 'hit', contest: { attackerId: 'hero', attackerName: 'Hero', targetId: 'bandit', targetName: 'Bandit', defenseName: 'Difficulty', defense: 12, verdict: 'hit', blow: null, damageRolled: false, attackRollId: null } };
const damage: RollEvent = { ...unrelated, id: 'damage', contest: { ...hit.contest!, verdict: null, blow: { step: 'Major', marks: 2, appliedMarks: 2, pendingArmor: false, majorThreshold: 7, severeThreshold: 13 }, attackRollId: 'hit' } };

describe('retained attack tray', () => {
  beforeEach(() => table.reset());

  it('replaces its settled hit with damage and preserves unrelated queued rolls', () => {
    table.enqueue(hit);
    table.finishRoll(hit.id);
    table.enqueue(unrelated);
    table.enqueue(damage);
    expect(table.activeRoll?.id).toBe('damage');
    expect(table.landedRollId).toBeNull();
    table.dismissRoll();
    expect(table.activeRoll?.id).toBe('skill');
  });

  it('lets an attack finish animating before a remotely requested damage roll', () => {
    table.enqueue(hit);
    table.enqueue(damage);
    expect(table.activeRoll?.id).toBe('hit');
    table.finishRoll(hit.id);
    table.dismissRoll();
    expect(table.activeRoll?.id).toBe('damage');
  });

  it('does not enqueue duplicate roll delivery', () => {
    table.enqueue(hit);
    table.enqueue(hit);
    table.enqueue(unrelated);
    table.enqueue(unrelated);
    table.dismissRoll();
    expect(table.activeRoll?.id).toBe('skill');
    table.dismissRoll();
    expect(table.activeRoll).toBeNull();
  });
});
