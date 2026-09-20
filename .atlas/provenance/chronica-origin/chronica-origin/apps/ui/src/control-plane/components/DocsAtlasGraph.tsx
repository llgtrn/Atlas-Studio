import { useMemo, useState } from 'react';
import { FileText, Focus, Network, Search, ZoomIn, ZoomOut } from 'lucide-react';
import type { DocsAtlasData, DocsAtlasEdge, DocsAtlasNode } from '../docsAtlasModel';

const CARD_WIDTH = 220;
const CARD_HEIGHT = 86;
const COLUMN_GAP = 70;
const ROW_GAP = 34;
const TOP_PAD = 64;
const LEFT_PAD = 32;

const GROUP_ORDER = [
  'architecture/constitution',
  'architecture/foundation',
  'architecture/governance',
  'architecture/intelligence',
  'architecture/organism',
  'architecture/physical',
  'architecture/operations',
  'blueprint',
  'guide',
  'reference',
  'decision',
  'frontend',
  'ops',
  'learning',
  'router',
  'contract',
  'runtime',
  'other',
];

const DEFAULT_FAMILIES = new Set(['architecture', 'blueprint']);
const SEMANTIC_INFERENCE = new Set(['frontmatter', 'atlas-relation-map']);
const EVIDENCE_EDGE_TYPES = new Set(['DECLARES', 'RUNTIME_OWNER', 'GUARDED_BY']);

type AtlasView = 'semantic' | 'evidence' | 'all';

export interface PositionedAtlasNode extends DocsAtlasNode {
  x: number;
  y: number;
  column: string;
}

function groupRank(group: string) {
  const exact = GROUP_ORDER.indexOf(group);
  if (exact >= 0) return exact;
  const family = group.split('/')[0] ?? group;
  const familyRank = GROUP_ORDER.indexOf(family);
  return familyRank >= 0 ? familyRank : GROUP_ORDER.length + 1;
}

export function layoutAtlasNodes(nodes: DocsAtlasNode[]): { nodes: PositionedAtlasNode[]; width: number; height: number } {
  const groups = new Map<string, DocsAtlasNode[]>();
  for (const node of nodes) {
    const key = node.group || node.family || 'other';
    groups.set(key, [...(groups.get(key) ?? []), node]);
  }
  const orderedGroups = [...groups.entries()].sort((a, b) => {
    const rank = groupRank(a[0]) - groupRank(b[0]);
    return rank !== 0 ? rank : a[0].localeCompare(b[0]);
  });

  const positioned: PositionedAtlasNode[] = [];
  let maxRows = 1;
  orderedGroups.forEach(([column, columnNodes], columnIndex) => {
    const sorted = [...columnNodes].sort((a, b) => a.title.localeCompare(b.title));
    maxRows = Math.max(maxRows, sorted.length);
    sorted.forEach((node, rowIndex) => {
      positioned.push({
        ...node,
        column,
        x: LEFT_PAD + columnIndex * (CARD_WIDTH + COLUMN_GAP),
        y: TOP_PAD + rowIndex * (CARD_HEIGHT + ROW_GAP),
      });
    });
  });

  return {
    nodes: positioned,
    width: Math.max(760, LEFT_PAD * 2 + orderedGroups.length * (CARD_WIDTH + COLUMN_GAP)),
    height: Math.max(520, TOP_PAD + maxRows * (CARD_HEIGHT + ROW_GAP) + 80),
  };
}

function matches(node: DocsAtlasNode, query: string) {
  if (!query.trim()) return true;
  const needle = query.trim().toLowerCase();
  return [node.title, node.path, node.summary, node.kind, node.group, ...node.contractIds, ...node.runtimeOwners]
    .filter(Boolean)
    .some((value) => String(value).toLowerCase().includes(needle));
}

function familyLabel(family: string) {
  if (family === 'architecture') return 'Architecture';
  if (family === 'blueprint') return 'Blueprints';
  if (family === 'frontend') return 'Frontend';
  if (family === 'ops') return 'Ops';
  if (family === 'contract') return 'Contracts';
  if (family === 'runtime') return 'Runtime';
  return family.charAt(0).toUpperCase() + family.slice(1);
}

function edgePath(from: PositionedAtlasNode, to: PositionedAtlasNode) {
  const x1 = from.x + CARD_WIDTH;
  const y1 = from.y + CARD_HEIGHT / 2;
  const x2 = to.x;
  const y2 = to.y + CARD_HEIGHT / 2;
  const bend = Math.max(40, Math.abs(x2 - x1) * 0.45);
  return `M ${x1} ${y1} C ${x1 + bend} ${y1}, ${x2 - bend} ${y2}, ${x2} ${y2}`;
}

