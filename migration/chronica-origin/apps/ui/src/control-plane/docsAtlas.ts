import type { DocsAtlasData } from './docsAtlasModel';

let cached: Promise<DocsAtlasData> | null = null;

export function loadDocsAtlas(): Promise<DocsAtlasData> {
  if (cached) return cached;
  const url = `${import.meta.env.BASE_URL}docs-atlas.json`;
  cached = fetch(url, { cache: 'no-store' }).then(async (response) => {
    if (!response.ok) throw new Error(`System Atlas unavailable (${response.status})`);
    const value = (await response.json()) as DocsAtlasData;
    const coverage = value.reality?.systemCoverage;
    if (
      value.schemaVersion !== 2
      || !Array.isArray(value.nodes)
      || !Array.isArray(value.edges)
      || !value.reality
      || !value.reality.tooling
      || !coverage
      || coverage.stats.graphFamilies !== 12
      || coverage.stats.requiredGraphFamilies !== 12
      || coverage.stats.domainCoveragePercent !== 100
      || coverage.stats.trackedArtifactCoveragePercent !== 100
      || !coverage.unified
      || !Array.isArray(coverage.unified.nodes)
      || !Array.isArray(coverage.unified.edges)
      || !Array.isArray(coverage.unified.crossFamilyEdges)
    ) {
      throw new Error('System Atlas payload has an unsupported schema or is missing unified 12-family full-system coverage evidence');
    }
    return value;
  });
  return cached;
}

export function resetDocsAtlasCacheForTests() {
  cached = null;
}
