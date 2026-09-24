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

(async () => {
  const browser = await chromium.launch();
  const result = {
    engine: 'chromium',
    engine_version: browser.version(),
    driver: 'playwright',
    driver_version: driverVersion,
    viewports: [],
  };
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
    const measured = await page.evaluate(([props, limit]) => {
      const skip = new Set(['script', 'style', 'noscript', 'template', 'link', 'meta']);
      const pathOf = (el) => {
        if (el === document.body) return 'body';
        let index = 1;
        for (let s = el.previousElementSibling; s; s = s.previousElementSibling) {
          if (s.tagName === el.tagName) index += 1;
        }
        return pathOf(el.parentElement) + '>' + el.tagName.toLowerCase() + ':' + index;
      };
      const elements = [];
      let truncated = false;
      for (const el of [document.body, ...document.body.querySelectorAll('*')]) {
        const tag = el.tagName.toLowerCase();
        if (skip.has(tag)) continue;
        if (elements.length >= limit) { truncated = true; break; }
        const rect = el.getBoundingClientRect();
        const computed = getComputedStyle(el);
        const style = {};
        for (const p of props) style[p] = computed.getPropertyValue(p);
        let text = 0;
        for (const node of el.childNodes) if (node.nodeType === 3) text += node.textContent.trim().length;
        elements.push({
          path: pathOf(el),
          parent: el === document.body ? null : pathOf(el.parentElement),
          tag,
          x: rect.x + window.scrollX,
          y: rect.y + window.scrollY,
          width: rect.width,
          height: rect.height,
          style,
          text_chars: text,
        });
      }
      return {
        document_width: document.documentElement.scrollWidth,
        document_height: document.documentElement.scrollHeight,
        root_font_size_px: parseFloat(getComputedStyle(document.documentElement).fontSize),
        elements,
        truncated,
      };
    }, [props, limit]);
    result.viewports.push({ width: viewport.width, height: viewport.height, ...measured });
    await context.close();
  }
  await browser.close();
  process.stdout.write(JSON.stringify(result));
})().catch((error) => {
  process.stderr.write(String(error && error.stack || error));
  process.exit(3);
});
