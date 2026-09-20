export interface DocsAtlasNode {
  id: string;
  title: string;
  kind: string;
  family: string;
  group: string;
  status: string | null;
  summary: string;
  path: string;
  headings: Array<{ depth: number; title: string }>;
  contractIds: string[];
  runtimeOwners: string[];
  sourceDocId?: string;
  contractKind?: string;
  severity?: string;
  gate?: string | null;
}

export interface DocsAtlasEdge {
  id: string;
  source: string;
  target: string;
  type: string;
  label?: string;
  inferredBy: string;
}

export interface DocsAtlasDiagnostic {
  level: string;
  source: string;
  message: string;
}

export type RealityImplementationState = 'TARGET_ONLY' | 'STRUCTURAL' | 'CODE_EVIDENCED' | 'TESTED' | 'INTEGRATED';

export interface RealityOwnerEvidence {
  docId: string;
  title: string;
  path: string;
  implementationState: RealityImplementationState;
  declaredRuntimeOwners: string[];
  missingRuntimeOwners: string[];
  runtimeSourceFiles: number;
  runtimeFileSamples: string[];
  directCodeEvidenceFiles: string[];
  directTestEvidenceFiles: string[];
  externalReferenceFiles: number | null;
  contractIds: string[];
  uncitedContracts: string[];
  evidenceStrength: 'TEST_LINKED' | 'CODE_LINKED' | 'STRUCTURAL_ONLY' | 'NO_IMPLEMENTATION_EVIDENCE';
}

export interface RealityRuntimeOwnerEvidence {
  path: string;
  normalizedPath: string;
  exists: boolean;
  trackedFiles: number;
  sourceFiles: number;
  testFiles: number;
  packageIdentity: { kind: string; name: string; tokens: string[] } | null;
  externalReferenceFiles: number | null;
  externalReferenceTests: number | null;
  sampleFiles: string[];
}

export interface RealityDriftFinding {
  severity: 'warning' | 'advisory' | string;
  kind: string;
  ownerDocId?: string;
  path?: string;
  message: string;
}

export interface RealityLegacyRule {
  id: string;
  description: string;
  sourceFiles: number;
  productionFiles: number;
  testFiles: number;
  sampleFiles: string[];
}

export type ToolingLifecycleStatus =
  | 'ACTIVE_AUTOMATED'
  | 'ACTIVE_IMPORTED'
  | 'ACTIVE_BUT_TRANSITIONAL'
  | 'MANUAL_ENTRYPOINT'
  | 'MANUAL_MILESTONE'
  | 'ORPHAN_CANDIDATE'
  | 'RETIREMENT_CANDIDATE'
  | 'UNREFERENCED_SUPPORT_DATA';

export interface ToolingGroupEvidence {
  path: string;
  status: ToolingLifecycleStatus;
  lifecycleKind: string | null;
  lifecycleNote: string | null;
  trackedFiles: number;
  executableFiles: number;
  hasReadme: boolean;
  hasTests: boolean;
  lastChangedAt: string | null;
  ageDays: number | null;
  references: {
    packageScripts: string[];
    workflows: string[];
    imports: string[];
    docs: string[];
    source: string[];
    metadata: string[];
  };
  referenceFiles: string[];
  referenceCount: number;
  weakMetadataReferences: string[];
  weakMetadataReferenceCount: number;
  sampleExecutables: string[];
}

export interface ToolingRootLegacySurface {
  path: string;
  id: string;
  note: string;
  trackedFiles: number;
  sampleFiles: string[];
}

export interface ToolingRealityData {
  source: string;
  staleThresholdDays: number;
  groups: ToolingGroupEvidence[];
  reviewCandidates: ToolingGroupEvidence[];
  transitional: ToolingGroupEvidence[];
  rootLegacySurfaces: ToolingRootLegacySurface[];
  stats: {
    trackedToolFiles: number;
    executableToolFiles: number;
    groups: number;
    reviewCandidates: number;
    transitionalGroups: number;
    rootLegacySurfaces: number;
    rootLegacyFiles: number;
    statusCounts: Record<string, number>;
  };
}

export interface RootTopologyEntry {
  path: string;
  status: 'CANONICAL_DIR' | 'CANONICAL_FILE' | 'REVIEW_ROOT' | 'REVIEW_CONFIG' | 'RETIRED_VIOLATION' | string;
  trackedFiles: number;
  ruleId: string | null;
}

export interface RootTopologyData {
  source: string;
  entries: RootTopologyEntry[];
  retiredViolations: RootTopologyEntry[];
  reviewEntries: RootTopologyEntry[];
  donorResidues: Array<{ id: string; path: string }>;
  stalePathReferences: Array<{ path: string; stalePath: string }>;
  stats: {
    rootEntries: number;
    canonicalDirs: number;
    canonicalFiles: number;
    reviewEntries: number;
    retiredViolations: number;
    donorResidues: number;
    stalePathReferences: number;
  };
}

export type UiLifecycleStatus =
  | 'ACTIVE_PRODUCTION'
  | 'TEST_ONLY'
  | 'STORY_ONLY'
  | 'FIXTURE_SUPPORT'
  | 'TEST_STORY_SUPPORT'
  | 'ORPHAN_CANDIDATE';

export interface UiFileEvidence {
  path: string;
  status: UiLifecycleStatus;
  inboundRefs: number;
  productionRefs: number;
  testRefs: number;
  storyRefs: number;
  sampleRefs: string[];
}

export interface UiPublicAssetEvidence {
  path: string;
  publicUrl: string;
  status: 'ACTIVE_REFERENCED' | 'UNREFERENCED_ASSET_CANDIDATE' | 'RUNTIME_ENTRY_REVIEW' | string;
  referenceCount: number;
  sampleRefs: string[];
}

