// Atlas visual observation instrument (ADR 0011). Driven by adapter::browser; prints one JSON
// document to stdout. Hermetic: every request other than the subject file itself is aborted.
const modulePath = process.env.ATLAS_PLAYWRIGHT_MODULE;
const { chromium } = require(modulePath);
const driverVersion = require(modulePath + '/package.json').version;
const url = process.env.ATLAS_OBSERVE_URL;
const viewports = process.env.ATLAS_OBSERVE_VIEWPORTS.split(',').map((v) => {
  const [width, height] = v.split('x').map(Number);
  return { width, height };
});
const props = JSON.parse(process.env.ATLAS_OBSERVE_PROPERTIES);
const limit = Number(process.env.ATLAS_OBSERVE_ELEMENT_LIMIT);

// Interaction properties: what a stimulus is observed to change (style, geometry, state).
const INTERACTION_PROPS = ['transform', 'opacity', 'box-shadow', 'background-color', 'color',
  'visibility', 'display', 'outline-style'];

async function freshPage(browser, viewport) {
  const context = await browser.newContext({ viewport, deviceScaleFactor: 1 });
  await context.route('**/*', (route) =>
    route.request().url() === url ? route.continue() : route.abort());
  const page = await context.newPage();
  await page.goto(url, { waitUntil: 'load', timeout: 15000 });
  return { context, page };
}

// Snapshot of every element's interaction-relevant state, keyed by structural path.
function snapshot(page) {
  return page.evaluate((props) => {
    const pathOf = (el) => {
      if (el === document.body) return 'body';
      let index = 1;
      for (let s = el.previousElementSibling; s; s = s.previousElementSibling) {
        if (s.tagName === el.tagName) index += 1;
      }
      return pathOf(el.parentElement) + '>' + el.tagName.toLowerCase() + ':' + index;
    };
    const state = {};
    for (const el of [document.body, ...document.body.querySelectorAll('*')]) {
      if (['script', 'style', 'noscript', 'template'].includes(el.tagName.toLowerCase())) continue;
      const cs = getComputedStyle(el);
      const r = el.getBoundingClientRect();
      const entry = { width: r.width.toFixed(4), height: r.height.toFixed(4) };
      for (const p of props) entry[p] = cs.getPropertyValue(p);
      for (const a of ['aria-expanded', 'aria-pressed', 'aria-selected', 'aria-hidden', 'open']) {
        if (el.hasAttribute(a)) entry['attr:' + a] = el.getAttribute(a);
      }
      entry['focused'] = String(document.activeElement === el);
      state[pathOf(el)] = entry;
    }
    return state;
  }, INTERACTION_PROPS);
}

async function settle(page) {
  return page.evaluate(async () => {
    const animations = document.getAnimations();
    await Promise.all(animations.map((a) => a.finished.catch(() => null)));
    return animations.length;
  });
}

function diff(before, after) {
  const changes = [];
  for (const element of Object.keys(after)) {
    const a = before[element] || {};
    const b = after[element];
    for (const property of Object.keys(b)) {
      if (a[property] !== b[property]) {
        changes.push({ element, property, before: a[property] ?? null, after: b[property] });
      }
    }
  }
  return changes;
}

async function interact(browser, viewport) {
  const probe = await freshPage(browser, viewport);
  const targets = await probe.page.evaluate(() => {
    const pathOf = (el) => {
      if (el === document.body) return 'body';
      let index = 1;
      for (let s = el.previousElementSibling; s; s = s.previousElementSibling) {
        if (s.tagName === el.tagName) index += 1;
      }
      return pathOf(el.parentElement) + '>' + el.tagName.toLowerCase() + ':' + index;
    };
    const interactive = document.querySelectorAll(
      'a[href], button, summary, input, select, textarea, [role=button], [role=tab], [tabindex]');
    const hoverables = [...document.body.querySelectorAll('*')].filter((el) =>
      getComputedStyle(el).cursor === 'pointer');
    return [...new Set([...interactive, ...hoverables])].map(pathOf).sort();
  });
  await probe.context.close();
  const toSelector = (path) => path.split('>').map((part) => {
    const [tag, index] = part.split(':');
    return index ? `${tag}:nth-of-type(${index})` : tag;
  }).join(' > ');
  const observations = [];
  for (const target of targets) {
    for (const stimulus of ['HOVER', 'CLICK', 'CLICK_TWICE', 'FOCUS']) {
      const { context, page } = await freshPage(browser, viewport);
      await page.mouse.move(0, 0);
      const baseline = await snapshot(page);
      const locator = page.locator(toSelector(target)).first();
      if (stimulus === 'HOVER') await locator.hover();
      if (stimulus === 'CLICK') await locator.click();
      if (stimulus === 'CLICK_TWICE') { await locator.click(); await settle(page); await locator.click(); }
      if (stimulus === 'FOCUS') await locator.focus();
      if (stimulus !== 'HOVER') await page.mouse.move(0, 0);
      const animations = await settle(page);
      const after = await snapshot(page);
      observations.push({ target, stimulus, animations, changes: diff(baseline, after) });
      await context.close();
    }
  }
  return observations;
}

