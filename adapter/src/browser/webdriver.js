// Atlas W3C WebDriver instrument harness (ADR 0022). Driven by adapter::browser; prints one JSON
// document to stdout in the same contract as observe.js. Uses only standard W3C WebDriver
// endpoints (new session, set window rect, navigate, execute/sync, delete session), which Ladybird's
// Services/WebDriver and chromedriver both implement. `atlasMeasureLayout` is prepended by the
// adapter (measure_layout.js) -- the same in-page procedure every backend runs.
const { spawn } = require('child_process');
const net = require('net');

const kind = process.env.ATLAS_WEBDRIVER_KIND; // 'chromedriver' | 'ladybird'
const driverBinary = process.env.ATLAS_WEBDRIVER_BINARY;
const browserBinary = process.env.ATLAS_BROWSER_BINARY || '';
const url = process.env.ATLAS_OBSERVE_URL;
const viewports = process.env.ATLAS_OBSERVE_VIEWPORTS.split(',').map((v) => {
  const [width, height] = v.split('x').map(Number);
  return { width, height };
});
const props = JSON.parse(process.env.ATLAS_OBSERVE_PROPERTIES);
const limit = Number(process.env.ATLAS_OBSERVE_ELEMENT_LIMIT);
const resolution = Number(process.env.ATLAS_LAYOUT_RESOLUTION_PX);

function freePort() {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.listen(0, '127.0.0.1', () => {
      const { port } = server.address();
      server.close(() => resolve(port));
    });
    server.on('error', reject);
  });
}

async function call(base, method, path, body) {
  const response = await fetch(base + path, {
    method,
    headers: { 'Content-Type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const json = await response.json();
  if (!response.ok || (json.value && json.value.error)) {
    throw new Error(`${method} ${path}: ${JSON.stringify(json.value || json)}`);
  }
  return json.value;
}

function capabilities() {
  if (kind === 'ladybird') {
    // No deny-by-default network control exists in Ladybird's WebDriver (only --resource-map
    // substitution); the report records the network policy as NOT_ENFORCED.
    return { alwaysMatch: { 'ladybird:headless': true } };
  }
  // chromedriver: unresolvable hosts block every network request; local files still load, so
  // this is weaker than Playwright's BLOCKED_EXCEPT_SUBJECT and is recorded as such.
  return {
    alwaysMatch: {
      browserName: 'chrome',
      'goog:chromeOptions': {
        binary: browserBinary,
        args: ['--headless=new', '--no-sandbox', '--force-device-scale-factor=1',
          '--host-resolver-rules=MAP * ~NOTFOUND', '--disable-gpu',
          // As Playwright's headless Chromium: overlay-free layout, no scrollbar gutter.
          '--hide-scrollbars'],
      },
    },
  };
}

const networkPolicy = kind === 'ladybird' ? 'NOT_ENFORCED' : 'HOSTS_BLOCKED_LOCAL_FILES_ALLOWED';

// W3C set-window-rect sizes the outer window; iterate until the viewport (window.innerWidth /
// innerHeight, the width media queries evaluate against) is exact.
async function fitViewport(base, session, viewport) {
  const read = () => call(base, 'POST', `/session/${session}/execute/sync`, {
    script: 'return [window.innerWidth, window.innerHeight];', args: [],
  });
  let outer = { width: viewport.width, height: viewport.height };
  for (let attempt = 0; attempt < 6; attempt += 1) {
    await call(base, 'POST', `/session/${session}/window/rect`, outer);
    // A resize applies asynchronously: read until two consecutive readings agree, so a
    // correction is never computed from a pre-resize viewport.
    let inner = await read();
    for (let settle = 0; settle < 20; settle += 1) {
      await new Promise((r) => setTimeout(r, 25));
      const again = await read();
      if (again[0] === inner[0] && again[1] === inner[1]) break;
      inner = again;
    }
    if (inner[0] === viewport.width && inner[1] === viewport.height) return true;
    outer = { width: outer.width + viewport.width - inner[0], height: outer.height + viewport.height - inner[1] };
    if (outer.width < viewport.width || outer.height < viewport.height
      || outer.width > viewport.width + 1000 || outer.height > viewport.height + 1000) {
      throw new Error(`implausible window correction ${JSON.stringify({ viewport, inner, outer })}`);
    }
  }
  return false;
}

(async () => {
  const port = await freePort();
  const args = kind === 'ladybird'
    ? ['--headless', '--listen-address', '127.0.0.1', '--port', String(port)]
    : [`--port=${port}`, '--allowed-ips=127.0.0.1'];
  const driver = spawn(driverBinary, args, { stdio: ['ignore', 'ignore', 'pipe'] });
  let stderr = '';
  driver.stderr.on('data', (d) => { stderr += d; });
  const base = `http://127.0.0.1:${port}`;
  try {
    let ready = false;
    for (let i = 0; i < 100 && !ready; i += 1) {
      try {
        const status = await call(base, 'GET', '/status');
        ready = status.ready !== false;
      } catch (_) {
        await new Promise((r) => setTimeout(r, 100));
      }
    }
    if (!ready) throw new Error('WebDriver server did not become ready: ' + stderr);
    const result = {
      engine: '', engine_version: '', driver: `webdriver:${kind}`, driver_version: '',
      network: networkPolicy, layout_resolution_px: resolution, viewports: [],
    };
    for (const viewport of viewports) {
      // A fresh session (fresh browser, fresh state) per viewport.
      const created = await call(base, 'POST', '/session', { capabilities: capabilities() });
      const session = created.sessionId;
      try {
        const caps = created.capabilities || {};
        result.engine = String(caps.browserName || kind);
        result.engine_version = String(caps.browserVersion || '');
        result.driver_version = String((caps.chrome && caps.chrome.chromedriverVersion || '').split(' ')[0]
          || caps['ladybird:version'] || '');
        await call(base, 'POST', `/session/${session}/url`, { url });
        if (!(await fitViewport(base, session, viewport))) {
          throw new Error(`could not fit the layout viewport to ${viewport.width}x${viewport.height}`);
        }
        const measured = await call(base, 'POST', `/session/${session}/execute/sync`, {
          script: `return (${atlasMeasureLayout.toString()})(arguments[0]);`,
          args: [[props, limit]],
        });
        result.viewports.push({ width: viewport.width, height: viewport.height, ...measured });
      } finally {
        await call(base, 'DELETE', `/session/${session}`).catch(() => null);
      }
    }
    process.stdout.write(JSON.stringify(result));
  } finally {
    driver.kill();
  }
})().catch((error) => {
  process.stderr.write(String(error && error.stack || error));
  process.exit(3);
});