function edgeIsVisibleInView(edge: DocsAtlasEdge, view: AtlasView) {
  if (view === 'semantic') return SEMANTIC_INFERENCE.has(edge.inferredBy);
  if (view === 'evidence') return EVIDENCE_EDGE_TYPES.has(edge.type);
  return true;
}

function NodeCard({ node, selected, related, dimmed, onSelect }: { node: PositionedAtlasNode; selected: boolean; related: boolean; dimmed: boolean; onSelect: () => void }) {
  return (
    <button
      type="button"
      onClick={onSelect}
      className={`absolute rounded-lg border bg-card p-3 text-left shadow-sm transition hover:border-foreground/30 hover:shadow-md ${selected ? 'border-foreground ring-2 ring-ring' : related ? 'border-foreground/50' : 'border-border'} ${dimmed ? 'opacity-35' : 'opacity-100'}`}
      style={{ left: node.x, top: node.y, width: CARD_WIDTH, minHeight: CARD_HEIGHT }}
      aria-pressed={selected}
    >
      <div className="flex items-start justify-between gap-2">
        <div className="min-w-0">
          <div className="truncate text-sm font-semibold">{node.title}</div>
          <div className="mt-1 truncate font-mono text-[10px] text-muted-foreground">{node.kind}</div>
        </div>
        <FileText className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" />
      </div>
      <div className="mt-2 line-clamp-2 text-[11px] leading-4 text-muted-foreground">
        {node.summary || node.path}
      </div>
    </button>
  );
}

function Inspector({ node, edges, nodesById }: { node: DocsAtlasNode; edges: DocsAtlasEdge[]; nodesById: Map<string, DocsAtlasNode> }) {
  const incoming = edges.filter((edge) => edge.target === node.id);
  const outgoing = edges.filter((edge) => edge.source === node.id);
  const relationRows = [
    ...outgoing.map((edge) => ({ edge, direction: '→', peer: nodesById.get(edge.target) })),
    ...incoming.map((edge) => ({ edge, direction: '←', peer: nodesById.get(edge.source) })),
  ];

  return (
    <aside className="w-full border-t border-border bg-card p-4 lg:w-[380px] lg:border-l lg:border-t-0">
      <div className="text-xs font-medium uppercase tracking-wider text-muted-foreground">Selected node</div>
      <h2 className="mt-2 text-lg font-semibold">{node.title}</h2>
      <div className="mt-1 font-mono text-xs text-muted-foreground">{node.id}</div>
      <p className="mt-3 text-sm leading-6 text-muted-foreground">{node.summary || 'No prose summary extracted.'}</p>

      <dl className="mt-5 space-y-3 text-xs">
        <div><dt className="font-medium">Source</dt><dd className="mt-1 break-all font-mono text-muted-foreground">{node.path}</dd></div>
        <div><dt className="font-medium">Family / group</dt><dd className="mt-1 text-muted-foreground">{node.family} · {node.group}</dd></div>
        {node.status ? <div><dt className="font-medium">Status</dt><dd className="mt-1 text-muted-foreground">{node.status}</dd></div> : null}
        {node.contractIds.length ? <div><dt className="font-medium">Contracts</dt><dd className="mt-1 flex flex-wrap gap-1">{node.contractIds.map((id) => <code key={id} className="rounded bg-muted px-1.5 py-0.5">{id}</code>)}</dd></div> : null}
        {node.runtimeOwners.length ? <div><dt className="font-medium">Runtime owners</dt><dd className="mt-1 space-y-1">{node.runtimeOwners.map((owner) => <div key={owner} className="font-mono text-muted-foreground">{owner}</div>)}</dd></div> : null}
      </dl>

      <div className="mt-6 text-xs font-medium uppercase tracking-wider text-muted-foreground">Typed relations</div>
      <div className="mt-2 max-h-72 space-y-2 overflow-auto">
        {relationRows.length ? relationRows.map(({ edge, direction, peer }) => (
          <div key={`${direction}-${edge.id}`} className="rounded border border-border p-2 text-xs">
            <div className="flex items-center justify-between gap-2">
              <span className="font-mono text-[10px] text-muted-foreground">{edge.type}</span>
              <span className="text-[10px] text-muted-foreground">{edge.inferredBy}</span>
            </div>
            <div className="mt-1">{direction} {peer?.title ?? (direction === '→' ? edge.target : edge.source)}</div>
            {edge.label ? <div className="mt-1 text-muted-foreground">{edge.label}</div> : null}
          </div>
        )) : <div className="text-xs text-muted-foreground">No graph relations yet.</div>}
      </div>
    </aside>
  );
}

