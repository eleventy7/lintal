//! Helper to fetch checkstyle repository for compatibility testing.
//!
//! This module clones the checkstyle repository at test time to avoid
//! bundling LGPL-licensed test files in our repository.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Once;

/// Pinned checkstyle commit (release checkstyle-12.3.0)
const CHECKSTYLE_COMMIT: &str = "7cd24ce03ffa97cf30b565a536d537fd89a84e6c";
const CHECKSTYLE_REPO: &str = "https://github.com/checkstyle/checkstyle.git";

static INIT: Once = Once::new();

/// Get the path to the checkstyle repository, cloning it if necessary.
///
/// Returns `None` if git is not available or clone fails.
pub fn checkstyle_repo() -> Option<PathBuf> {
    let target_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()? // lintal_linter -> crates
        .parent()? // crates -> lintal
        .join("target")
        .join("checkstyle-tests");

    INIT.call_once(|| {
        if let Err(e) = ensure_repo(&target_dir) {
            eprintln!("Warning: Failed to fetch checkstyle repo: {}", e);
        }
    });

    if target_dir.join(".git").exists() {
        Some(target_dir)
    } else {
        None
    }
}

/// Get path to a checkstyle test input file for any whitespace check.
#[allow(dead_code)]
pub fn whitespace_test_input(check_name: &str, file_name: &str) -> Option<PathBuf> {
    let repo = checkstyle_repo()?;
    let path = repo
        .join("src/test/resources/com/puppycrawl/tools/checkstyle/checks/whitespace")
        .join(check_name.to_lowercase())
        .join(file_name);

    if path.exists() { Some(path) } else { None }
}

/// Get path to a checkstyle test input file for misc checks (UpperEll, ArrayTypeStyle, etc.)
#[allow(dead_code)]
pub fn misc_test_input(check_name: &str, file_name: &str) -> Option<PathBuf> {
    let repo = checkstyle_repo()?;
    let path = repo
        .join("src/test/resources/com/puppycrawl/tools/checkstyle/checks")
        .join(check_name.to_lowercase())
        .join(file_name);

    if path.exists() { Some(path) } else { None }
}

fn ensure_repo(target_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if target_dir.join(".git").exists() {
        // Repo exists, verify we're at the right commit
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(target_dir)
            .output()?;

        let current = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if current == CHECKSTYLE_COMMIT {
            return Ok(());
        }

        // Wrong commit, fetch and checkout
        eprintln!("Updating checkstyle repo to {}", &CHECKSTYLE_COMMIT[..12]);
        Command::new("git")
            .args(["fetch", "origin"])
            .current_dir(target_dir)
            .status()?;

        Command::new("git")
            .args(["checkout", CHECKSTYLE_COMMIT])
            .current_dir(target_dir)
            .status()?;
    } else {
        // Clone fresh
        eprintln!("Cloning checkstyle repo for compatibility tests (one-time)...");
        std::fs::create_dir_all(target_dir)?;

        // Shallow clone with just the commit we need
        Command::new("git")
            .args([
                "clone",
                "--filter=blob:none",
                "--no-checkout",
                CHECKSTYLE_REPO,
            ])
            .arg(target_dir)
            .status()?;

        Command::new("git")
            .args(["checkout", CHECKSTYLE_COMMIT])
            .current_dir(target_dir)
            .status()?;
    }

    Ok(())
}
