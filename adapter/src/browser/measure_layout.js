// Atlas layout measurement (ADR 0011/0022): engine-neutral web-platform code that runs inside the
// page. Every browser instrument backend (Playwright, W3C WebDriver) evaluates exactly this
// function, so all engines are measured by the same procedure.
function atlasMeasureLayout(args) {
  const [props, limit] = args;
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
}
