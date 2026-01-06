# Autofix Roundtrip Test Suite Design

## Overview

A test suite that verifies lintal's auto-fix functionality produces correct, compilable code. Each test:
1. Compiles a Java file with intentional formatting violations
2. Runs `lintal fix` to auto-correct violations
3. Recompiles to verify the fix didn't break the code
4. Re-runs `lintal check` to verify zero violations remain

## Goals

- Coverage for all 25 auto-fixable rules
- Dependency-free Java fixtures (no external libraries)
- Runs as part of `cargo test --all`
- Integrated into CI pipeline

## Directory Structure

```
crates/lintal_linter/tests/
├── fixtures/
│   └── autofix/
│       ├── whitespace/
│       │   ├── whitespace_around/
│       │   │   ├── default/
│       │   │   │   ├── checkstyle.xml
│       │   │   │   └── Input.java
│       │   │   ├── allow_empty_methods/
│       │   │   ├── allow_empty_types/
│       │   │   ├── allow_empty_constructors/
│       │   │   └── allow_empty_lambdas/
│       │   ├── whitespace_after/
│       │   │   └── default/
│       │   ├── no_whitespace_after/
│       │   │   └── default/
│       │   ├── no_whitespace_before/
│       │   │   └── default/
│       │   ├── paren_pad/
│       │   │   ├── nospace/
│       │   │   └── space/
│       │   ├── method_param_pad/
│       │   │   ├── nospace/
│       │   │   └── space/
│       │   ├── typecast_paren_pad/
│       │   │   ├── nospace/
│       │   │   └── space/
│       │   ├── empty_for_initializer_pad/
│       │   │   ├── nospace/
│       │   │   └── space/
│       │   ├── single_space_separator/
│       │   │   └── default/
│       │   ├── empty_line_separator/
│       │   │   └── default/
│       │   ├── operator_wrap/
│       │   │   ├── nl/
│       │   │   └── eol/
│       │   └── file_tab_character/
│       │       └── default/
│       ├── modifier/
│       │   ├── modifier_order/
│       │   │   └── default/
│       │   ├── final_local_variable/
│       │   │   └── default/
│       │   ├── final_parameters/
│       │   │   └── default/
│       │   └── redundant_modifier/
│       │       └── default/
│       ├── imports/
│       │   ├── unused_imports/
│       │   │   └── default/
│       │   └── redundant_import/
│       │       └── default/
│       ├── blocks/
│       │   ├── left_curly/
│       │   │   ├── eol/
│       │   │   ├── nl/
│       │   │   └── nlow/
│       │   ├── right_curly/
│       │   │   ├── same/
│       │   │   └── alone/
│       │   └── need_braces/
│       │       └── default/
│       ├── coding/
│       │   ├── one_statement_per_line/
│       │   │   └── default/
│       │   └── multiple_variable_declarations/
│       │       └── default/
│       └── style/
│           ├── upper_ell/
│           │   └── default/
│           └── array_type_style/
│               ├── java/
│               └── c/
└── autofix_roundtrip.rs
```

Each leaf directory contains:
- `checkstyle.xml` - Minimal config enabling just that rule with specific options
- `Input.java` - Compilable Java 25 code with intentional formatting violations

## Test Harness

Location: `crates/lintal_linter/tests/autofix_roundtrip.rs`

