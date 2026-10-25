// ------------------------------------------------------------
// Module: host
// Purpose: Ask the server whether this browser is on the
//          computer running the game. Only that machine is
//          told the GM key.
// ------------------------------------------------------------

export interface HostInfo {
  gmKey: string;
  passphrase: string;
}

export async function loadHost(): Promise<HostInfo | null> {
  try {
    const response = await fetch('/api/host');
    if (!response.ok) return null;
    const body = (await response.json()) as Partial<HostInfo>;
    if (!body.gmKey || !body.passphrase) return null;
    return { gmKey: body.gmKey, passphrase: body.passphrase };
  } catch {
    return null;
  }
}