export interface UiWorkspacePackageEvidence {
  path: string;
  name: string | null;
  status: 'ACTIVE_IMPORTED' | 'ACTIVE_TRANSITIVE' | 'MANIFEST_ONLY' | 'UNREFERENCED_PACKAGE' | string;
  rootDeclared: boolean;
  appReferenceCount: number;
  transitiveReferenceCount: number;
  sampleRefs: string[];
}

export interface UiRealityData {
  source: string;
  entrypoint: string;
  files: UiFileEvidence[];
  orphanCandidates: UiFileEvidence[];
  previewArtifacts: string[];
  retiredPublicArtifacts: string[];
  donorIdentity: Array<{ field: string; value: string; path: string }>;
  transitional: Array<{ id: string; note: string; files: string[] }>;
  publicAssets: UiPublicAssetEvidence[];
  unreferencedPublicAssets: UiPublicAssetEvidence[];
  workspacePackages: UiWorkspacePackageEvidence[];
  packageReviewCandidates: UiWorkspacePackageEvidence[];
  stats: {
    trackedUiFiles: number;
    sourceFiles: number;
    productionReachableFiles: number;
    orphanCandidates: number;
    testOnlyFiles: number;
    storyOnlyFiles: number;
    supportOnlyFiles: number;
    previewArtifacts: number;
    retiredPublicArtifacts: number;
    donorIdentityResidues: number;
    transitionalFiles: number;
    publicAssets: number;
    unreferencedPublicAssets: number;
    workspacePackages: number;
    packageReviewCandidates: number;
    statusCounts: Record<string, number>;
  };
}

export type SystemGraphFamily =
  | 'repository'
  | 'semantic'
  | 'api_protocol'
  | 'data'
  | 'execution'
  | 'ui'
  | 'tooling'
  | 'test_evidence'
  | 'deployment'
  | 'ops'
  | 'external_integration'
  | 'machine_physical';

export interface SystemCoverageNode {
  id: string;
  family: SystemGraphFamily;
  kind: string;
  label: string;
  path?: string;
  [key: string]: unknown;
}

export interface SystemCoverageEdge {
  id: string;
  family: SystemGraphFamily | 'unified';
  source: string;
  target: string;
  type: string;
  [key: string]: unknown;
}

export interface SystemCoverageGap {
  kind: string;
  artifact?: string | null;
  message: string;
}

export interface SystemCoverageFamily {
  name: SystemGraphFamily;
  nodes: SystemCoverageNode[];
  edges: SystemCoverageEdge[];
  artifacts: string[];
  gaps: SystemCoverageGap[];
}

export interface UnifiedSystemCoverage {
  nodes: SystemCoverageNode[];
  familyEdges: SystemCoverageEdge[];
  crossFamilyEdges: SystemCoverageEdge[];
  edges: SystemCoverageEdge[];
  stats: {
    nodes: number;
    familyEdges: number;
    crossFamilyEdges: number;
    edges: number;
    artifactBridges: number;
    semanticBridges: number;
    invariantBridges: number;
    endpointBridges: number;
  };
}

export interface SystemCoverageData {
  schemaVersion: number;
  source: string;
  sourceSha: string;
  graphFamilies: SystemGraphFamily[];
  families: Record<SystemGraphFamily, SystemCoverageFamily>;
  artifactClassification: Array<{ file: string; families: SystemGraphFamily[] }>;
  unclassifiedArtifacts: string[];
  unified: UnifiedSystemCoverage;
  stats: {
    graphFamilies: number;
    requiredGraphFamilies: number;
    domainCoveragePercent: number;
    trackedArtifacts: number;
    classifiedTrackedArtifacts: number;
    trackedArtifactCoveragePercent: number;
    sourceFiles: number;
    semanticMappedSourceFiles: number;
    semanticMappingPercent: number;
    totalNodes: number;
    totalEdges: number;
    totalGaps: number;
    unifiedNodes: number;
    unifiedEdges: number;
    crossFamilyEdges: number;
    familyStats: Record<SystemGraphFamily, { nodes: number; edges: number; artifacts: number; gaps: number }>;
  };
}

export interface RealityAtlasData {
  schemaVersion: number;
  source: string;
  sourceSha: string;
  workingTreeDirty: boolean;
  workingTreeStatusCount: number;
  untrackedSourceFiles: string[];
  owners: RealityOwnerEvidence[];
  runtimeOwners: RealityRuntimeOwnerEvidence[];
  legacy: {
    terminalTarget: string;
    totalSourceFiles: number;
    byRule: RealityLegacyRule[];
  };
  orphanPackages: string[];
  drift: RealityDriftFinding[];
  tooling?: ToolingRealityData;
  rootTopology?: RootTopologyData;
  ui?: UiRealityData;
  systemCoverage?: SystemCoverageData;
  stats: {
    observedSourceFiles: number;
    trackedSourceFiles: number;
    untrackedSourceFiles: number;
    architectureOwners: number;
    runtimeOwners: number;
    contractReferencesInCode: number;
    testFilesWithArchitectureRefs: number;
    legacySourceFiles: number;
    orphanPackages: number;
    driftFindings: number;
    ownerStates: Record<RealityImplementationState, number>;
  };
}

export interface DocsAtlasData {
  schemaVersion: number;
  sourceOfTruth: string;
  nodes: DocsAtlasNode[];
  edges: DocsAtlasEdge[];
  diagnostics: DocsAtlasDiagnostic[];
  reality: RealityAtlasData;
  stats: {
    documents: number;
    architectureOwners: number;
    contracts: number;
    runtimeOwners: number;
    edges: number;
    semanticEdges: number;
    ownersWithSemanticEdges: number;
    semanticOwnerCoverage: number;
  };
}
