//! Autofix roundtrip tests.
//!
//! These tests verify that lintal's auto-fix functionality produces
//! correct, compilable Java code. For each fixture:
//! 1. Compile original Java file (must succeed)
//! 2. Run lintal fix
//! 3. Compile fixed file (must still succeed)
//! 4. Run lintal check (must report zero violations)

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;
use walkdir::WalkDir;

/// A test fixture consisting of a checkstyle.xml config and Input.java file.
struct Fixture {
    /// Human-readable name (path relative to fixtures dir)
    name: String,
    /// Path to the Input.java file
    input_java: PathBuf,
    /// Path to the checkstyle.xml config
    checkstyle_xml: PathBuf,
}

/// Discover all fixture directories under the given base path.
/// A fixture directory must contain both `checkstyle.xml` and `Input.java`.
fn discover_fixtures(base: &Path) -> Vec<Fixture> {
    let mut fixtures = Vec::new();

    for entry in WalkDir::new(base).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_dir() {
            let dir = entry.path();
            let xml = dir.join("checkstyle.xml");
            let java = dir.join("Input.java");

            if xml.exists() && java.exists() {
                let name = dir
                    .strip_prefix(base)
                    .unwrap_or(dir)
                    .display()
                    .to_string();

                fixtures.push(Fixture {
                    name,
                    input_java: java,
                    checkstyle_xml: xml,
                });
            }
        }
    }

    fixtures
}

/// Check if javac is available in PATH.
fn javac_available() -> bool {
    Command::new("javac")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Get the path to the lintal binary (built by cargo).
fn lintal_binary() -> PathBuf {
    // The test is run from crates/lintal_linter, so we need to go up to find target/
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.parent().unwrap().parent().unwrap();

    // Try release first, then debug
    let release = workspace_root.join("target/release/lintal");
    if release.exists() {
        return release;
    }

    workspace_root.join("target/debug/lintal")
}

/// Run the roundtrip test for a single fixture.
fn test_fixture(fixture: &Fixture) -> Result<(), String> {
    let lintal = lintal_binary();
    if !lintal.exists() {
        return Err(format!(
            "lintal binary not found at {:?}. Run `cargo build` first.",
            lintal
        ));
    }

    // Create temp directory and copy the input file
    let temp_dir = TempDir::new().map_err(|e| format!("Failed to create temp dir: {}", e))?;
    let test_file = temp_dir.path().join("Input.java");
    fs::copy(&fixture.input_java, &test_file)
        .map_err(|e| format!("Failed to copy input file: {}", e))?;

    // Step 1: Compile original - must succeed
    let output = Command::new("javac")
        .arg(&test_file)
        .output()
        .map_err(|e| format!("Failed to run javac: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Original file failed to compile:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    // Remove .class file to ensure clean recompilation
    let class_file = temp_dir.path().join("Input.class");
    let _ = fs::remove_file(&class_file);

    // Step 2: Run lintal fix
    let output = Command::new(&lintal)
        .arg("fix")
        .arg(&test_file)
        .arg("--config")
        .arg(&fixture.checkstyle_xml)
        .output()
        .map_err(|e| format!("Failed to run lintal fix: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "lintal fix failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    // Step 3: Compile fixed - must still succeed
    let output = Command::new("javac")
        .arg(&test_file)
        .output()
        .map_err(|e| format!("Failed to run javac on fixed file: {}", e))?;

    if !output.status.success() {
        // Read the fixed file content for debugging
        let fixed_content = fs::read_to_string(&test_file).unwrap_or_default();
        return Err(format!(
            "Fixed file failed to compile:\n{}\n\nFixed content:\n{}",
            String::from_utf8_lossy(&output.stderr),
            fixed_content
        ));
    }

    // Step 4: Run lintal check - must have zero violations
    let output = Command::new(&lintal)
        .arg("check")
        .arg(&test_file)
        .arg("--config")
        .arg(&fixture.checkstyle_xml)
        .output()
        .map_err(|e| format!("Failed to run lintal check: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check for "No violations found" in output
    if !stdout.contains("No violations found") {
        let fixed_content = fs::read_to_string(&test_file).unwrap_or_default();
        return Err(format!(
            "Violations remain after fix:\n{}\n\nFixed content:\n{}",
            stdout, fixed_content
        ));
    }

    Ok(())
}

#[test]
fn test_autofix_roundtrip() {
    if !javac_available() {
        if std::env::var("CI").is_ok() {
            panic!("javac not found in CI environment - Java setup may be missing");
        }
        eprintln!("Skipping autofix roundtrip test: javac not found in PATH");
        return;
    }

    let fixtures_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/autofix");

    if !fixtures_dir.exists() {
        if std::env::var("CI").is_ok() {
            panic!("Fixtures directory not found in CI: {:?}", fixtures_dir);
        }
        eprintln!(
            "Skipping autofix roundtrip test: fixtures directory not found at {:?}",
            fixtures_dir
        );
        return;
    }

    let fixtures = discover_fixtures(&fixtures_dir);

    if fixtures.is_empty() {
        eprintln!("No fixtures found in {:?}", fixtures_dir);
        return;
    }

    println!("Found {} fixtures", fixtures.len());

    let mut failures = Vec::new();

    for fixture in &fixtures {
        print!("Testing {}... ", fixture.name);
        match test_fixture(fixture) {
            Ok(()) => println!("OK"),
            Err(e) => {
                println!("FAILED");
                failures.push((fixture.name.clone(), e));
            }
        }
    }

    if !failures.is_empty() {
        eprintln!("\n{} fixture(s) failed:", failures.len());
        for (name, error) in &failures {
            eprintln!("\n=== {} ===\n{}", name, error);
        }
        panic!("{} fixture(s) failed", failures.len());
    }
}
