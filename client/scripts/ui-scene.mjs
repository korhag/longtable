import { spawn } from 'node:child_process';
import { deflateSync, crc32 } from 'node:zlib';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
const client = join(root, 'client');
const work = await mkdtemp(join(tmpdir(), 'longtable-ui-'));
const dataDir = join(work, 'data');
const assetsDir = join(work, 'assets');
const packPath = join(work, 'gate.zip');
await mkdir(dataDir);
await mkdir(assetsDir);
await writeFile(packPath, gateZip());

const server = spawn(
  'cargo',
  ['run', '--quiet', '--manifest-path', 'server/Cargo.toml', '--offline'],
  {
    cwd: root,
    stdio: ['ignore', 'pipe', 'pipe'],
    env: {
      ...process.env,
      RPG_BIND: '127.0.0.1:8080',
      RPG_DATA_DIR: dataDir,
      RPG_ASSETS_DIR: assetsDir,
      RPG_SYSTEMS_DIR: join(root, 'systems'),
      RPG_PASSPHRASE: 'table',
      RPG_GM_KEY: 'testkey',
    },
  },
);
const vite = spawn('npx', ['vite', '--host', '127.0.0.1', '--port', '5174', '--strictPort'], {
  cwd: client,
  stdio: ['ignore', 'pipe', 'pipe'],
});
const chrome = spawn(
  'chromium',
  [
    '--headless=new',
    '--remote-debugging-port=9222',
    '--user-data-dir=' + join(work, 'chrome'),
    '--no-sandbox',
    '--use-gl=angle',
    '--use-angle=swiftshader',
    '--enable-webgl',
    '--ignore-gpu-blocklist',
    '--enable-unsafe-swiftshader',
    '--window-size=1400,900',
    'about:blank',
  ],
  { stdio: ['ignore', 'pipe', 'pipe'] },
);

let serverLog = '';
server.stdout.on('data', (chunk) => {
  serverLog += chunk;
});
server.stderr.on('data', (chunk) => {
  serverLog += chunk;
});

