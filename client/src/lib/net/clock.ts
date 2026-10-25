// ------------------------------------------------------------
// Module: clock
// Purpose: Estimate how far this browser's clock sits from the
//          server, so a roll's playAt moment is the same moment
//          at every seat.
// ------------------------------------------------------------

/** serverTime - localTime, corrected for half the round trip. */
export function offsetFromPing(sent: number, serverTime: number, received: number): number {
  const rtt = Math.max(0, received - sent);
  return serverTime - (sent + rtt / 2);
}

export class ClockSync {
  offset = 0;
  private primed = false;

  push(sample: number) {
    if (!this.primed) {
      this.offset = sample;
      this.primed = true;
      return;
    }
    this.offset = this.offset * 0.8 + sample * 0.2;
  }

  serverNow(localNow = Date.now()): number {
    return localNow + this.offset;
  }
}
