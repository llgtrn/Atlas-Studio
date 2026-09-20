import { useEffect, useState } from 'react';
import { AlertTriangle, Network } from 'lucide-react';
import { useBreadcrumbs } from '../../context/BreadcrumbContext';
import { DocsAtlasGraph } from '../components/DocsAtlasGraph';
import { RealityMirrorPanel } from '../components/RealityMirrorPanel';
import { SystemCoveragePanel } from '../components/SystemCoveragePanel';
import { UnifiedSystemGraphExplorer } from '../components/UnifiedSystemGraphExplorer';
import { loadDocsAtlas } from '../docsAtlas';
import type { DocsAtlasData } from '../docsAtlasModel';

export function DocsAtlasPage() {
  const { setBreadcrumbs } = useBreadcrumbs();
  const [atlas, setAtlas] = useState<DocsAtlasData | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setBreadcrumbs([{ label: 'System Atlas' }]);
  }, [setBreadcrumbs]);

  useEffect(() => {
    let cancelled = false;
    loadDocsAtlas()
      .then((value) => {
        if (!cancelled) setAtlas(value);
      })
      .catch((reason: unknown) => {
        if (!cancelled) setError(reason instanceof Error ? reason.message : String(reason));
      });
    return () => {
      cancelled = true;
    };
  }, []);

  if (error) {
    return (
      <div className="p-4">
        <div className="rounded-lg border border-destructive/40 bg-card p-5">
          <div className="flex items-center gap-2 font-semibold"><AlertTriangle className="h-4 w-4" /> System Atlas unavailable</div>
          <p className="mt-2 text-sm text-muted-foreground">{error}</p>
          <p className="mt-2 text-xs text-muted-foreground">Run <code>pnpm --filter @chronica/ui atlas:build</code> or start the UI through its normal dev/build script.</p>
        </div>
      </div>
    );
  }

  if (!atlas) {
    return <p role="status" className="p-4 text-sm text-muted-foreground">Building System Atlas view…</p>;
  }

  const coverage = atlas.reality.systemCoverage;

  return (
    <div className="flex h-full min-h-0 flex-col gap-4 p-4">
      <header className="shrink-0">
        <div className="flex items-center gap-2">
          <Network className="h-5 w-5" />
          <h1 className="text-xl font-bold">Chronica System Atlas</h1>
        </div>
        <p className="mt-2 max-w-5xl text-sm leading-6 text-muted-foreground">
          Architecture intent and implementation reality in one rebuildable projection. Markdown owns intended architecture;
          Reality Mirror observes the current repository and the full-system layer maps repository, semantic, API/protocol,
          data, execution, UI, tooling, evidence, deployment, Ops, external integration and machine/physical graphs. A unified
          cross-family graph connects matching artifacts, invariants, endpoints and semantics without becoming canonical runtime truth.
        </p>
        <div className="mt-3 flex flex-wrap gap-x-5 gap-y-1 text-xs text-muted-foreground">
          <span>{atlas.stats.documents} docs</span>
          <span>{atlas.stats.architectureOwners} architecture owners</span>
          <span>{atlas.stats.semanticEdges} semantic edges</span>
          <span>{atlas.stats.semanticOwnerCoverage}% owner semantic coverage</span>
          <span>{atlas.stats.contracts} contracts</span>
          <span>{atlas.stats.runtimeOwners} declared runtime owners</span>
          <span>{atlas.reality.stats.observedSourceFiles} observed source files</span>
          <span>{atlas.reality.stats.legacySourceFiles} configured legacy files</span>
          <span>{atlas.reality.tooling?.stats.reviewCandidates ?? 0} tooling review candidates</span>
          {coverage ? <span>{coverage.stats.graphFamilies}/{coverage.stats.requiredGraphFamilies} system graph families</span> : null}
          {coverage ? <span>{coverage.stats.trackedArtifactCoveragePercent}% tracked artifact classification</span> : null}
          {coverage ? <span>{coverage.unified.stats.crossFamilyEdges} cross-family bridges</span> : null}
        </div>
      </header>

      {atlas.diagnostics.length ? (
        <div className="shrink-0 rounded-md border border-border bg-card px-3 py-2 text-xs text-muted-foreground">
          Atlas compiler reported {atlas.diagnostics.length} diagnostic{atlas.diagnostics.length === 1 ? '' : 's'}; run <code>pnpm --filter @chronica/ui atlas:check</code> for details.
        </div>
      ) : null}

      {coverage ? <SystemCoveragePanel coverage={coverage} /> : null}
      {coverage ? <UnifiedSystemGraphExplorer coverage={coverage} /> : null}
      <RealityMirrorPanel reality={atlas.reality} />
      <DocsAtlasGraph atlas={atlas} />
    </div>
  );
}
