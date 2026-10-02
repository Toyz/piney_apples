//! The commit the game is built from, as `PINEY_BUILD` (seven hex digits,
//! empty when unknown), for the launcher's credit, the console's `version`
//! and the pad log. Read from the `.git` files themselves, so building
//! does not need git installed; a tree without `.git` (a ZIP download)
//! builds with none, unless `PINEY_COMMIT` names it.

use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-env-changed=PINEY_COMMIT");
    let hash = std::env::var("PINEY_COMMIT").ok().or_else(head).unwrap_or_default();
    let short: String = hash.chars().take_while(char::is_ascii_hexdigit).take(7).collect();
    println!("cargo:rustc-env=PINEY_BUILD={short}");
}

/// The commit `HEAD` names, watching the files that change with it.
fn head() -> Option<String> {
    let manifest = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR")?);
    // The crate, crates/, the workspace: not a repository the tree sits in.
    let dot = manifest.ancestors().take(3).map(|d| d.join(".git")).find(|p| p.exists())?;
    let git = git_dir(&dot)?;
    // A worktree keeps HEAD in its own folder and the refs in the main one.
    let common = match fs::read_to_string(git.join("commondir")) {
        Ok(c) => git.join(c.trim()),
        Err(_) => git.clone(),
    };
    watch(&git.join("HEAD"));
    let head = fs::read_to_string(git.join("HEAD")).ok()?;
    let Some(name) = head.trim().strip_prefix("ref: ") else {
        return Some(head.trim().to_string());
    };
    watch(&common.join("refs/heads"));
    watch(&common.join("packed-refs"));
    if let Ok(h) = fs::read_to_string(common.join(name)) {
        return Some(h.trim().to_string());
    }
    let packed = fs::read_to_string(common.join("packed-refs")).ok()?;
    packed.lines().filter_map(|l| l.split_once(' ')).find(|&(_, r)| r == name).map(|(h, _)| h.to_string())
}

/// `.git` is the folder, or in a worktree a file naming it.
fn git_dir(dot: &Path) -> Option<PathBuf> {
    if dot.is_dir() {
        return Some(dot.to_path_buf());
    }
    let text = fs::read_to_string(dot).ok()?;
    let to = text.trim().strip_prefix("gitdir: ")?;
    Some(dot.parent()?.join(to))
}

fn watch(p: &Path) {
    if p.exists() {
        println!("cargo:rerun-if-changed={}", p.display());
    }
}
