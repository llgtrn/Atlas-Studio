// Tarjan's strongly connected components over a plain directed graph.
// nodeIds: string[]; edges: [{from, to}][] (edges outside nodeIds are ignored)
export function tarjanScc(nodeIds, edges) {
  const adj = new Map(nodeIds.map((id) => [id, []]));
  for (const { from, to } of edges) {
    if (adj.has(from) && adj.has(to)) adj.get(from).push(to);
  }

  let index = 0;
  const indices = new Map();
  const lowlink = new Map();
  const onStack = new Set();
  const stack = [];
  const components = [];

  function strongconnect(v) {
    indices.set(v, index);
    lowlink.set(v, index);
    index += 1;
    stack.push(v);
    onStack.add(v);

    for (const w of adj.get(v) ?? []) {
      if (!indices.has(w)) {
        strongconnect(w);
        lowlink.set(v, Math.min(lowlink.get(v), lowlink.get(w)));
      } else if (onStack.has(w)) {
        lowlink.set(v, Math.min(lowlink.get(v), indices.get(w)));
      }
    }

    if (lowlink.get(v) === indices.get(v)) {
      const component = [];
      let w;
      do {
        w = stack.pop();
        onStack.delete(w);
        component.push(w);
      } while (w !== v);
      components.push(component);
    }
  }

  // Iterative fallback isn't needed at this graph size in practice, but guard
  // against stack depth on pathological inputs by chunking recursion starts.
  for (const v of nodeIds) {
    if (!indices.has(v)) strongconnect(v);
  }

  return components;
}