try {
  await waitForHttp('http://127.0.0.1:8080/api/host', 120);
  await waitForHttp('http://127.0.0.1:5174/', 60);
  const browser = await browserSocket();
  const gm = await openPage(browser, 'gm');
  const player = await openPage(browser, 'player');
  await gm.goto('http://127.0.0.1:5174/');
  await player.goto('http://127.0.0.1:5174/');

  await gm.clickButton('Continue as host');
  await gm.waitFor('Play');
  await player.clickButton('Create account');
  await player.fill('input[autocomplete="username"]', 'ada');
  await player.fill('input[type="password"]', 'secret');
  await player.clickSubmit('Create account');
  await player.waitFor('Signed in as ada');

  await gm.clickButton('Settings');
  await gm.clickButton('Account');
  await gm.fill('input[placeholder="Friend\'s name"]', 'ada');
  await gm.clickSubmit('Add');
  await player.reload();
  await player.waitFor('wants to be friends');
  await player.clickButton('Accept');
  await player.clickButton('Request a seat');
  await gm.waitFor('wants a seat');
  await gm.clickButton('Always');
  await player.reload();
  await player.waitFor('Leave');

  await gm.clickLabeled('Close');
  await gm.clickButton('Map');
  await gm.setFile(packPath);
  await gm.waitFor('Floor');
  await player.waitForSnapshot((value) => value.catalogRevision === '2');

  await gm.fill('[aria-label="Draft name"]', 'Gate');
  await gm.fill('[aria-label="Width"]', '12');
  await gm.fill('[aria-label="Height"]', '12');
  await gm.clickButton('New');
  await gm.waitForEnabled('Put on the table');
  await gm.clickButton('Floor');
  await gm.clickButton('Fit');
  await delay(400);
  await gm.clickCanvas(0.5, 0.48);
  await gm.clickButton('Objects');
  await gm.clickButton('Barrel');
  await gm.clickCanvas(0.56, 0.52);
  await gm.clickButton('Cast');
  await gm.clickButton('Bandit');
  await gm.clickButton('Put on the table');
  await gm.waitFor('On the table');
  const published = await player.waitForSnapshot((value) => value.scene?.name === 'Gate');
  const terrains = published.scene?.layout?.terrains ?? [];
  const props = published.scene?.layout?.props ?? [];
  if (!terrains.includes('gate/floor')) throw new Error(`the floor click did not paint; terrains are ${JSON.stringify(terrains)}`);
  if (!props.some((prop) => prop.asset === 'gate/barrel')) throw new Error('the barrel click did not place');

  await gm.clickButton('Play');
  await gm.fill('input[placeholder="Token name"]', 'Ada');
  await gm.clickButton('Add');
  await gm.fill('input[placeholder="Token name"]', 'Lurker');
  await gm.clickButton('Add');
  await gm.clickButton('Fit');
  await delay(500);
  let hidden = '';
  for (const [x, y] of [
    [0.5, 0.5],
    [0.55, 0.42],
    [0.5, 0.4],
    [0.6, 0.46],
    [0.48, 0.5],
    [0.66, 0.38],
    [0.52, 0.36],
  ]) {
    await gm.clickCanvas(x, y);
    hidden = await gm.eval(`document.querySelector('.display.text-lg')?.textContent?.trim() ?? ''`);
    if (hidden === 'Ada' || hidden === 'Lurker') break;
  }
  if (hidden !== 'Ada' && hidden !== 'Lurker') throw new Error(`the click did not select a figure (${hidden})`);
  const hiddenClick = await gm.eval(`(() => {
    const section = [...document.querySelectorAll('section')].find((node) => node.textContent.includes('sight'));
    const button = [...(section?.querySelectorAll('button') ?? [])].find((node) => node.textContent.trim() === 'Hide');
    if (!button) return false;
    button.click();
    return true;
  })()`);
  if (!hiddenClick) throw new Error(`the figure panel has no Hide button\n${await gm.text()}`);
  await gm.waitFor('Hidden from players');
  const shown = hidden === 'Ada' ? 'Lurker' : 'Ada';
  const hiddenView = await player.waitForSnapshot((value) => {
    const names = (value.tokens ?? []).map((token) => token.name);
    return names.includes(shown) && !names.includes(hidden);
  });
  const visible = hiddenView.tokens.find((token) => token.name === shown);
  if (visible?.castAsset !== 'gate/bandit') throw new Error(`cast picture missing on ${shown}`);

  await gm.chooseOption('ada');
  await gm.waitForSnapshot((value) => value.viewAs && !(value.tokens ?? []).some((token) => token.name === hidden));
  await gm.clickButton('Your eyes');
  await gm.waitForSnapshot((value) => value.viewAs == null && (value.tokens ?? []).some((token) => token.name === hidden));

  const before = JSON.parse(await readFile(join(dataDir, 'state.json'), 'utf8'));
  await gm.clickButton('Map');
  await gm.clickButton('Put on the table');
  await gm.waitFor('On the table');
  await delay(800);
  const after = JSON.parse(await readFile(join(dataDir, 'state.json'), 'utf8'));
  if (!after.scene?.layout?.terrains?.includes('gate/floor')) throw new Error('the painted tile is not in the save');
  if (!after.scene.layout.props?.some((prop) => prop.asset === 'gate/barrel')) throw new Error('the barrel is not in the save');
  if (after.scene.id !== before.scene.id) throw new Error('publishing again changed the scene id');
  const savedHidden = after.tokens.find((token) => token.name === hidden);
  const savedShown = after.tokens.find((token) => token.name === shown);
  if (!savedHidden?.hidden || savedShown?.cast_asset !== 'gate/bandit') throw new Error('the save lost the hidden figure or the cast picture');

  console.log('UI_OK');
  console.log('renderer: headless Chromium SwiftShader. This checks the buttons, the published scene, and the second seat. It is not a discrete-GPU frame measurement.');
} catch (err) {
  console.error(serverLog.slice(-2000));
  throw err;
} finally {
  chrome.kill('SIGTERM');
  vite.kill('SIGTERM');
  server.kill('SIGTERM');
  await delay(300);
  await rm(work, { recursive: true, force: true });
}

async function waitForHttp(url, attempts) {
  for (let attempt = 0; attempt < attempts; attempt += 1) {
    try {
      const response = await fetch(url);
      if (response.status < 500) return;
    } catch {
      // The process is still starting.
    }
    await delay(500);
  }
  throw new Error(`${url} did not answer`);
}

async function browserSocket() {
  for (let attempt = 0; attempt < 40; attempt += 1) {
    try {
      const version = await fetch('http://127.0.0.1:9222/json/version').then((response) => response.json());
      return connect(version.webSocketDebuggerUrl);
    } catch {
      await delay(250);
    }
  }
  throw new Error('chromium did not open a debugging port');
}

