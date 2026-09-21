use atlas_core::RepositorySnapshot;
use std::{io, path::Path, process::Command};

fn git(root: &Path, args: &[&str]) -> io::Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(io::Error::other(format!(
            "git {} failed: {}",
            args.join(" "),
            stderr.trim()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

pub fn snapshot_git(root: impl AsRef<Path>) -> io::Result<RepositorySnapshot> {
    let root = root.as_ref().canonicalize()?;
    let head_sha = git(&root, &["rev-parse", "HEAD"])?;
    let branch = git(&root, &["branch", "--show-current"])
        .ok()
        .filter(|value| !value.is_empty());
    let status_text = git(&root, &["status", "--porcelain=v1"])?;
    let status_entries = status_text
        .lines()
        .map(|line| line.trim_end().to_owned())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    Ok(RepositorySnapshot {
        schema: "atlas.repository-snapshot.v1".into(),
        root: root.to_string_lossy().into_owned(),
        head_sha,
        branch,
        dirty: !status_entries.is_empty(),
        status_entries,
    })
}