function MiniMap({ layout, edges, positionedById }: { layout: { nodes: PositionedAtlasNode[]; width: number; height: number }; edges: DocsAtlasEdge[]; positionedById: Map<string, PositionedAtlasNode> }) {
  const width = 220;
  const height = 120;
  const scaleX = width / Math.max(layout.width, 1);
  const scaleY = height / Math.max(layout.height, 1);
  return (
    <div className="pointer-events-none absolute bottom-3 right-3 hidden rounded-md border border-border bg-background/90 p-1 shadow-sm xl:block" aria-hidden="true">
      <svg width={width} height={height} viewBox={`0 0 ${width} ${height}`}>
        {edges.map((edge) => {
          const from = positionedById.get(edge.source);
          const to = positionedById.get(edge.target);
          if (!from || !to) return null;
          return (
            <line
              key={edge.id}
              x1={(from.x + CARD_WIDTH / 2) * scaleX}
              y1={(from.y + CARD_HEIGHT / 2) * scaleY}
              x2={(to.x + CARD_WIDTH / 2) * scaleX}
              y2={(to.y + CARD_HEIGHT / 2) * scaleY}
              stroke="currentColor"
              className="text-muted-foreground/25"
              strokeWidth="0.7"
            />
          );
        })}
        {layout.nodes.map((node) => (
          <rect
            key={node.id}
            x={node.x * scaleX}
            y={node.y * scaleY}
            width={Math.max(3, CARD_WIDTH * scaleX)}
            height={Math.max(2, CARD_HEIGHT * scaleY)}
            rx="1"
            fill="currentColor"
            className="text-foreground/55"
          />
        ))}
      </svg>
    </div>
  );
}

