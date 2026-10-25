// ------------------------------------------------------------
// Module: bits
// Purpose: Read the explored-fog bitmap the server sends.
//          Row-major, x fastest, high bit of each byte first.
// ------------------------------------------------------------

export function decodeBits(encoded: string): Uint8Array {
  const binary = atob(encoded);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes;
}

export function bitAt(bytes: Uint8Array, index: number): boolean {
  const byte = bytes[index >> 3];
  if (byte === undefined) return false;
  const bit = 7 - (index & 7);
  return (byte & (1 << bit)) !== 0;
}
