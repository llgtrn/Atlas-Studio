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
    // `trim_end` only -- NEVER `trim()`. `git status --porcelain=v1` encodes real state in a
    // fixed two-column `XY` code at the START of each line, where a literal leading space is
    // meaningful (`" M path"` = an UNSTAGED modification, `"M  path"` = a STAGED one -- opposite
    // git states). Porcelain entries are emitted in path-sorted order, not grouped by status, so
    // whichever entry happens to sort first is exposed: a whole-string `.trim()` here would strip
    // that entry's leading space before `snapshot_git` ever splits the blob into lines, silently
    // flipping a genuinely unstaged change into one that looks staged in `RepositorySnapshot.
    // status_entries` -- the literal audit trail backing EXACT_BASE_SHA_REQUIRED provenance.
    Ok(String::from_utf8_lossy(&output.stdout)
        .trim_end()
        .to_owned())
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

#[cfg(test)]
mod tests {
    use super::*;

    fn run_git(dir: &Path, args: &[&str]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(dir)
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?} failed");
    }

    /// Falsification target: `git status --porcelain=v1` encodes real state in a fixed
    /// two-column `XY` code at the START of each line -- a leading space is meaningful (`" M"` =
    /// unstaged modification, `"M "` = staged). Before this fix, `git()`'s whole-string `.trim()`
    /// stripped that leading space off whichever entry happened to sort first, silently making a
    /// genuinely UNSTAGED change look STAGED in `RepositorySnapshot.status_entries`. This is the
    /// single most common real git state ("edited a file, never `git add`ed it"), not an edge case.
    #[test]
    fn an_unstaged_only_modification_keeps_its_leading_space_status_code() {
        let dir = std::env::temp_dir().join(format!(
            "atlas-vcs-test-unstaged-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        run_git(&dir, &["init", "--quiet"]);
        run_git(&dir, &["config", "user.email", "test@example.com"]);
        run_git(&dir, &["config", "user.name", "test"]);
        std::fs::write(dir.join("file.txt"), "hello\n").unwrap();
        run_git(&dir, &["add", "file.txt"]);
        run_git(&dir, &["commit", "--quiet", "-m", "init"]);
        // An unstaged-only modification -- never staged -- produces real git's `" M file.txt"`,
        // with a literal leading space distinguishing it from a staged `"M  file.txt"`.
        std::fs::write(dir.join("file.txt"), "hello\nworld\n").unwrap();

        let snapshot = snapshot_git(&dir).unwrap();

        assert!(snapshot.dirty);
        assert_eq!(
            snapshot.status_entries,
            vec![" M file.txt".to_owned()],
            "an unstaged modification's leading space (its real XY status code) must survive, \
             not be silently stripped as if it were incidental whitespace"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