function connect(url) {
  const ws = new WebSocket(url);
  const pending = new Map();
  let next = 0;
  const ready = new Promise((resolve, reject) => {
    ws.addEventListener('open', () => resolve());
    ws.addEventListener('error', () => reject(new Error('browser socket failed')));
  });
  ws.addEventListener('message', (event) => {
    const message = JSON.parse(event.data);
    if (message.id && pending.has(message.id)) pending.get(message.id)(message);
  });
  return ready.then(() => ({
    send(method, params = {}, sessionId) {
      const id = ++next;
      return new Promise((resolve, reject) => {
        pending.set(id, (message) => {
          pending.delete(id);
          if (message.error) reject(new Error(`${method}: ${JSON.stringify(message.error)}`));
          else resolve(message.result);
        });
        ws.send(JSON.stringify({ id, method, params, sessionId }));
      });
    },
  }));
}

async function openPage(browser, name) {
  const context = await browser.send('Target.createBrowserContext');
  const created = await browser.send('Target.createTarget', {
    url: 'about:blank',
    browserContextId: context.browserContextId,
    width: 1400,
    height: 900,
  });
  const attached = await browser.send('Target.attachToTarget', { targetId: created.targetId, flatten: true });
  const sessionId = attached.sessionId;
  const send = (method, params) => browser.send(method, params, sessionId);
  await send('Page.enable');
  await send('Runtime.enable');
  await send('DOM.enable');
  await send('Page.addScriptToEvaluateOnNewDocument', {
    source: `(() => {
      window.__frames = [];
      const Native = WebSocket;
      function Wrapped(url, protocols) {
        const socket = protocols === undefined ? new Native(url) : new Native(url, protocols);
        socket.addEventListener('message', (event) => {
          if (typeof event.data === 'string' && event.data.includes('"snapshot"')) {
            window.__frames.push(event.data);
            if (window.__frames.length > 12) window.__frames.shift();
          }
        });
        return socket;
      }
      Wrapped.prototype = Native.prototype;
      Wrapped.CONNECTING = Native.CONNECTING;
      Wrapped.OPEN = Native.OPEN;
      Wrapped.CLOSING = Native.CLOSING;
      Wrapped.CLOSED = Native.CLOSED;
      window.WebSocket = Wrapped;
    })();`,
  });
  return {
    name,
    async goto(url) {
      await send('Page.navigate', { url });
      await this.waitFor('Take a seat');
    },
    async reload() {
      await send('Page.reload');
      for (let attempt = 0; attempt < 40; attempt += 1) {
        const state = await this.eval(`document.readyState`);
        if (state === 'complete') {
          await delay(500);
          return;
        }
        await delay(200);
      }
    },
    async eval(expression) {
      const result = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
      if (result.exceptionDetails) throw new Error(`${name}: ${JSON.stringify(result.exceptionDetails.text ?? result.exceptionDetails)}`);
      return result.result.value;
    },
    async clickIn(selector, text) {
      const clicked = await this.eval(`(() => {
        const node = [...document.querySelectorAll(${JSON.stringify(selector)})].find((button) => button.textContent.trim() === ${JSON.stringify(text)} && !button.disabled);
        if (!node) return false;
        node.click();
        return true;
      })()`);
      if (!clicked) throw new Error(`${name} could not find ${text} in ${selector}\n${await this.text()}`);
      await delay(300);
    },
    async clickLabeled(label) {
      await this.eval(`document.querySelector(${JSON.stringify(`[aria-label="${label}"]`)})?.click()`);
      await delay(200);
    },
    async clickButton(text) {
      const clicked = await this.eval(`(() => {
        const node = [...document.querySelectorAll('button')].find((button) => button.textContent.trim() === ${JSON.stringify(text)} && !button.disabled);
        if (!node) return false;
        node.click();
        return true;
      })()`);
      if (!clicked) throw new Error(`${name} could not find the button ${text}\n${await this.text()}`);
      await delay(250);
    },
    async clickSubmit(text) {
      const clicked = await this.eval(`(() => {
        const node = [...document.querySelectorAll('button[type="submit"]')].find((button) => button.textContent.trim() === ${JSON.stringify(text)});
        if (!node) return false;
        node.click();
        return true;
      })()`);
      if (!clicked) throw new Error(`${name} could not submit ${text}\n${await this.text()}`);
      await delay(400);
    },
    async fill(selector, value) {
      const wrote = await this.eval(`(() => {
        const node = document.querySelector(${JSON.stringify(selector)});
        if (!node) return false;
        node.focus();
        node.value = ${JSON.stringify(value)};
        node.dispatchEvent(new Event('input', { bubbles: true }));
        node.dispatchEvent(new Event('change', { bubbles: true }));
        return true;
      })()`);
      if (!wrote) throw new Error(`${name} could not fill ${selector}\n${await this.text()}`);
    },
    async setFile(path) {
      const doc = await send('DOM.getDocument');
      const found = await send('DOM.querySelector', { nodeId: doc.root.nodeId, selector: 'input[type="file"][accept*="zip"]' });
      if (!found.nodeId) throw new Error('the pack file input is not on the page');
      await send('DOM.setFileInputFiles', { nodeId: found.nodeId, files: [path] });
      await delay(500);
    },
    async clickCanvas(xRatio, yRatio) {
      const point = await this.eval(`(() => {
        const canvas = [...document.querySelectorAll('canvas')].find((node) => !node.className && node.width > 200);
        if (!canvas) return null;
        const box = canvas.getBoundingClientRect();
        const x = box.x + box.width * ${xRatio};
        const y = box.y + box.height * ${yRatio};
        const init = { clientX: x, clientY: y, button: 0, bubbles: true, cancelable: true, pointerId: 1, pointerType: 'mouse', isPrimary: true, view: window };
        canvas.setPointerCapture = canvas.setPointerCapture ?? (() => {});
        canvas.dispatchEvent(new PointerEvent('pointerdown', init));
        canvas.dispatchEvent(new PointerEvent('pointerup', init));
        return true;
      })()`);
      if (!point) throw new Error(`${name} has no map canvas`);
      await delay(200);
    },
    async chooseOption(label) {
      const picked = await this.eval(`(() => {
        const select = [...document.querySelectorAll('select')].find((node) => [...node.options].some((option) => option.textContent.trim() === 'ada'));
        if (!select) return false;
        const option = [...select.options].find((item) => item.textContent.trim() === ${JSON.stringify(label)} || item.value === ${JSON.stringify(label)});
        if (!option && ${JSON.stringify(label)} !== '') return false;
        select.value = option ? option.value : '';
        select.dispatchEvent(new Event('change', { bubbles: true }));
        return true;
      })()`);
      if (!picked) throw new Error(`${name} could not look through ${label || 'the GM'}`);
    },
    async waitFor(text) {
      const needle = text.toLowerCase();
      for (let attempt = 0; attempt < 50; attempt += 1) {
        if ((await this.text()).toLowerCase().includes(needle)) return;
        await delay(250);
      }
      throw new Error(`${name} never showed ${text}\n${await this.text()}`);
    },
    async waitForEnabled(text) {
      for (let attempt = 0; attempt < 40; attempt += 1) {
        const ready = await this.eval(`[...document.querySelectorAll('button')].some((button) => button.textContent.trim() === ${JSON.stringify(text)} && !button.disabled)`);
        if (ready) return;
        await delay(250);
      }
      throw new Error(`${name} never enabled ${text}`);
    },
    async waitForSnapshot(ready) {
      for (let attempt = 0; attempt < 40; attempt += 1) {
        const frames = await this.eval(`window.__frames ?? []`);
        for (const text of [...frames].reverse()) {
          try {
            const value = JSON.parse(text);
            if (value.type === 'snapshot' && ready(value)) return value;
          } catch {
            // A frame can be partial while the socket is still writing.
          }
        }
        await delay(250);
      }
      const last = await this.eval(`(() => {
        const frames = window.__frames ?? [];
        for (let index = frames.length - 1; index >= 0; index -= 1) {
          try {
            const value = JSON.parse(frames[index]);
            if (value.type === 'snapshot') return JSON.stringify((value.tokens ?? []).map((token) => token.name));
          } catch { /* ignore */ }
        }
        return 'none';
      })()`);
      throw new Error(`${name} snapshot did not match (tokens ${last})\n${await this.text()}`);
    },
    text() {
      return this.eval(`document.body?.innerText?.slice(0, 1200) ?? ''`);
    },
  };
}

