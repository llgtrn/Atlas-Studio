//! Agent global context: what an agent working inside one shard must know about the rest of
//! Chronica before it builds anything substantial — what exists, who physically holds it, how
//! to depend on it, which canonical technologies it should reuse instead of re-inventing, and
//! the current migration and extinction state. Generated from the graph, never hand-written.

use crate::linker::SystemImage;
use crate::schema::{DonorState, TechnologyLifecycle, WaveStatus};
use crate::Assessment;

pub fn render(a: &Assessment, system: Option<&SystemImage>) -> String {
    let d = &a.declaration;
    let shard = &d.repository.shard;
    let mut s = format!(
        "SYSTEM\n  {} (Ynventa protocol v{}, schema {})\n\nCURRENT PHYSICAL SHARD\n  {} ({}) head {}\n\n",
        d.repository.system,
        crate::YNVENTA_PROTOCOL_VERSION,
        crate::protocol::schema_identity(),
        shard,
        d.repository.origin,
        crate::repository::files::head_commit(&a.root).unwrap_or_else(|| "-".into())
    );
    s.push_str(&format!(
        "THIS SHARD OWNS ({} nodes; identities are Chronica's: ynv://chronica/<key>)\n",
        d.repository.nodes.len()
    ));
    for n in &d.repository.nodes {
        s.push_str(&format!(
            "  {:<36} {:<12} {}{}\n",
            n.key,
            n.kind.wire(),
            n.path,
            if n.path != n.canonical_path {
                format!(" -> {}", n.canonical_path)
            } else {
                String::new()
            }
        ));
    }
    let provides: Vec<&String> = d
        .repository
        .nodes
        .iter()
        .flat_map(|n| &n.provides)
        .collect();
    let requires: Vec<&String> = d
        .repository
        .nodes
        .iter()
        .flat_map(|n| &n.requires)
        .filter(|c| !provides.contains(c))
        .collect();
    s.push_str(&format!(
        "\nPROVIDES\n  {}\n",
        if provides.is_empty() {
            "(none declared)".into()
        } else {
            provides
                .iter()
                .map(|x| x.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        }
    ));
    s.push_str("\nREQUIRES (depend on the capability, never on another shard's paths)\n");
    if requires.is_empty() {
        s.push_str("  (none declared)\n");
    }
    for r in &requires {
        let providers = system
            .and_then(|sys| sys.capabilities.iter().find(|c| &c.key == *r))
            .map(|c| {
                c.providers
                    .iter()
                    .map(|(sh, n)| format!("{n} in {sh}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .filter(|p| !p.is_empty())
            .unwrap_or_else(|| "UNPROVIDED".into());
        s.push_str(&format!("  {r} <- {providers}\n"));
    }
    if let Some(sys) = system {
        s.push_str("\nAVAILABLE GLOBAL CAPABILITIES (physical owner)\n");
        for c in sys
            .capabilities
            .iter()
            .filter(|c| c.providers.iter().any(|(sh, _)| sh != shard))
        {
            s.push_str(&format!(
                "  {:<36} {}\n",
                c.key,
                c.providers
                    .iter()
                    .map(|(sh, n)| format!("{n}@{sh}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        s.push_str("\nCANONICAL TECHNOLOGIES (reuse natively: `ynventa technology materialize <key>`; no service calls)\n");
        for t in sys.technologies.iter().filter(|t| {
            t.effective >= TechnologyLifecycle::Canonical
                && t.effective != TechnologyLifecycle::Superseded
        }) {
            s.push_str(&format!(
                "  {:<36} {:<10} implements {} (born in {})\n",
                t.key,
                t.effective.wire(),
                t.implements.join(", "),
                t.birthplace
            ));
        }
        s.push_str("\nFORBIDDEN DUPLICATION (already provided elsewhere; declare SPECIALIZES / ALTERNATIVE_FOR if you must differ)\n");
        for c in sys
            .capabilities
            .iter()
            .filter(|c| !c.providers.is_empty() && c.providers.iter().all(|(sh, _)| sh != shard))
        {
            s.push_str(&format!("  capability {}\n", c.key));
        }
        for t in sys
            .technologies
            .iter()
            .filter(|t| t.birthplace != *shard && t.effective >= TechnologyLifecycle::Native)
        {
            s.push_str(&format!(
                "  technology {} ({})\n",
                t.key,
                t.implements.join(", ")
            ));
        }
        s.push_str(&format!(
            "\nSYSTEM LINK\n  {} ({} shards, {} errors)\n",
            if sys.pass() { "PASS" } else { "FAIL" },
            sys.shards.len(),
            sys.issues
                .iter()
                .filter(|i| i.severity == crate::Severity::Error)
                .count()
        ));
    } else {
        s.push_str("\n(no system image given: pass --system target/ynventa/chronica.system.ynv for global capabilities and technologies)\n");
    }
    let applied = d
        .migration
        .waves
        .iter()
        .filter(|w| w.status == WaveStatus::Applied)
        .count();
    s.push_str(&format!(
        "\nMIGRATION STATE\n  waves applied {applied}/{}; legacy nodes remaining {}; active shims {}; .atlas present: {}\n",
        d.migration.waves.len(),
        a.metric("legacy_nodes_remaining"),
        a.migration.shims.iter().filter(|x| x.present).count(),
        a.files.exists(".atlas")
    ));
    let extinct = a
        .analysis
        .donors
        .iter()
        .filter(|x| x.effective == DonorState::Extinct)
        .count();
    s.push_str(&format!(
        "\nEXTINCTION STATE\n  donors extinct {extinct} / registered {}; external technology edges {}; extinction ratio {}\n",
        a.metric("donors_registered"),
        a.metric("external_technology_edges"),
        a.metric("extinction_ratio")
    ));
    s.push_str("\nBEFORE BUILDING INFRASTRUCTURE\n  ynventa technology search <need>  ·  ynventa show <key> --system <image>  ·  never `cargo add` a donor\n");
    s
}