```rust
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

struct Fixture {
    name: String,
    input_java: PathBuf,
    checkstyle_xml: PathBuf,
}

fn discover_fixtures(base: &Path) -> Vec<Fixture> {
    // Walk directory tree, find dirs with both checkstyle.xml and Input.java
    let mut fixtures = Vec::new();
    for entry in walkdir::WalkDir::new(base) {
        let entry = entry.unwrap();
        if entry.file_type().is_dir() {
            let dir = entry.path();
            let xml = dir.join("checkstyle.xml");
            let java = dir.join("Input.java");
            if xml.exists() && java.exists() {
                fixtures.push(Fixture {
                    name: dir.strip_prefix(base).unwrap().display().to_string(),
                    input_java: java,
                    checkstyle_xml: xml,
                });
            }
        }
    }
    fixtures
}

fn javac_available() -> bool {
    Command::new("javac")
        .arg("--version")
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn lintal_binary() -> PathBuf {
    // Find the built lintal binary
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/debug/lintal")
}

#[test]
fn test_autofix_roundtrip() {
    if !javac_available() {
        eprintln!("Skipping autofix roundtrip test: javac not found");
        return;
    }

    let fixtures_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/autofix");

    let fixtures = discover_fixtures(&fixtures_dir);
    assert!(!fixtures.is_empty(), "No fixtures found");

    for fixture in fixtures {
        let temp_dir = tempdir().unwrap();
        let test_file = temp_dir.path().join("Input.java");
        fs::copy(&fixture.input_java, &test_file).unwrap();

        // 1. Compile original - must succeed
        let status = Command::new("javac")
            .arg(&test_file)
            .status()
            .expect("Failed to run javac");
        assert!(
            status.success(),
            "{}: original file must compile",
            fixture.name
        );

        // 2. Run lintal fix
        let status = Command::new(lintal_binary())
            .args(["fix", test_file.to_str().unwrap()])
            .arg("--config")
            .arg(&fixture.checkstyle_xml)
            .status()
            .expect("Failed to run lintal fix");
        assert!(
            status.success(),
            "{}: lintal fix failed",
            fixture.name
        );

        // 3. Compile fixed - must still succeed
        let status = Command::new("javac")
            .arg(&test_file)
            .status()
            .expect("Failed to run javac on fixed file");
        assert!(
            status.success(),
            "{}: fixed code must compile",
            fixture.name
        );

        // 4. Re-check - must have zero violations for this rule
        let output = Command::new(lintal_binary())
            .args(["check", test_file.to_str().unwrap()])
            .arg("--config")
            .arg(&fixture.checkstyle_xml)
            .output()
            .expect("Failed to run lintal check");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("0 violations") || stdout.is_empty(),
            "{}: violations remain after fix: {}",
            fixture.name,
            stdout
        );
    }
}
```

## Example Fixtures

### WhitespaceAround (default)

**checkstyle.xml:**
```xml
<?xml version="1.0"?>
<!DOCTYPE module PUBLIC "-//Checkstyle//DTD Checkstyle Configuration 1.3//EN"
  "https://checkstyle.org/dtds/configuration_1_3.dtd">
<module name="Checker">
  <module name="TreeWalker">
    <module name="WhitespaceAround"/>
  </module>
</module>
```

**Input.java:**
```java
public class Input {
    public void method() {
        int x=1;
        if(x==1){
            x=x+1;
        }
    }
}
```

### ModifierOrder (default)

**Input.java:**
```java
public class Input {
    final public static int X = 1;
    synchronized public void method() {}
}
```

### UnusedImports (default)

**Input.java:**
```java
import java.util.List;
import java.util.Map;
import java.util.ArrayList;

public class Input {
    List<String> items = new ArrayList<>();
}
```

## CI Integration

Update `.github/workflows/ci.yml` to include Java 25:

```yaml
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Set up Java 25
        uses: actions/setup-java@v4
        with:
          distribution: 'temurin'
          java-version: '25'

      - name: Set up Rust
        uses: dtolnay/rust-action@stable

      - name: Run tests
        run: cargo test --all
```

The roundtrip test gracefully skips if `javac` is not available, allowing local development without Java.

## Coverage Summary

| Category | Rules | Fixtures |
|----------|-------|----------|
| Whitespace | 12 | ~20 |
| Modifier | 4 | 4 |
| Imports | 2 | 2 |
| Blocks | 3 | 6 |
| Coding | 2 | 2 |
| Style | 2 | 3 |
| **Total** | **25** | **~40** |

## Success Criteria

1. All ~40 fixtures pass the roundtrip test
2. CI runs the test on every PR
3. Any auto-fix regression breaks the build
4. Test execution time under 60 seconds
