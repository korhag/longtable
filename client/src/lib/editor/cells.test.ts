// ------------------------------------------------------------
// Module: cells.test
// Purpose: Cell indices survive the same base64 the server stores.
// ------------------------------------------------------------

import { describe, expect, it } from 'vitest';

import { decodeCells, encodeCells } from './cells';

describe('cells', () => {
  it('round-trips every index', () => {
    const bytes = new Uint8Array([0, 1, 7, 255, 3]);
    expect([...decodeCells(encodeCells(bytes))]).toEqual([0, 1, 7, 255, 3]);
  });
});