function png(width, height, alpha) {
  const raw = Buffer.alloc((width * 4 + 1) * height);
  for (let y = 0; y < height; y += 1) {
    const row = y * (width * 4 + 1);
    for (let x = 0; x < width; x += 1) {
      const index = row + 1 + x * 4;
      raw[index] = 180;
      raw[index + 1] = 90;
      raw[index + 2] = 40;
      raw[index + 3] = 255;
    }
  }
  raw[4] = alpha;
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8;
  ihdr[9] = 6;
  const signature = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
  return Buffer.concat([signature, chunk('IHDR', ihdr), chunk('IDAT', deflateSync(raw)), chunk('IEND', Buffer.alloc(0))]);
}

function chunk(type, data) {
  const length = Buffer.alloc(4);
  length.writeUInt32BE(data.length);
  const body = Buffer.concat([Buffer.from(type), data]);
  const sum = Buffer.alloc(4);
  sum.writeUInt32BE(crc32(body) >>> 0);
  return Buffer.concat([length, body, sum]);
}

function gateZip() {
  return zipStore({
    'pack.json': packManifest(),
    'tiles/floor.png': png(128, 128, 255),
    'objects/barrel.png': png(32, 48, 0),
    'cast/bandit/north.png': png(32, 64, 0),
    'cast/bandit/east.png': png(32, 64, 0),
    'cast/bandit/south.png': png(32, 64, 0),
    'cast/bandit/west.png': png(32, 64, 0),
  });
}

