use super::*;
use crate::census::extraction::{
    CensusExtractionAccounting, extract_semantics, requested_dimensions,
};
use atlas_core::{ArtifactId, ArtifactKind, ArtifactRecord, RepositoryId, RevisionRef};
use std::time::{SystemTime, UNIX_EPOCH};

fn artifact(path: &str, language: &str) -> ArtifactRecord {
    ArtifactRecord {
        id: ArtifactId::new(format!("artifact:{path}")),
        path: path.to_owned(),
        kind: ArtifactKind::File,
        bytes: 0,
        disposition: ArtifactDisposition::Parsed,
        language: Some(language.into()),
        reason: None,
        content_digest: None,
        content_digest_withheld: None,
    }
}

#[test]
fn imported_callees_resolve_across_files_and_workspace_packages() {
    // G158: a call of an imported bare identifier is observed again, resolved to the
    // module-level function the import names, through a workspace package's declared `source`
    // entry and its star exports; a member call, a registry import and a rebound name are not.
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir =
        std::env::temp_dir().join(format!("atlas-g158-modules-{}-{nonce}", std::process::id()));
    let files = [
        (
            "packages/system/package.json",
            "{ \"name\": \"@x/system\", \"main\": \"dist/index.js\", \"source\": \"src/index.ts\" }",
            "json",
        ),
        (
            "packages/system/src/index.ts",
            "export * from './utils';\n",
            "typescript",
        ),
        (
            "packages/system/src/utils/index.ts",
            "export * from './edge';\n",
            "typescript",
        ),
        (
            "packages/system/src/utils/edge.ts",
            "export function getPath(a: number) {\n  return a;\n}\n",
            "typescript",
        ),
        (
            "packages/react/src/Edge.tsx",
            "import { getPath } from '@x/system';\nimport { memo } from 'react';\nimport * as sys from '@x/system';\nexport const Edge = memo(() => {\n  const p = getPath(1);\n  sys.getPath(2);\n  return p;\n});\n",
            "typescript",
        ),
        (
            "packages/react/src/Rebound.ts",
            "import { getPath } from '@x/system';\nfunction run(getPath: () => void) {\n  getPath();\n}\n",
            "typescript",
        ),
    ];
    for (path, text, _) in files {
        let full = dir.join(path);
        fs::create_dir_all(full.parent().unwrap()).unwrap();
        fs::write(full, text).unwrap();
    }
    let inventory = InventoryReport::new(
        dir.to_string_lossy().into_owned(),
        files
            .iter()
            .map(|(path, _, language)| artifact(path, language))
            .collect(),
    );
    let revision = RevisionRef {
        kind: "git".into(),
        value: "abc123".into(),
    };
    let batches = extract_semantics(&inventory, RepositoryId::new("atlas-studio"), revision);
    // G158: RESOURCE is canonical, so the syntactic extractor accounts for it (unsupported).
    for batch in &batches {
        let resource = batch
            .obligations
            .iter()
            .find(|o| o.dimension == SemanticDimension::Resource)
            .expect("every extractor accounts for RESOURCE");
        assert_eq!(resource.status, EpistemicStatus::Unsupported);
    }
    let linked = resolve_typescript_modules(&inventory, &batches);
    assert_eq!(linked.len(), 5, "one batch per TypeScript artifact");
    let definition = batches
        .iter()
        .flat_map(|b| &b.observations)
        .find_map(|o| match o {
            SemanticObservation::FunctionIdentity(h) if h.subject.symbol.name == "getPath" => {
                Some(h.record_id.clone())
            }
            _ => None,
        })
        .unwrap();
    let resolved: Vec<(String, usize, String)> = linked
        .iter()
        .flat_map(|b| &b.observations)
        .filter_map(|o| match o {
            SemanticObservation::Call(h) => {
                assert_eq!(h.status, EpistemicStatus::Derived);
                assert_eq!(h.extractor.id, TYPESCRIPT_MODULE_RESOLUTION_ID);
                assert_eq!(h.subject.dispatch, CallDispatchKind::StaticResolved);
                assert_eq!(h.subject.callees, std::slice::from_ref(&definition));
                Some((
                    h.subject.span.path.clone(),
                    h.subject.span.line,
                    h.subject.callee_spelling.clone().unwrap(),
                ))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        resolved,
        [(
            "packages/react/src/Edge.tsx".to_owned(),
            5,
            "getPath".to_owned()
        )],
        "the namespace member call and the call of a parameter named like the import stay unresolved"
    );
    // The claim observed is the syntactic extractor's own record.
    let claimed = batches
        .iter()
        .flat_map(|b| &b.observations)
        .any(|o| matches!(o, SemanticObservation::Call(h) if h.subject.span.line == 5 && h.subject.span.path.ends_with("Edge.tsx") && linked.iter().flat_map(|b| &b.observations).any(|l| l.record_id() == &h.record_id)));
    assert!(claimed);
    for batch in &linked {
        assert_eq!(batch.obligations.len(), 1);
        assert_eq!(batch.obligations[0].dimension, SemanticDimension::Call);
        assert_eq!(batch.obligations[0].status, EpistemicStatus::Unknown);
    }
    let mut accounting = CensusExtractionAccounting::new();
    for batch in batches.iter().chain(&linked) {
        accounting.record_batch(batch);
    }
    assert!(accounting.is_closed_per_extractor(requested_dimensions));
    fs::remove_dir_all(&dir).unwrap();
}
