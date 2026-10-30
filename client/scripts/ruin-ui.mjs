import { spawn } from 'node:child_process';
import { crc32 } from 'node:zlib';
import { mkdir, mkdtemp, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join, relative } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { fileURLToPath } from 'node:url';

const cameraJs = `
  function mapCanvas() {
    return [...document.querySelectorAll('canvas')].find((node) => !node.className && node.width > 200) ?? null;
  }
  function normalize(value) {
    const length = Math.hypot(value.x, value.y, value.z) || 1;
    return { x: value.x / length, y: value.y / length, z: value.z / length };
  }
  function cross(a, b) {
    return { x: a.y * b.z - a.z * b.y, y: a.z * b.x - a.x * b.z, z: a.x * b.y - a.y * b.x };
  }
  function dot(a, b) {
    return a.x * b.x + a.y * b.y + a.z * b.z;
  }
  function basis() {
    const azimuth = 45 * Math.PI / 180;
    const elevation = 30 * Math.PI / 180;
    const sinE = Math.sin(elevation);
    const cosE = Math.cos(elevation);
    const sinA = Math.sin(azimuth);
    const cosA = Math.cos(azimuth);
    const forward = normalize({ x: sinA * cosE, y: -sinE, z: -cosA * cosE });
    const rawUp = { x: sinA * cosE * sinE, y: cosE * cosE, z: -cosA * cosE * sinE };
    let up = normalize(rawUp);
    const right = normalize(cross(forward, up));
    up = normalize(cross(right, forward));
    return { right, up };
  }
  function insetsOf(canvas) {
    return {
      left: document.querySelector('aside.dock:not(.right)')?.offsetWidth ?? 0,
      right: document.querySelector('aside.dock.right')?.offsetWidth ?? 0,
      top: document.querySelector('div.pointer-events-auto')?.offsetHeight ?? 0,
      bottom: document.querySelector('div.absolute.inset-x-0.bottom-0')?.offsetHeight ?? 0,
      canvasW: canvas.clientWidth || 1,
      canvasH: canvas.clientHeight || 1,
    };
  }
  function frame(canvas, mapW, mapH, rise, insets) {
    const { right, up } = basis();
    const focus = { x: mapW / 2, y: 0, z: mapH / 2 };
    let maxRight = 0;
    let maxUp = 0;
    for (const y of [0, rise]) {
      for (const x of [0, mapW]) {
        for (const z of [0, mapH]) {
          const rel = { x: x - focus.x, y, z: z - focus.z };
          maxRight = Math.max(maxRight, Math.abs(dot(rel, right)));
          maxUp = Math.max(maxUp, Math.abs(dot(rel, up)));
        }
      }
    }
    const spanW = maxRight * 2;
    const spanH = maxUp * 2;
    const visW = Math.max(1, insets.canvasW - insets.left - insets.right);
    const visH = Math.max(1, insets.canvasH - insets.top - insets.bottom);
    const aspect = visW / visH;
    const pad = 1.08;
    return Math.max(Math.max(spanH * pad, (spanW * pad) / aspect) * (insets.canvasH / visH), 8);
  }
`;

const root = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
const client = join(root, 'client');
const work = await mkdtemp(join(tmpdir(), 'longtable-ruin-ui-'));
const dataDir = join(work, 'data');
const assetsDir = join(work, 'assets');
const packPath = join(work, 'ruin.zip');
await mkdir(dataDir);
await mkdir(assetsDir);
await writeFile(packPath, await ruinZip());

const bind = '127.0.0.1:8097';
let serverLog = '';
let server = startTable();
watch(server);
const vite = spawn('npx', ['vite', '--host', '127.0.0.1', '--port', '5175', '--strictPort'], {
  cwd: client,
  stdio: ['ignore', 'pipe', 'pipe'],
  env: { ...process.env, LONGTABLE_API: 'http://127.0.0.1:8097' },
});
const chrome = spawn('chromium', [
  '--headless=new',
  '--remote-debugging-port=9224',
  '--user-data-dir=' + join(work, 'chrome'),
  '--no-sandbox',
  '--use-gl=angle',
  '--use-angle=swiftshader',
  '--enable-webgl',
  '--ignore-gpu-blocklist',
  '--enable-unsafe-swiftshader',
  '--window-size=1400,900',
  'about:blank',
], { stdio: ['ignore', 'pipe', 'pipe'] });

