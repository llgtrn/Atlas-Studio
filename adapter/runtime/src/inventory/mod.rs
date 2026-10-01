use atlas_core::{InventoryReport, RepoManifest};
use std::{io, path::Path};

pub fn build_inventory(
    root: &Path,
    manifest: Option<&RepoManifest>,
) -> io::Result<InventoryReport> {
    match manifest {
        Some(manifest) => adapter::inventory_declared_source(root, manifest),
        None => adapter::inventory_source(root),
    }
}
