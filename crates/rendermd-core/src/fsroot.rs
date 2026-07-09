//! Confinement for the `preview://.../fs/` route.
//!
//! Threat: an attacker-authored markdown file runs script in the preview
//! frame (comrak renders raw HTML) and can request any `/fs/<path>` URL, or
//! embed `![](../../../../etc/passwd)`. Without confinement that is an
//! arbitrary local-file read of anything the process can open.
//!
//! Policy: reads are confined to an **allowed root** — the document's git
//! repository top-level when it has one (so legitimate `../assets/img.png`
//! references that stay inside the repo keep working), otherwise the
//! document's own directory. A requested path is served only if, after
//! resolving symlinks and `..`, it still lies within that root.

use std::path::{Path, PathBuf};
use std::process::Command;

fn git_command() -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new("git");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    cmd
}

/// The read-confinement root for a document: its git top-level, else its
/// parent directory. Canonicalized so later `starts_with` checks compare
/// real paths (symlinks resolved).
pub fn allowed_root(doc_path: &Path) -> Option<PathBuf> {
    let dir = doc_path.parent()?;
    let toplevel = git_command()
        .arg("-C")
        .arg(dir)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .map(PathBuf::from);
    let root = toplevel.unwrap_or_else(|| dir.to_path_buf());
    // Canonicalize; fall back to the raw dir if that fails (e.g. the dir was
    // removed) so we still confine rather than opening up.
    Some(std::fs::canonicalize(&root).unwrap_or(root))
}

/// Resolve a `/fs/`-requested filesystem path and confirm it is inside
/// `root`. Returns the canonical path to read, or None to 404. `None` root
/// (no document open) denies everything.
pub fn resolve_within(root: Option<&Path>, requested: &Path) -> Option<PathBuf> {
    let root = root?;
    // Canonicalize the target: resolves `..` and symlinks to a real path.
    // Fails for nonexistent files — which we want to 404 anyway.
    let canonical = std::fs::canonicalize(requested).ok()?;
    if canonical.starts_with(root) {
        Some(canonical)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn confines_to_root_and_blocks_escapes() {
        let tmp = std::env::temp_dir().join(format!("rmd-fsroot-{}", std::process::id()));
        let root = tmp.join("repo");
        let sub = root.join("assets");
        fs::create_dir_all(&sub).unwrap();
        fs::write(root.join("ok.png"), b"x").unwrap();
        fs::write(sub.join("deep.png"), b"x").unwrap();
        let outside = tmp.join("secret.txt");
        fs::write(&outside, b"secret").unwrap();

        let root_c = fs::canonicalize(&root).unwrap();

        // In-root files resolve.
        assert!(resolve_within(Some(&root_c), &root.join("ok.png")).is_some());
        assert!(resolve_within(Some(&root_c), &sub.join("deep.png")).is_some());
        // `..` that stays in root is fine.
        assert!(resolve_within(Some(&root_c), &sub.join("../ok.png")).is_some());

        // Escapes are denied.
        assert!(resolve_within(Some(&root_c), &root.join("../secret.txt")).is_none());
        assert!(resolve_within(Some(&root_c), Path::new("/etc/passwd")).is_none());
        // Nonexistent → None (404).
        assert!(resolve_within(Some(&root_c), &root.join("nope.png")).is_none());
        // No document open → deny all.
        assert!(resolve_within(None, &root.join("ok.png")).is_none());

        let _ = fs::remove_dir_all(&tmp);
    }
}