try {
  await waitForHttp('http://127.0.0.1:8097/api/host', 120);
  await waitForHttp('http://127.0.0.1:5175/', 60);
  const browser = await browserSocket();
  const gm = await openPage(browser, 'gm');
  const player = await openPage(browser, 'player');
  await gm.goto('http://127.0.0.1:5175/');
  await player.goto('http://127.0.0.1:5175/');
  await gm.clickButton('Continue as host');
  await delay(600);
  if (!(await gm.text()).toLowerCase().includes('play') && (await gm.text()).toLowerCase().includes('sit down')) {
    await gm.clickButton('Sit down');
  }
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
  await gm.waitFor('Grass');

  await gm.fill('[aria-label="Draft name"]', 'Ruin');
  await gm.fill('[aria-label="Width"]', '12');
  await gm.fill('[aria-label="Height"]', '12');
  await gm.clickButton('New');
  await gm.waitForEnabled('Put on the table as a new map');
  await gm.clickButton('Fit');
  await delay(400);
  await gm.rememberFit(12, 12, 1);

  await gm.clickButton('Tiles');
  await gm.clickButton('Grass');
  await gm.waitFor('Seam with');
  await gm.clickButton('1');
  await paintBlock(gm, 0, 0, 6, 6);
  await gm.clickButton('Dirt');
  await paintBlock(gm, 6, 0, 12, 6);
  await gm.clickButton('Cobble');
  await paintBlock(gm, 0, 6, 6, 12);
  await gm.clickButton('Ruin floor');
  await paintBlock(gm, 6, 6, 12, 12);
  await gm.clickButton('Leaves');
  await gm.clickWorld(1.5, 0, 1.5);

  await gm.clickButton('Objects');
  await gm.clickButton('Oak');
  await gm.waitFor('The gold mark is the pivot');
  await gm.clickWorld(3.25, 0, 8.25);
  await gm.clickButton('Barrel');
  await gm.clickWorld(5.5, 0, 3.5);
  await gm.clickButton('Table');
  await gm.clickWorld(6.5, 0, 8.5);
  await gm.clickButton('Hedge');
  await gm.clickWorld(6.5, 0, 2.5);
  await gm.clickButton('Wall');
  for (const [x, z] of [[1.5, 5.12], [3.5, 5.12], [4.08, 5.5], [4.08, 6.5], [4.08, 7.5]]) {
    await gm.clickWorld(x, 0, z);
  }
  await gm.clickButton('Door');
  await gm.clickWorld(2.5, 0, 5.12);

  await gm.clickButton('Raise');
  await gm.clickButton('Step');
  await gm.clickWorld(8.5, 0, 2.5);
  await gm.clickButton('Rise');
  await gm.clickWorld(9.5, 0, 2.5);
  await gm.clickButton('Stair');
  await gm.clickWorld(7.82, 0, 2.5);
  await gm.clickWorld(8.82, 0, 2.5);

  await gm.clickButton('Cast');
  await gm.clickButton('Scout');
  await gm.clickButton('Put on the table as a new map');
  await gm.waitFor('On the table');
  const published = await player.waitForSnapshot((value) => value.scene?.name === 'Ruin');
  assertRuin(published, 'publish');

  await gm.clickButton('Play');
  await gm.clickButton('Fit');
  await delay(400);
  await gm.clickButton('Campaign');
  await gm.clickButton('New');
  for (let attempt = 0; attempt < 40; attempt += 1) {
    const ready = await gm.eval(`!!document.querySelector('[aria-label="Character name"]')`);
    if (ready) break;
    await delay(250);
    if (attempt === 39) throw new Error(`the new character did not appear\n${await gm.text()}`);
  }
  await gm.fill('[aria-label="Character name"]', 'Ada');
  await gm.waitForSnapshot((value) => (value.characters ?? []).some((character) => character.name === 'Ada'));
  const owned = await gm.eval(`(() => {
    const select = [...document.querySelectorAll('select')].find((node) => node.options[0]?.textContent.trim() === 'No owner');
    const option = [...(select?.options ?? [])].find((item) => item.textContent.trim() === 'ada');
    if (!select || !option) return false;
    select.value = option.value;
    select.dispatchEvent(new Event('change', { bubbles: true }));
    return true;
  })()`);
  if (!owned) throw new Error(`Ada could not be given to the seated player\n${await gm.text()}`);
  await gm.clickButton('Picture');
  await gm.waitFor('Open Map and click the picture');
  await gm.clickButton('Map');
  await gm.clickButton('Cast');
  await gm.clickButton('Scout');
  await gm.waitForSnapshot((value) => (value.characters ?? []).some((character) => character.name === 'Ada' && character.castAssetId === 'ruin/scout'));
  await gm.clickButton('Campaign');
  await gm.clickButton('Fit');
  await delay(300);
  await gm.rememberFit(12, 12, 3);
  await placeCharacter(gm, player, 9.5, 2, 2.5);
  await gm.clickButton('Play');
  await gm.clickButton('Fit');
  await delay(300);
  await gm.rememberFit(12, 12, 3);
  await gm.clickButton('Map');
  await gm.clickButton('Cast');
  await gm.clickButton('Bandit');
  await gm.clickButton('Play');
  await gm.clickButton('Fit');
  await delay(300);
  await gm.rememberFit(12, 12, 3);
  await placeFigure(gm, player, 'Bram', 2.5, 0, 2.5);
  await placeFigure(gm, player, 'Shade', 3.5, 0, 6.5);
  await placeFigure(gm, player, 'Wren', 4.5, 0, 8.5);
  await placeFigure(gm, player, 'Lurker', 10.5, 0, 10.5);
  await delay(700);
  const outline = await gm.creamNear(4.5, 0.45, 8.5, 48);
  if (outline.cream < 4 || outline.green < 8) {
    throw new Error(`the oak did not outline Wren (${outline.cream} cream, ${outline.green} green)`);
  }
  for (const [name, x, z] of [['Bram', 2.5, 2.5], ['Shade', 3.5, 6.5], ['Wren', 4.5, 8.5]]) {
    await selectFigure(gm, name, x, z);
    await gm.clickButton('Sight on');
    await gm.waitFor('Sight off');
  }
  await selectFigure(gm, 'Lurker', 10.5, 10.5);
  const hiddenClick = await gm.eval(`(() => {
    const section = [...document.querySelectorAll('section')].find((node) => node.textContent.includes('sight'));
    const button = [...(section?.querySelectorAll('button') ?? [])].find((node) => node.textContent.trim() === 'Hide');
    if (!button) return false;
    button.click();
    return true;
  })()`);
  if (!hiddenClick) throw new Error(`the figure panel has no Hide button\n${await gm.text()}`);
  await gm.waitFor('Hidden from players');
  const hidden = 'Lurker';
  const seated = await player.waitForSnapshot((value) => {
    const names = (value.tokens ?? []).map((token) => token.name);
    return names.includes('Ada') && names.includes('Bram') && names.includes('Wren') && !names.includes('Shade') && !names.includes('Lurker');
  });
  const ada = seated.tokens.find((token) => token.name === 'Ada');
  const bram = seated.tokens.find((token) => token.name === 'Bram');
  const wren = seated.tokens.find((token) => token.name === 'Wren');
  if (!ada || ada.castAsset !== 'ruin/scout' || !ada.characterId || ada.x !== 9.5 || ada.y !== 2.5) {
    throw new Error(`Ada is not the scout on the rise (${JSON.stringify(ada)})`);
  }
  if (!bram || bram.castAsset !== 'ruin/bandit' || bram.x !== 2.5 || bram.y !== 2.5) {
    throw new Error(`Bram is not beside the hedge (${JSON.stringify(bram)})`);
  }
  if (!wren || wren.castAsset !== 'ruin/bandit' || wren.x !== 4.5 || wren.y !== 8.5) {
    throw new Error(`Wren is not beside the oak (${JSON.stringify(wren)})`);
  }
  await gm.chooseOption('ada');
  await gm.waitForSnapshot((value) => {
    const names = (value.tokens ?? []).map((token) => token.name);
    return value.viewAs && names.includes('Wren') && names.includes('Bram') && !names.includes('Shade') && !names.includes('Lurker');
  });
  await gm.clickButton('Your eyes');

  await gm.clickButton('Campaign');
  const before = await gm.waitForSnapshot((value) => (value.scenes ?? []).some((scene) => scene.name === 'Ruin'));
  const beforeNames = (before.tokens ?? []).map((token) => token.name).sort().join(',');
  await gm.clickButton('Copy');
  const copied = await gm.waitForSnapshot((value) => value.scene?.name === 'Ruin' && (value.scenes ?? []).some((scene) => scene.name === 'Ruin copy'));
  if (copied.scenes.length !== before.scenes.length + 1) {
    throw new Error(`copy did not add one scene (${copied.scenes.length})`);
  }
  const afterNames = (copied.tokens ?? []).map((token) => token.name).sort().join(',');
  if (afterNames !== beforeNames) throw new Error(`copy moved the figures (${beforeNames} -> ${afterNames})`);

  await delay(800);
  server.kill('SIGTERM');
  for (let attempt = 0; attempt < 40; attempt += 1) {
    try {
      await fetch('http://127.0.0.1:8097/api/host');
    } catch {
      break;
    }
    await delay(200);
  }
  server = startTable();
  watch(server);
  await waitForHttp('http://127.0.0.1:8097/api/host', 120);
  await gm.reload();
  await delay(800);
  if (!(await gm.text()).toLowerCase().includes('play') && (await gm.text()).toLowerCase().includes('sit down')) {
    await gm.clickButton('Sit down');
  }
  await gm.waitFor('Play');
  await player.reload();
  const restored = await player.waitForSnapshot((value) => value.scene?.name === 'Ruin' && (value.tokens ?? []).every((token) => token.name !== hidden));
  assertRuin(restored, 'restart');
  const restoredNames = restored.tokens.map((token) => token.name);
  if (!restoredNames.includes('Ada') || !restoredNames.includes('Bram') || !restoredNames.includes('Wren') || restoredNames.includes('Shade')) {
    throw new Error(`a restart changed who the seat can see (${restoredNames.join(', ')})`);
  }

  console.log('RUIN_UI_OK');
  console.log('renderer: headless Chromium SwiftShader. This builds the 12 by 12 ruin scene through the buttons, checks Wren’s outline behind the oak, the second seat, and a restart. It is not a frame-time measurement.');
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

function assertRuin(snapshot, label) {
  const layout = snapshot.scene?.layout;
  const terrains = layout?.terrains ?? [];
  const props = (layout?.props ?? []).map((prop) => prop.asset);
  const pieces = layout?.pieces ?? [];
  const stairs = layout?.stairs ?? [];
  const decals = (layout?.decals ?? []).map((decal) => decal.asset);
  const heights = heightsOf(layout?.levels);
  const axes = new Set(pieces.map((piece) => piece.axis));
  const problems = [];
  for (const tile of ['ruin/grass', 'ruin/dirt', 'ruin/cobble', 'ruin/floor']) {
    if (!terrains.includes(tile)) problems.push(`missing ${tile}`);
  }
  if (!decals.includes('ruin/leaves')) problems.push('missing leaves');
  for (const asset of ['ruin/oak', 'ruin/barrel', 'ruin/table', 'ruin/hedge']) {
    if (!props.includes(asset)) problems.push(`missing ${asset}`);
  }
  if (!axes.has('h') || !axes.has('v')) problems.push(`walls are ${[...axes].join(',') || 'absent'}`);
  if (!pieces.some((piece) => piece.kind === 'door')) problems.push('missing door');
  if (!heights.includes(1) || !heights.includes(2)) problems.push(`heights ${heights.join(',') || 'flat'}`);
  const bytes = layout?.levels ? [...Buffer.from(layout.levels, 'base64')] : [];
  if (bytes[2 * 12 + 8] !== 1 || bytes[2 * 12 + 9] !== 2) problems.push(`the rise is not on (8,2) and (9,2)`);
  const named = (asset) => terrains.indexOf(asset);
  const cells = layout?.cells ? [...Buffer.from(layout.cells, 'base64')] : [];
  const expect = [
    ['ruin/grass', 0, 0, 6, 6],
    ['ruin/dirt', 6, 0, 12, 6],
    ['ruin/cobble', 0, 6, 6, 12],
    ['ruin/floor', 6, 6, 12, 12],
  ];
  for (const [asset, x0, z0, x1, z1] of expect) {
    const index = named(asset);
    for (let z = z0; z < z1; z += 1) {
      for (let x = x0; x < x1; x += 1) {
        if (cells[z * 12 + x] !== index) problems.push(`${asset} missing at ${x},${z}`);
      }
    }
  }
  const propAt = (asset, x, y) => (layout?.props ?? []).some((prop) => prop.asset === asset && prop.x === x && prop.y === y);
  if (!propAt('ruin/oak', 3, 8)) problems.push('the oak is not at (3, 8)');
  if (!propAt('ruin/barrel', 5, 3)) problems.push('the barrel is not at (5, 3)');
  if (!propAt('ruin/table', 6, 8)) problems.push('the table is not at (6, 8)');
  if (!propAt('ruin/hedge', 6, 2)) problems.push('the hedge is not at (6, 2)');
  const pieceAt = (kind, x, y, axis) => pieces.some((piece) => piece.kind === kind && piece.x === x && piece.y === y && piece.axis === axis);
  if (!pieceAt('wall', 1, 5, 'h') || !pieceAt('wall', 3, 5, 'h')) problems.push('the east-west walls are not on row 5');
  if (!pieceAt('door', 2, 5, 'h')) problems.push('the door is not between those walls');
  if (!pieceAt('wall', 4, 5, 'v') || !pieceAt('wall', 4, 6, 'v') || !pieceAt('wall', 4, 7, 'v')) problems.push('the north-south wall is not beside the door');
  const linked = (x, y, x2, y2) => stairs.some((stair) => (stair.x === x && stair.y === y && stair.x2 === x2 && stair.y2 === y2) || (stair.x === x2 && stair.y === y2 && stair.x2 === x && stair.y2 === y));
  if (!linked(7, 2, 8, 2) || !linked(8, 2, 9, 2)) problems.push('the stairs do not climb (7,2) to (9,2)');
  const leaf = (layout?.decals ?? []).find((decal) => decal.asset === 'ruin/leaves');
  if (!leaf || Math.abs(leaf.x - 1.5) > 0.2 || Math.abs(leaf.y - 1.5) > 0.2) problems.push('the leaves are not on (1, 1)');
  if (stairs.length < 2) problems.push(`${stairs.length} stairs`);
  if (problems.length) {
    throw new Error(`${label}: ${problems.slice(0, 8).join('; ')}\nterrains ${terrains.join(', ')}\nprops ${props.join(', ')}`);
  }
}

function heightsOf(levels) {
  if (!levels) return [];
  return [...new Set(Buffer.from(levels, 'base64'))].sort((a, b) => a - b);
}

function startTable() {
  return spawn('cargo', ['run', '--quiet', '--manifest-path', 'server/Cargo.toml', '--offline'], {
    cwd: root,
    stdio: ['ignore', 'pipe', 'pipe'],
    env: {
      ...process.env,
      RPG_BIND: bind,
      RPG_DATA_DIR: dataDir,
      RPG_ASSETS_DIR: assetsDir,
      RPG_SYSTEMS_DIR: join(root, 'systems'),
      RPG_PASSPHRASE: 'table',
      RPG_GM_KEY: 'testkey',
    },
  });
}

function watch(child) {
  child.stdout.on('data', (chunk) => {
    serverLog += chunk;
  });
  child.stderr.on('data', (chunk) => {
    serverLog += chunk;
  });
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
      const version = await fetch('http://127.0.0.1:9224/json/version').then((response) => response.json());
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
        const state = await this.eval('document.readyState');
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
    async chooseOption(label) {
      const picked = await this.eval(`(() => {
        const select = [...document.querySelectorAll('select')].find((node) => node.options[0]?.textContent.trim() === 'Yourself, the GM');
        const option = [...(select?.options ?? [])].find((item) => item.textContent.trim() === ${JSON.stringify(label)});
        if (!select || !option) return false;
        select.value = option.value;
        select.dispatchEvent(new Event('change', { bubbles: true }));
        return true;
      })()`);
      if (!picked) throw new Error(`${name} could not look through ${label}\n${await this.text()}`);
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
    async clickLabeled(label) {
      await this.eval(`document.querySelector(${JSON.stringify(`[aria-label="${label}"]`)})?.click()`);
      await delay(200);
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
      await delay(800);
    },
    async clickCanvas(xRatio, yRatio) {
      const point = await this.pointer(xRatio, yRatio, xRatio, yRatio);
      if (!point) throw new Error(`${name} has no map canvas`);
      await delay(120);
    },
    async rememberFit(mapW, mapH, rise) {
      const distance = await this.eval(`(() => {
        ${cameraJs}
        const canvas = mapCanvas();
        if (!canvas) return null;
        const insets = insetsOf(canvas);
        const distance = frame(canvas, ${mapW}, ${mapH}, ${rise}, insets);
        window.__fit = { distance, mapW: ${mapW}, mapH: ${mapH}, insets };
        return distance;
      })()`);
      if (!distance) throw new Error(`${name} could not frame the map`);
    },
    async clickWorld(x, y, z) {
      const point = await this.eval(`(() => {
        ${cameraJs}
        const fit = window.__fit;
        const canvas = mapCanvas();
        if (!fit || !canvas) return null;
        const box = canvas.getBoundingClientRect();
        const insets = fit.insets;
        const { right, up } = basis();
        const focus = { x: fit.mapW / 2, y: 0, z: fit.mapH / 2 };
        const rel = { x: ${x} - focus.x, y: ${y} - focus.y, z: ${z} - focus.z };
        const scale = insets.canvasH / fit.distance;
        const visibleX = (insets.left + (insets.canvasW - insets.right)) / 2;
        const visibleY = (insets.top + (insets.canvasH - insets.bottom)) / 2;
        const u = visibleX + scale * dot(rel, right);
        const v = visibleY - scale * dot(rel, up);
        const clientX = box.left + u;
        const clientY = box.top + v;
        const init = { clientX, clientY, button: 0, bubbles: true, cancelable: true, pointerId: 1, pointerType: 'mouse', isPrimary: true, view: window };
        canvas.setPointerCapture = canvas.setPointerCapture ?? (() => {});
        canvas.dispatchEvent(new PointerEvent('pointerdown', init));
        canvas.dispatchEvent(new PointerEvent('pointerup', init));
        return { u, v };
      })()`);
      if (!point) throw new Error(`${name} has no framed map for ${x},${z}`);
      await delay(30);
    },
    async creamNear(x, y, z, radius) {
      const point = await this.eval(`(() => {
        ${cameraJs}
        const fit = window.__fit;
        const canvas = mapCanvas();
        if (!fit?.insets || !canvas) return null;
        const box = canvas.getBoundingClientRect();
        const { right, up } = basis();
        const focus = { x: fit.mapW / 2, y: 0, z: fit.mapH / 2 };
        const rel = { x: ${x} - focus.x, y: ${y} - focus.y, z: ${z} - focus.z };
        const scale = fit.insets.canvasH / fit.distance;
        const visibleX = (fit.insets.left + (fit.insets.canvasW - fit.insets.right)) / 2;
        const visibleY = (fit.insets.top + (fit.insets.canvasH - fit.insets.bottom)) / 2;
        return { x: box.left + visibleX + scale * dot(rel, right), y: box.top + visibleY - scale * dot(rel, up) };
      })()`);
      if (!point) throw new Error(`${name} has no framed map to inspect`);
      const clip = {
        x: Math.max(0, Math.min(1300, point.x - radius)),
        y: Math.max(0, Math.min(800, point.y - radius)),
        width: radius * 2,
        height: radius * 2,
        scale: 1,
      };
      const shot = await send('Page.captureScreenshot', { format: 'png', clip, fromSurface: true });
      return this.eval(`(async () => {
        const img = new Image();
        img.src = ${JSON.stringify(`data:image/png;base64,${shot.data}`)};
        await img.decode();
        const copy = document.createElement('canvas');
        copy.width = img.width;
        copy.height = img.height;
        const ctx = copy.getContext('2d');
        ctx.drawImage(img, 0, 0);
        const pixels = ctx.getImageData(0, 0, copy.width, copy.height).data;
        let cream = 0;
        let green = 0;
        for (let index = 0; index < pixels.length; index += 4) {
          const r = pixels[index];
          const g = pixels[index + 1];
          const b = pixels[index + 2];
          if (r > 160 && g > 150 && b > 120) cream += 1;
          if (g > 70 && g > r + 25 && g > b + 25) green += 1;
        }
        return { cream, green, width: img.width, height: img.height };
      })()`);
    },
    async dragCanvas(x0, y0, x1, y1) {
      const point = await this.pointer(x0, y0, x1, y1);
      if (!point) throw new Error(`${name} has no map canvas`);
      await delay(200);
    },
    async pointer(x0, y0, x1, y1) {
      return this.eval(`(() => {
        const canvas = [...document.querySelectorAll('canvas')].find((node) => !node.className && node.width > 200);
        if (!canvas) return null;
        const box = canvas.getBoundingClientRect();
        const at = (xRatio, yRatio) => ({ clientX: box.x + box.width * xRatio, clientY: box.y + box.height * yRatio });
        const start = at(${x0}, ${y0});
        const end = at(${x1}, ${y1});
        const init = { button: 0, bubbles: true, cancelable: true, pointerId: 1, pointerType: 'mouse', isPrimary: true, view: window };
        canvas.setPointerCapture = canvas.setPointerCapture ?? (() => {});
        canvas.dispatchEvent(new PointerEvent('pointerdown', { ...init, ...start }));
        canvas.dispatchEvent(new PointerEvent('pointermove', { ...init, ...end }));
        canvas.dispatchEvent(new PointerEvent('pointerup', { ...init, ...end }));
        return true;
      })()`);
    },
    async waitFor(text) {
      const needle = text.toLowerCase();
      for (let attempt = 0; attempt < 100; attempt += 1) {
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
        const frames = await this.eval('window.__frames ?? []');
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
      throw new Error(`${name} snapshot did not match\n${await this.text()}`);
    },
    text() {
      return this.eval(`document.body?.innerText?.slice(0, 1500) ?? ''`);
    },
  };
}

async function paintBlock(gm, x0, z0, x1, z1) {
  for (let z = z0; z < z1; z += 1) {
    for (let x = x0; x < x1; x += 1) {
      await gm.clickWorld(x + 0.5, 0, z + 0.5);
    }
  }
}

async function selectFigure(gm, name, x, z) {
  await gm.clickWorld(x, 0, z);
  await delay(250);
  await gm.clickWorld(x, 0, z);
  await delay(150);
  const shown = await gm.eval(`document.querySelector('.display.text-lg')?.textContent?.trim() ?? ''`);
  if (shown !== name) {
    const where = await gm.eval(`(() => {
      const frames = window.__frames ?? [];
      for (let index = frames.length - 1; index >= 0; index -= 1) {
        try {
          const value = JSON.parse(frames[index]);
          if (value.type !== 'snapshot') continue;
          return (value.tokens ?? []).map((token) => token.name + ' ' + token.x + ',' + token.y).join('; ');
        } catch {
          // A frame can arrive in pieces.
        }
      }
      return 'no snapshot';
    })()`);
    throw new Error(`the click selected ${shown || 'nothing'} instead of ${name} (${where})`);
  }
}

async function placeCharacter(gm, player, x, y, z) {
  await gm.clickButton('Place');
  await gm.waitFor('Click the ground');
  await gm.clickWorld(x, y, z);
  const seated = await player.waitForSnapshot((value) => (value.tokens ?? []).some((token) => token.name === 'Ada' && token.characterId && token.castAsset === 'ruin/scout'));
  return seated;
}

async function placeFigure(gm, player, name, x, y, z) {
  await gm.fill('input[placeholder="Token name"]', name);
  await gm.clickButton('Add');
  await gm.waitFor('Click the ground');
  await gm.clickWorld(x, y, z);
  await player.waitForSnapshot((value) => (value.tokens ?? []).some((token) => token.name === name));
}

async function ruinZip() {
  const dir = join(root, 'assets', 'packs', 'ruin');
  const files = {};
  await walk(dir, dir, files);
  return zipStore(files);
}

async function walk(dir, base, files) {
  for (const entry of await readdir(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) await walk(path, base, files);
    else files[relative(base, path).replaceAll('\\', '/')] = await readFile(path);
  }
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