function packManifest() {
  return Buffer.from(`{
    "schemaVersion": 2,
    "id": "gate",
    "name": "Gate",
    "visualProfile": "dimetric-30-v1",
    "pixelsPerCell": 128,
    "filtering": "linear",
    "tiles": [{
      "id": "floor",
      "name": "Floor",
      "kind": "ground",
      "sourceSizePx": [128, 128],
      "coverageCells": [1, 1],
      "files": ["tiles/floor.png"]
    }],
    "objects": [{
      "id": "barrel",
      "name": "Barrel",
      "kind": "static",
      "placement": "cell",
      "footprintCells": [1, 1],
      "blocksMovement": true,
      "blocksVision": false,
      "pixelsPerWorldUnit": 128,
      "renderLayers": [{
        "kind": "upright",
        "file": "objects/barrel.png",
        "sizePx": [32, 48],
        "pivotPx": [16, 46]
      }]
    }],
    "cast": [{
      "id": "bandit",
      "name": "Bandit",
      "pixelsPerWorldUnit": 128,
      "views": {
        "north": {"file": "cast/bandit/north.png", "sizePx": [32, 64], "pivotPx": [16, 60]},
        "east": {"file": "cast/bandit/east.png", "sizePx": [32, 64], "pivotPx": [16, 60]},
        "south": {"file": "cast/bandit/south.png", "sizePx": [32, 64], "pivotPx": [16, 60]},
        "west": {"file": "cast/bandit/west.png", "sizePx": [32, 64], "pivotPx": [16, 60]}
      }
    }]
  }`);
}

function zipStore(files) {
  const locals = [];
  const centrals = [];
  let offset = 0;
  for (const [name, data] of Object.entries(files)) {
    const fileName = Buffer.from(name);
    const local = Buffer.alloc(30);
    local.writeUInt32LE(0x04034b50, 0);
    local.writeUInt16LE(20, 4);
    local.writeUInt16LE(0, 8);
    local.writeUInt32LE(crc32(data) >>> 0, 14);
    local.writeUInt32LE(data.length, 18);
    local.writeUInt32LE(data.length, 22);
    local.writeUInt16LE(fileName.length, 26);
    const central = Buffer.alloc(46);
    central.writeUInt32LE(0x02014b50, 0);
    central.writeUInt16LE(20, 4);
    central.writeUInt16LE(20, 6);
    central.writeUInt32LE(crc32(data) >>> 0, 16);
    central.writeUInt32LE(data.length, 20);
    central.writeUInt32LE(data.length, 24);
    central.writeUInt16LE(fileName.length, 28);
    central.writeUInt32LE(offset, 42);
    locals.push(Buffer.concat([local, fileName, data]));
    centrals.push(Buffer.concat([central, fileName]));
    offset += local.length + fileName.length + data.length;
  }
  const directory = Buffer.concat(centrals);
  const end = Buffer.alloc(22);
  end.writeUInt32LE(0x06054b50, 0);
  end.writeUInt16LE(Object.keys(files).length, 8);
  end.writeUInt16LE(Object.keys(files).length, 10);
  end.writeUInt32LE(directory.length, 12);
  end.writeUInt32LE(offset, 16);
  return Buffer.concat([...locals, directory, end]);
}