export function DocsAtlasGraph({ atlas }: { atlas: DocsAtlasData }) {
  const families = useMemo(() => [...new Set(atlas.nodes.map((node) => node.family))].sort(), [atlas.nodes]);
  const [enabledFamilies, setEnabledFamilies] = useState<Set<string>>(() => new Set(DEFAULT_FAMILIES));
  const [query, setQuery] = useState('');
  const [zoom, setZoom] = useState(0.8);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [view, setView] = useState<AtlasView>('semantic');

  const nodesById = useMemo(() => new Map(atlas.nodes.map((node) => [node.id, node])), [atlas.nodes]);
  const visibleNodes = useMemo(() => {
    if (query.trim()) return atlas.nodes.filter((node) => matches(node, query));
    if (view === 'evidence') return atlas.nodes.filter((node) => ['architecture', 'contract', 'runtime'].includes(node.family));
    return atlas.nodes.filter((node) => enabledFamilies.has(node.family));
  }, [atlas.nodes, enabledFamilies, query, view]);
  const visibleIds = useMemo(() => new Set(visibleNodes.map((node) => node.id)), [visibleNodes]);
  const visibleEdges = useMemo(
    () => atlas.edges.filter((edge) => visibleIds.has(edge.source) && visibleIds.has(edge.target) && edgeIsVisibleInView(edge, view)),
    [atlas.edges, visibleIds, view],
  );
  const layout = useMemo(() => layoutAtlasNodes(visibleNodes), [visibleNodes]);
  const positionedById = useMemo(() => new Map(layout.nodes.map((node) => [node.id, node])), [layout.nodes]);
  const selected = selectedId ? nodesById.get(selectedId) ?? null : null;
  const relatedIds = useMemo(() => {
    if (!selectedId) return new Set<string>();
    const related = new Set<string>([selectedId]);
    for (const edge of visibleEdges) {
      if (edge.source === selectedId) related.add(edge.target);
      if (edge.target === selectedId) related.add(edge.source);
    }
    return related;
  }, [selectedId, visibleEdges]);

  const toggleFamily = (family: string) => {
    setQuery('');
    setView('all');
    setEnabledFamilies((current) => {
      const next = new Set(current);
      if (next.has(family)) next.delete(family); else next.add(family);
      return next;
    });
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-hidden rounded-lg border border-border bg-background">
      <div className="flex flex-wrap items-center gap-2 border-b border-border p-3">
        <div className="relative min-w-[240px] flex-1 sm:max-w-md">
          <Search className="pointer-events-none absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
          <input
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Search docs, contracts, runtime owners…"
            className="h-9 w-full rounded-md border border-input bg-background pl-9 pr-3 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
          />
        </div>
        <div className="flex rounded-md border border-border p-0.5" aria-label="Atlas view">
          {(['semantic', 'evidence', 'all'] as AtlasView[]).map((mode) => (
            <button
              key={mode}
              type="button"
              onClick={() => { setView(mode); setQuery(''); }}
              className={`rounded px-2.5 py-1.5 text-xs ${view === mode ? 'bg-foreground text-background' : 'text-muted-foreground'}`}
            >
              {mode === 'semantic' ? 'Semantic' : mode === 'evidence' ? 'Contracts + runtime' : 'All edges'}
            </button>
          ))}
        </div>
        <button type="button" onClick={() => setZoom((value) => Math.max(0.45, value - 0.1))} className="rounded-md border border-border p-2" aria-label="Zoom out"><ZoomOut className="h-4 w-4" /></button>
        <button type="button" onClick={() => setZoom((value) => Math.min(1.35, value + 0.1))} className="rounded-md border border-border p-2" aria-label="Zoom in"><ZoomIn className="h-4 w-4" /></button>
        <button type="button" onClick={() => setZoom(0.8)} className="rounded-md border border-border p-2" aria-label="Reset zoom"><Focus className="h-4 w-4" /></button>
        <span className="text-xs text-muted-foreground">{Math.round(zoom * 100)}%</span>
      </div>

      <div className="flex flex-wrap gap-1.5 border-b border-border px-3 py-2">
        {families.map((family) => {
          const active = enabledFamilies.has(family) && !query.trim() && view !== 'evidence';
          return (
            <button
              key={family}
              type="button"
              onClick={() => toggleFamily(family)}
              className={`rounded-full border px-2.5 py-1 text-xs ${active ? 'border-foreground bg-foreground text-background' : 'border-border text-muted-foreground'}`}
            >
              {familyLabel(family)}
            </button>
          );
        })}
        <span className="ml-auto self-center text-xs text-muted-foreground">{visibleNodes.length} nodes · {visibleEdges.length} edges</span>
      </div>

      <div className="flex min-h-0 flex-1 flex-col lg:flex-row">
        <div className="relative min-h-[520px] min-w-0 flex-1 overflow-auto bg-[radial-gradient(circle_at_1px_1px,hsl(var(--border))_1px,transparent_0)] bg-[length:20px_20px]">
          <div style={{ width: layout.width * zoom, height: layout.height * zoom, position: 'relative' }}>
            <div style={{ width: layout.width, height: layout.height, transform: `scale(${zoom})`, transformOrigin: 'top left', position: 'absolute', inset: 0 }}>
              <svg className="absolute inset-0" width={layout.width} height={layout.height} aria-hidden="true">
                <defs>
                  <marker id="docs-atlas-arrow" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto" markerUnits="strokeWidth">
                    <path d="M0,0 L8,4 L0,8 Z" className="fill-muted-foreground" />
                  </marker>
                </defs>
                {visibleEdges.map((edge) => {
                  const from = positionedById.get(edge.source);
                  const to = positionedById.get(edge.target);
                  if (!from || !to) return null;
                  const active = !selectedId || edge.source === selectedId || edge.target === selectedId;
                  return <path key={edge.id} d={edgePath(from, to)} fill="none" stroke="currentColor" className={active ? 'text-muted-foreground/70' : 'text-muted-foreground/15'} strokeWidth={active && selectedId ? 2 : 1.2} markerEnd="url(#docs-atlas-arrow)" />;
                })}
              </svg>
              {layout.nodes.map((node) => (
                <NodeCard
                  key={node.id}
                  node={node}
                  selected={selectedId === node.id}
                  related={relatedIds.has(node.id) && selectedId !== node.id}
                  dimmed={Boolean(selectedId) && !relatedIds.has(node.id)}
                  onSelect={() => setSelectedId(node.id)}
                />
              ))}
            </div>
          </div>
          <MiniMap layout={layout} edges={visibleEdges} positionedById={positionedById} />
        </div>
        {selected ? <Inspector node={selected} edges={atlas.edges} nodesById={nodesById} /> : (
          <aside className="w-full border-t border-border bg-card p-4 text-sm text-muted-foreground lg:w-[380px] lg:border-l lg:border-t-0">
            <div className="flex items-center gap-2 font-medium text-foreground"><Network className="h-4 w-4" /> Semantic Atlas</div>
            <p className="mt-2 leading-6">Select a node to inspect its human summary, source document, contracts, runtime ownership and typed relations. Semantic view hides incidental Markdown links so the architecture DAG stays readable.</p>
          </aside>
        )}
      </div>
    </div>
  );
}