// Motion (ADR 0016): hover each candidate, then pause every running animation and seek it to
// SAMPLES+1 evenly spaced times across its active interval, reading the animated property at each
// -- deterministic sampling, no wall-clock jitter. The Web Animations timing is recorded as the
// declared (OBSERVED) ground truth the curve inference is validated against.
const SAMPLES = 20;
async function motion(browser, viewport) {
  const probe = await freshPage(browser, viewport);
  const targets = await probe.page.evaluate(() => {
    const pathOf = (el) => {
      if (el === document.body) return 'body';
      let index = 1;
      for (let s = el.previousElementSibling; s; s = s.previousElementSibling) {
        if (s.tagName === el.tagName) index += 1;
      }
      return pathOf(el.parentElement) + '>' + el.tagName.toLowerCase() + ':' + index;
    };
    return [...document.body.querySelectorAll('*')]
      .filter((el) => getComputedStyle(el).cursor === 'pointer'
        || el.matches('a[href], button, summary, [role=button]'))
      .map(pathOf).sort();
  });
  await probe.context.close();
  const toSelector = (path) => path.split('>').map((part) => {
    const [tag, index] = part.split(':');
    return index ? `${tag}:nth-of-type(${index})` : tag;
  }).join(' > ');
  const motions = [];
  for (const target of targets) {
    const { context, page } = await freshPage(browser, viewport);
    await page.mouse.move(0, 0);
    await page.locator(toSelector(target)).first().hover();
    const sampled = await page.evaluate((samples) => {
      const pathOf = (el) => {
        if (el === document.body) return 'body';
        let index = 1;
        for (let s = el.previousElementSibling; s; s = s.previousElementSibling) {
          if (s.tagName === el.tagName) index += 1;
        }
        return pathOf(el.parentElement) + '>' + el.tagName.toLowerCase() + ':' + index;
      };
      return document.getAnimations().map((animation) => {
        animation.pause();
        const timing = animation.effect.getTiming();
        const element = animation.effect.target;
        const property = animation.transitionProperty || null;
        const points = [];
        for (let k = 0; k <= samples; k += 1) {
          const time = timing.delay + (timing.duration * k) / samples;
          animation.currentTime = time;
          points.push([time, property ? getComputedStyle(element).getPropertyValue(property) : '']);
        }
        return {
          element: pathOf(element),
          property,
          kind: animation.constructor.name,
          declared: { duration_ms: timing.duration, delay_ms: timing.delay, easing: timing.easing },
          samples: points,
        };
      });
    }, SAMPLES);
    for (const m of sampled) motions.push({ target, stimulus: 'HOVER', ...m });
    await context.close();
  }
  return motions;
}

(async () => {
  const browser = await chromium.launch();
  const result = {
    engine: 'chromium',
    engine_version: browser.version(),
    driver: 'playwright',
    driver_version: driverVersion,
    // Enforced by context.route below: every request other than the subject is aborted.
    network: 'BLOCKED_EXCEPT_SUBJECT',
    // Blink LayoutUnit: 1/64 px.
    layout_resolution_px: 1 / 64,
    viewports: [],
  };
  if (process.env.ATLAS_OBSERVE_MODE === 'motion') {
    result.motions = await motion(browser, viewports[0]);
    await browser.close();
    process.stdout.write(JSON.stringify(result));
    return;
  }
  if (process.env.ATLAS_OBSERVE_MODE === 'interact') {
    result.interactions = await interact(browser, viewports[0]);
    await browser.close();
    process.stdout.write(JSON.stringify(result));
    return;
  }
  for (const viewport of viewports) {
    const context = await browser.newContext({ viewport, deviceScaleFactor: 1 });
    await context.route('**/*', (route) =>
      route.request().url() === url ? route.continue() : route.abort());
    const page = await context.newPage();
    await page.goto(url, { waitUntil: 'load', timeout: 15000 });
    const measured = await page.evaluate(atlasMeasureLayout, [props, limit]);
    result.viewports.push({ width: viewport.width, height: viewport.height, ...measured });
    await context.close();
  }
  await browser.close();
  process.stdout.write(JSON.stringify(result));
})().catch((error) => {
  process.stderr.write(String(error && error.stack || error));
  process.exit(3);
});
