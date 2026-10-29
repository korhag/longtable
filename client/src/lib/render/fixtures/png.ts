// ------------------------------------------------------------
// Module: png
// Purpose: Write a tiny uncompressed PNG so diagnostic fixtures
//          do not depend on an art tool or a canvas library.
// ------------------------------------------------------------

export function png(width: number, height: number, pixel: (x: number, y: number) => [number, number, number, number]) {
  const row = width * 4 + 1;
  const raw = new Uint8Array(row * height);
  for (let y = 0; y < height; y += 1) {
    const start = y * row;
    raw[start] = 0;
    for (let x = 0; x < width; x += 1) {
      const [r, g, b, a] = pixel(x, y);
      const index = start + 1 + x * 4;
      raw[index] = r;
      raw[index + 1] = g;
      raw[index + 2] = b;
      raw[index + 3] = a;
    }
  }
  const idat = zlibStore(raw);
  const bytes = new Uint8Array(8 + chunk('IHDR', ihdr(width, height)).length + chunk('IDAT', idat).length + chunk('IEND', new Uint8Array()).length);
  bytes.set(signature(), 0);
  let offset = 8;
  for (const part of [chunk('IHDR', ihdr(width, height)), chunk('IDAT', idat), chunk('IEND', new Uint8Array())]) {
    bytes.set(part, offset);
    offset += part.length;
  }
  return bytes;
}

export function pngSize(bytes: Uint8Array) {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  return { width: view.getUint32(16), height: view.getUint32(20) };
}

function signature() {
  return Uint8Array.from([137, 80, 78, 71, 13, 10, 26, 10]);
}

function ihdr(width: number, height: number) {
  const data = new Uint8Array(13);
  const view = new DataView(data.buffer);
  view.setUint32(0, width);
  view.setUint32(4, height);
  data[8] = 8;
  data[9] = 6;
  return data;
}

function chunk(kind: string, data: Uint8Array) {
  const out = new Uint8Array(12 + data.length);
  const view = new DataView(out.buffer);
  view.setUint32(0, data.length);
  out[4] = kind.charCodeAt(0);
  out[5] = kind.charCodeAt(1);
  out[6] = kind.charCodeAt(2);
  out[7] = kind.charCodeAt(3);
  out.set(data, 8);
  view.setUint32(8 + data.length, crc(out.subarray(4, 8 + data.length)));
  return out;
}

function zlibStore(data: Uint8Array) {
  const chunks: Uint8Array[] = [Uint8Array.from([0x78, 0x01])];
  let offset = 0;
  while (offset < data.length) {
    const size = Math.min(65535, data.length - offset);
    const last = offset + size >= data.length;
    const header = new Uint8Array(5);
    header[0] = last ? 1 : 0;
    header[1] = size & 0xff;
    header[2] = (size >> 8) & 0xff;
    header[3] = (~size) & 0xff;
    header[4] = ((~size) >> 8) & 0xff;
    chunks.push(header, data.subarray(offset, offset + size));
    offset += size;
  }
  const sum = adler32(data);
  const tail = new Uint8Array(4);
  new DataView(tail.buffer).setUint32(0, sum);
  chunks.push(tail);
  const length = chunks.reduce((total, part) => total + part.length, 0);
  const out = new Uint8Array(length);
  let cursor = 0;
  for (const part of chunks) {
    out.set(part, cursor);
    cursor += part.length;
  }
  return out;
}

function adler32(data: Uint8Array) {
  let a = 1;
  let b = 0;
  for (const byte of data) {
    a = (a + byte) % 65521;
    b = (b + a) % 65521;
  }
  return ((b << 16) | a) >>> 0;
}

const CRC_TABLE = (() => {
  const table = new Uint32Array(256);
  for (let index = 0; index < 256; index += 1) {
    let c = index;
    for (let bit = 0; bit < 8; bit += 1) c = (c & 1) ? (0xedb88320 ^ (c >>> 1)) : (c >>> 1);
    table[index] = c >>> 0;
  }
  return table;
})();

function crc(data: Uint8Array) {
  let c = 0xffffffff;
  for (const byte of data) c = CRC_TABLE[(c ^ byte) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}
