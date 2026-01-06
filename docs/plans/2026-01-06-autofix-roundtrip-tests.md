# Autofix Roundtrip Test Suite Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Create a test suite that verifies lintal's auto-fix produces correct, compilable Java code for all 25 fixable rules.

**Architecture:** Rust integration test discovers fixture directories, each containing a minimal `checkstyle.xml` and `Input.java`. Test copies fixture to temp dir, compiles with javac, runs `lintal fix`, recompiles, and verifies zero violations remain.

**Tech Stack:** Rust integration tests, tempfile crate, walkdir crate, std::process::Command for javac/lintal invocation.

---

### Task 1: Add dev-dependencies to Cargo.toml

**Files:**
- Modify: `crates/lintal_linter/Cargo.toml`

**Step 1: Add tempfile and walkdir dependencies**

Add to `[dev-dependencies]` section:
```toml
[dev-dependencies]
tempfile = "3"
walkdir = "2"
```

**Step 2: Verify it compiles**

Run: `cargo check --package lintal_linter`
Expected: Success

**Step 3: Commit**

```bash
git add crates/lintal_linter/Cargo.toml
git commit -m "chore: add tempfile and walkdir dev-dependencies for autofix tests"
```

---

### Task 2: Create test harness skeleton

**Files:**
- Create: `crates/lintal_linter/tests/autofix_roundtrip.rs`

**Step 1: Create the test file**

```rust
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
        eprintln!("Skipping autofix roundtrip test: javac not found in PATH");
        return;
    }

    let fixtures_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/autofix");

    if !fixtures_dir.exists() {
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
```

**Step 2: Create fixtures directory**

```bash
mkdir -p crates/lintal_linter/tests/fixtures/autofix
```

**Step 3: Verify test compiles (will skip since no fixtures yet)**

Run: `cargo test --package lintal_linter --test autofix_roundtrip -- --nocapture`
Expected: Test runs and skips (no fixtures found)

**Step 4: Commit**

```bash
git add crates/lintal_linter/tests/autofix_roundtrip.rs
git add crates/lintal_linter/tests/fixtures/autofix
git commit -m "feat: add autofix roundtrip test harness"
```

---

### Task 3: Create WhitespaceAround default fixture

**Files:**
- Create: `crates/lintal_linter/tests/fixtures/autofix/whitespace/whitespace_around/default/checkstyle.xml`
- Create: `crates/lintal_linter/tests/fixtures/autofix/whitespace/whitespace_around/default/Input.java`

**Step 1: Create directory structure**

```bash
mkdir -p crates/lintal_linter/tests/fixtures/autofix/whitespace/whitespace_around/default
```

**Step 2: Create checkstyle.xml**

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

**Step 3: Create Input.java**

```java
public class Input {
    public void method() {
        int x=1;
        int y =2;
        int z= 3;
        if(x==1){
            x=x+1;
        }
        while(y>0){
            y=y-1;
        }
        for(int i=0;i<10;i++){
            z=z+i;
        }
    }
}
```

**Step 4: Build lintal and run test**

Run: `cargo build && cargo test --package lintal_linter --test autofix_roundtrip -- --nocapture`
Expected: 1 fixture found, test passes

**Step 5: Commit**

```bash
git add crates/lintal_linter/tests/fixtures/autofix/whitespace/whitespace_around/
git commit -m "test: add WhitespaceAround default autofix fixture"
```

---

### Task 4: Create WhitespaceAround allow_empty_methods fixture

**Files:**
- Create: `crates/lintal_linter/tests/fixtures/autofix/whitespace/whitespace_around/allow_empty_methods/checkstyle.xml`
- Create: `crates/lintal_linter/tests/fixtures/autofix/whitespace/whitespace_around/allow_empty_methods/Input.java`

**Step 1: Create directory**

```bash
mkdir -p crates/lintal_linter/tests/fixtures/autofix/whitespace/whitespace_around/allow_empty_methods
```

**Step 2: Create checkstyle.xml**

```xml
<?xml version="1.0"?>
<!DOCTYPE module PUBLIC "-//Checkstyle//DTD Checkstyle Configuration 1.3//EN"
  "https://checkstyle.org/dtds/configuration_1_3.dtd">
<module name="Checker">
  <module name="TreeWalker">
    <module name="WhitespaceAround">
      <property name="allowEmptyMethods" value="true"/>
    </module>
  </module>
</module>
```

**Step 3: Create Input.java**

```java
public class Input {
    public void emptyMethod() {}
    public void nonEmptyMethod() {int x=1;}
}
```

**Step 4: Run test**

Run: `cargo test --package lintal_linter --test autofix_roundtrip -- --nocapture`
Expected: 2 fixtures found, tests pass

**Step 5: Commit**

```bash
git add crates/lintal_linter/tests/fixtures/autofix/whitespace/whitespace_around/allow_empty_methods/
git commit -m "test: add WhitespaceAround allowEmptyMethods autofix fixture"
```

---

### Task 5: Create WhitespaceAround allow_empty_types fixture

**Files:**
- Create: `crates/lintal_linter/tests/fixtures/autofix/whitespace/whitespace_around/allow_empty_types/checkstyle.xml`
- Create: `crates/lintal_linter/tests/fixtures/autofix/whitespace/whitespace_around/allow_empty_types/Input.java`

**Step 1: Create directory and files**

checkstyle.xml:
```xml
<?xml version="1.0"?>
<!DOCTYPE module PUBLIC "-//Checkstyle//DTD Checkstyle Configuration 1.3//EN"
  "https://checkstyle.org/dtds/configuration_1_3.dtd">
<module name="Checker">
  <module name="TreeWalker">
    <module name="WhitespaceAround">
      <property name="allowEmptyTypes" value="true"/>
    </module>
  </module>
</module>
```

Input.java:
```java
public class Input {}

class EmptyClass {}

interface EmptyInterface {}

class NonEmpty {int x=1;}
```

**Step 2: Run test and commit**

---

### Task 6: Create WhitespaceAround allow_empty_constructors fixture

checkstyle.xml:
```xml
<?xml version="1.0"?>
<!DOCTYPE module PUBLIC "-//Checkstyle//DTD Checkstyle Configuration 1.3//EN"
  "https://checkstyle.org/dtds/configuration_1_3.dtd">
<module name="Checker">
  <module name="TreeWalker">
    <module name="WhitespaceAround">
      <property name="allowEmptyConstructors" value="true"/>
    </module>
  </module>
</module>
```

Input.java:
```java
public class Input {
    public Input() {}
    public Input(int x) {this.x=x;}
    private int x;
}
```

---

### Task 7: Create WhitespaceAround allow_empty_lambdas fixture

checkstyle.xml:
```xml
<?xml version="1.0"?>
<!DOCTYPE module PUBLIC "-//Checkstyle//DTD Checkstyle Configuration 1.3//EN"
  "https://checkstyle.org/dtds/configuration_1_3.dtd">
<module name="Checker">
  <module name="TreeWalker">
    <module name="WhitespaceAround">
      <property name="allowEmptyLambdas" value="true"/>
    </module>
  </module>
</module>
```

Input.java:
```java
import java.util.function.Consumer;

public class Input {
    Consumer<String> empty = s -> {};
    Consumer<String> nonEmpty = s -> {System.out.println(s);};
}
```

---

### Task 8: Create WhitespaceAfter default fixture

**Directory:** `whitespace/whitespace_after/default/`

checkstyle.xml:
```xml
<?xml version="1.0"?>
<!DOCTYPE module PUBLIC "-//Checkstyle//DTD Checkstyle Configuration 1.3//EN"
  "https://checkstyle.org/dtds/configuration_1_3.dtd">
<module name="Checker">
  <module name="TreeWalker">
    <module name="WhitespaceAfter"/>
  </module>
</module>
```

Input.java:
```java
public class Input {
    public void method() {
        int x = 1,y = 2,z = 3;
        if(x > 0) {
            x++;
        }
        for(int i = 0;i < 10;i++) {
            y++;
        }
        int[] arr = new int[] {1,2,3};
    }
}
```

---

### Task 9: Create NoWhitespaceAfter default fixture

**Directory:** `whitespace/no_whitespace_after/default/`

checkstyle.xml:
```xml
<?xml version="1.0"?>
<!DOCTYPE module PUBLIC "-//Checkstyle//DTD Checkstyle Configuration 1.3//EN"
  "https://checkstyle.org/dtds/configuration_1_3.dtd">
<module name="Checker">
  <module name="TreeWalker">
    <module name="NoWhitespaceAfter"/>
  </module>
</module>
```

Input.java:
```java
public class Input {
    public void method() {
        int[] arr = new int[ 10];
        int x = arr[ 0];
        int y = - 5;
        int z = + 3;
        boolean b = ! true;
        x = ~ x;
        x++ ;
        y-- ;
    }
}
```

---

### Task 10: Create NoWhitespaceBefore default fixture

**Directory:** `whitespace/no_whitespace_before/default/`

checkstyle.xml:
```xml
<?xml version="1.0"?>
<!DOCTYPE module PUBLIC "-//Checkstyle//DTD Checkstyle Configuration 1.3//EN"
  "https://checkstyle.org/dtds/configuration_1_3.dtd">
<module name="Checker">
  <module name="TreeWalker">
    <module name="NoWhitespaceBefore"/>
  </module>
</module>
```

Input.java:
```java
public class Input {
    public void method() {
        int x = 1 ;
        int y = 2 , z = 3;
        x ++;
        y --;
    }
}
```

---

### Task 11: Create ParenPad nospace fixture

**Directory:** `whitespace/paren_pad/nospace/`

checkstyle.xml:
```xml
<?xml version="1.0"?>
<!DOCTYPE module PUBLIC "-//Checkstyle//DTD Checkstyle Configuration 1.3//EN"
  "https://checkstyle.org/dtds/configuration_1_3.dtd">
<module name="Checker">
  <module name="TreeWalker">
    <module name="ParenPad">
      <property name="option" value="nospace"/>
    </module>
  </module>
</module>
```

Input.java:
```java
public class Input {
    public void method( int x ) {
        if ( x > 0 ) {
            System.out.println( x );
        }
        for ( int i = 0; i < 10; i++ ) {
            x++;
        }
    }
}
```

---

### Task 12: Create ParenPad space fixture

**Directory:** `whitespace/paren_pad/space/`

checkstyle.xml:
```xml
<?xml version="1.0"?>
<!DOCTYPE module PUBLIC "-//Checkstyle//DTD Checkstyle Configuration 1.3//EN"
  "https://checkstyle.org/dtds/configuration_1_3.dtd">
<module name="Checker">
  <module name="TreeWalker">
    <module name="ParenPad">
      <property name="option" value="space"/>
    </module>
  </module>
</module>
```

Input.java:
```java
public class Input {
    public void method(int x) {
        if (x > 0) {
            System.out.println(x);
        }
        for (int i = 0; i < 10; i++) {
            x++;
        }
    }
}
```

---

### Task 13: Create MethodParamPad nospace fixture

**Directory:** `whitespace/method_param_pad/nospace/`

checkstyle.xml:
```xml
<?xml version="1.0"?>
<!DOCTYPE module PUBLIC "-//Checkstyle//DTD Checkstyle Configuration 1.3//EN"
  "https://checkstyle.org/dtds/configuration_1_3.dtd">
<module name="Checker">
  <module name="TreeWalker">
    <module name="MethodParamPad">
      <property name="option" value="nospace"/>
    </module>
  </module>
</module>
```

Input.java:
```java
public class Input {
    public void method (int x) {
        System.out.println (x);
    }

    public Input () {
    }
}
```

---

### Task 14: Create MethodParamPad space fixture

**Directory:** `whitespace/method_param_pad/space/`

checkstyle.xml:
```xml
<?xml version="1.0"?>
<!DOCTYPE module PUBLIC "-//Checkstyle//DTD Checkstyle Configuration 1.3//EN"
  "https://checkstyle.org/dtds/configuration_1_3.dtd">
<module name="Checker">
  <module name="TreeWalker">
    <module name="MethodParamPad">
      <property name="option" value="space"/>
    </module>
  </module>
</module>
```

Input.java:
```java
public class Input {
    public void method(int x) {
        System.out.println(x);
    }

    public Input() {
    }
}
```

---

### Task 15: Create TypecastParenPad nospace fixture

**Directory:** `whitespace/typecast_paren_pad/nospace/`

checkstyle.xml:
```xml
<?xml version="1.0"?>
<!DOCTYPE module PUBLIC "-//Checkstyle//DTD Checkstyle Configuration 1.3//EN"
  "https://checkstyle.org/dtds/configuration_1_3.dtd">
<module name="Checker">
  <module name="TreeWalker">
    <module name="TypecastParenPad">
      <property name="option" value="nospace"/>
    </module>
  </module>
</module>
```

Input.java:
```java
public class Input {
    public void method() {
        Object obj = "test";
        String s = ( String ) obj;
        int x = ( int ) 3.14;
    }
}
```

---

### Task 16: Create TypecastParenPad space fixture

**Directory:** `whitespace/typecast_paren_pad/space/`

checkstyle.xml:
```xml
<?xml version="1.0"?>
<!DOCTYPE module PUBLIC "-//Checkstyle//DTD Checkstyle Configuration 1.3//EN"
  "https://checkstyle.org/dtds/configuration_1_3.dtd">
<module name="Checker">
  <module name="TreeWalker">
    <module name="TypecastParenPad">
      <property name="option" value="space"/>
    </module>
  </module>
</module>
```

Input.java:
```java
public class Input {
    public void method() {
        Object obj = "test";
        String s = (String) obj;
        int x = (int) 3.14;
    }
}
```

---

### Task 17: Create EmptyForInitializerPad nospace fixture

**Directory:** `whitespace/empty_for_initializer_pad/nospace/`

checkstyle.xml:
```xml
<?xml version="1.0"?>
<!DOCTYPE module PUBLIC "-//Checkstyle//DTD Checkstyle Configuration 1.3//EN"
  "https://checkstyle.org/dtds/configuration_1_3.dtd">
<module name="Checker">
  <module name="TreeWalker">
    <module name="EmptyForInitializerPad">
      <property name="option" value="nospace"/>
    </module>
  </module>
</module>
```

Input.java:
```java
public class Input {
    public void method() {
        int i = 0;
        for ( ; i < 10; i++) {
            System.out.println(i);
        }
    }
}
```

---

### Task 18: Create EmptyForInitializerPad space fixture

**Directory:** `whitespace/empty_for_initializer_pad/space/`

checkstyle.xml:
```xml
<?xml version="1.0"?>
<!DOCTYPE module PUBLIC "-//Checkstyle//DTD Checkstyle Configuration 1.3//EN"
  "https://checkstyle.org/dtds/configuration_1_3.dtd">
<module name="Checker">
  <module name="TreeWalker">
    <module name="EmptyForInitializerPad">
      <property name="option" value="space"/>
    </module>
  </module>
</module>
```

Input.java:
```java
public class Input {
    public void method() {
        int i = 0;
        for (; i < 10; i++) {
            System.out.println(i);
        }
    }
}
```

---

### Task 19: Create SingleSpaceSeparator default fixture

**Directory:** `whitespace/single_space_separator/default/`

checkstyle.xml:
```xml
<?xml version="1.0"?>
<!DOCTYPE module PUBLIC "-//Checkstyle//DTD Checkstyle Configuration 1.3//EN"
  "https://checkstyle.org/dtds/configuration_1_3.dtd">
<module name="Checker">
  <module name="TreeWalker">
    <module name="SingleSpaceSeparator"/>
  </module>
</module>
```

Input.java:
```java
public class Input {
    private int    x;
    private int  y;

    public void method() {
        int  a  =  1;
        int   b   =   2;
    }
}
```

---

### Task 20: Create FileTabCharacter default fixture

**Directory:** `whitespace/file_tab_character/default/`

checkstyle.xml:
```xml
<?xml version="1.0"?>
<!DOCTYPE module PUBLIC "-//Checkstyle//DTD Checkstyle Configuration 1.3//EN"
  "https://checkstyle.org/dtds/configuration_1_3.dtd">
<module name="Checker">
  <module name="FileTabCharacter"/>
</module>
```

Input.java (NOTE: use actual tab characters, represented as \t here):
```java
public class Input {
	public void method() {
		int x = 1;
	}
}
```

---

### Task 21: Create ModifierOrder default fixture

**Directory:** `modifier/modifier_order/default/`

checkstyle.xml:
```xml
<?xml version="1.0"?>
<!DOCTYPE module PUBLIC "-//Checkstyle//DTD Checkstyle Configuration 1.3//EN"
  "https://checkstyle.org/dtds/configuration_1_3.dtd">
<module name="Checker">
  <module name="TreeWalker">
    <module name="ModifierOrder"/>
  </module>
</module>
```

Input.java:
```java
public class Input {
    final public static int X = 1;
    static public int Y = 2;
    synchronized public void method() {
        System.out.println(X);
    }
    final private int z = 3;
}
```

---

### Task 22: Create FinalLocalVariable default fixture

**Directory:** `modifier/final_local_variable/default/`

checkstyle.xml:
```xml
<?xml version="1.0"?>
<!DOCTYPE module PUBLIC "-//Checkstyle//DTD Checkstyle Configuration 1.3//EN"
  "https://checkstyle.org/dtds/configuration_1_3.dtd">
<module name="Checker">
  <module name="TreeWalker">
    <module name="FinalLocalVariable"/>
  </module>
</module>
```

Input.java:
```java
public class Input {
    public void method() {
        int x = 1;
        String s = "hello";
        System.out.println(x + s);
    }
}
```

---

### Task 23: Create FinalParameters default fixture

**Directory:** `modifier/final_parameters/default/`

checkstyle.xml:
```xml
<?xml version="1.0"?>
<!DOCTYPE module PUBLIC "-//Checkstyle//DTD Checkstyle Configuration 1.3//EN"
  "https://checkstyle.org/dtds/configuration_1_3.dtd">
<module name="Checker">
  <module name="TreeWalker">
    <module name="FinalParameters"/>
  </module>
</module>
```

Input.java:
```java
public class Input {
    public void method(int x, String s) {
        System.out.println(x + s);
    }

    public Input(int value) {
        System.out.println(value);
    }
}
```

---

### Task 24: Create RedundantModifier default fixture

**Directory:** `modifier/redundant_modifier/default/`

checkstyle.xml:
```xml
<?xml version="1.0"?>
<!DOCTYPE module PUBLIC "-//Checkstyle//DTD Checkstyle Configuration 1.3//EN"
  "https://checkstyle.org/dtds/configuration_1_3.dtd">
<module name="Checker">
  <module name="TreeWalker">
    <module name="RedundantModifier"/>
  </module>
</module>
```

Input.java:
```java
public interface Input {
    public void method();
    public abstract void abstractMethod();
    public static final int CONSTANT = 1;
}
```

---

### Task 25: Create UnusedImports default fixture

**Directory:** `imports/unused_imports/default/`

checkstyle.xml:
```xml
<?xml version="1.0"?>
<!DOCTYPE module PUBLIC "-//Checkstyle//DTD Checkstyle Configuration 1.3//EN"
  "https://checkstyle.org/dtds/configuration_1_3.dtd">
<module name="Checker">
  <module name="TreeWalker">
    <module name="UnusedImports"/>
  </module>
</module>
```

Input.java:
```java
import java.util.List;
import java.util.Map;
import java.util.ArrayList;
import java.util.HashMap;

public class Input {
    private List<String> items = new ArrayList<>();

    public void method() {
        items.add("test");
    }
}
```

---

### Task 26: Create RedundantImport default fixture

**Directory:** `imports/redundant_import/default/`

checkstyle.xml:
```xml
<?xml version="1.0"?>
<!DOCTYPE module PUBLIC "-//Checkstyle//DTD Checkstyle Configuration 1.3//EN"
  "https://checkstyle.org/dtds/configuration_1_3.dtd">
<module name="Checker">
  <module name="TreeWalker">
    <module name="RedundantImport"/>
  </module>
</module>
```

Input.java:
```java
import java.lang.String;
import java.lang.System;
import java.util.List;
import java.util.List;

public class Input {
    private List<String> items;

    public void method() {
        System.out.println("test");
    }
}
```

---

### Task 27: Create LeftCurly eol fixture

**Directory:** `blocks/left_curly/eol/`

checkstyle.xml:
```xml
<?xml version="1.0"?>
<!DOCTYPE module PUBLIC "-//Checkstyle//DTD Checkstyle Configuration 1.3//EN"
  "https://checkstyle.org/dtds/configuration_1_3.dtd">
<module name="Checker">
  <module name="TreeWalker">
    <module name="LeftCurly">
      <property name="option" value="eol"/>
    </module>
  </module>
</module>
```

Input.java:
```java
public class Input
{
    public void method()
    {
        if (true)
        {
            System.out.println("test");
        }
    }
}
```

---

### Task 28: Create LeftCurly nl fixture

**Directory:** `blocks/left_curly/nl/`

checkstyle.xml:
```xml
<?xml version="1.0"?>
<!DOCTYPE module PUBLIC "-//Checkstyle//DTD Checkstyle Configuration 1.3//EN"
  "https://checkstyle.org/dtds/configuration_1_3.dtd">
<module name="Checker">
  <module name="TreeWalker">
    <module name="LeftCurly">
      <property name="option" value="nl"/>
    </module>
  </module>
</module>
```

Input.java:
```java
public class Input {
    public void method() {
        if (true) {
            System.out.println("test");
        }
    }
}
```

---

### Task 29: Create LeftCurly nlow fixture

**Directory:** `blocks/left_curly/nlow/`

checkstyle.xml:
```xml
<?xml version="1.0"?>
<!DOCTYPE module PUBLIC "-//Checkstyle//DTD Checkstyle Configuration 1.3//EN"
  "https://checkstyle.org/dtds/configuration_1_3.dtd">
<module name="Checker">
  <module name="TreeWalker">
    <module name="LeftCurly">
      <property name="option" value="nlow"/>
    </module>
  </module>
</module>
```

Input.java:
```java
public class Input
{
    public void shortMethod() {
        System.out.println("short");
    }

    public void methodWithVeryLongNameThatExceedsTheLineLimit()
    {
        System.out.println("long");
    }
}
```

---

### Task 30: Create RightCurly same fixture

**Directory:** `blocks/right_curly/same/`

checkstyle.xml:
```xml
<?xml version="1.0"?>
<!DOCTYPE module PUBLIC "-//Checkstyle//DTD Checkstyle Configuration 1.3//EN"
  "https://checkstyle.org/dtds/configuration_1_3.dtd">
<module name="Checker">
  <module name="TreeWalker">
    <module name="RightCurly">
      <property name="option" value="same"/>
    </module>
  </module>
</module>
```

Input.java:
```java
public class Input {
    public void method() {
        if (true) {
            System.out.println("if");
        }
        else {
            System.out.println("else");
        }

        try {
            System.out.println("try");
        }
        catch (Exception e) {
            System.out.println("catch");
        }
        finally {
            System.out.println("finally");
        }
    }
}
```

---

### Task 31: Create RightCurly alone fixture

**Directory:** `blocks/right_curly/alone/`

checkstyle.xml:
```xml
<?xml version="1.0"?>
<!DOCTYPE module PUBLIC "-//Checkstyle//DTD Checkstyle Configuration 1.3//EN"
  "https://checkstyle.org/dtds/configuration_1_3.dtd">
<module name="Checker">
  <module name="TreeWalker">
    <module name="RightCurly">
      <property name="option" value="alone"/>
    </module>
  </module>
</module>
```

Input.java:
```java
public class Input {
    public void method() {
        if (true) {
            System.out.println("if");
        } else {
            System.out.println("else");
        }

        try {
            System.out.println("try");
        } catch (Exception e) {
            System.out.println("catch");
        } finally {
            System.out.println("finally");
        }
    }
}
```

---

### Task 32: Create NeedBraces default fixture

**Directory:** `blocks/need_braces/default/`

checkstyle.xml:
```xml
<?xml version="1.0"?>
<!DOCTYPE module PUBLIC "-//Checkstyle//DTD Checkstyle Configuration 1.3//EN"
  "https://checkstyle.org/dtds/configuration_1_3.dtd">
<module name="Checker">
  <module name="TreeWalker">
    <module name="NeedBraces"/>
  </module>
</module>
```

Input.java:
```java
public class Input {
    public void method() {
        int x = 1;

        if (x > 0)
            x++;

        while (x < 10)
            x++;

        for (int i = 0; i < 5; i++)
            x++;

        do
            x--;
        while (x > 0);
    }
}
```

---

### Task 33: Create OneStatementPerLine default fixture

**Directory:** `coding/one_statement_per_line/default/`

checkstyle.xml:
```xml
<?xml version="1.0"?>
<!DOCTYPE module PUBLIC "-//Checkstyle//DTD Checkstyle Configuration 1.3//EN"
  "https://checkstyle.org/dtds/configuration_1_3.dtd">
<module name="Checker">
  <module name="TreeWalker">
    <module name="OneStatementPerLine"/>
  </module>
</module>
```

Input.java:
```java
public class Input {
    public void method() {
        int x = 1; int y = 2; int z = 3;
        x++; y++; z++;
        System.out.println(x); System.out.println(y);
    }
}
```

---

### Task 34: Create MultipleVariableDeclarations default fixture

**Directory:** `coding/multiple_variable_declarations/default/`

checkstyle.xml:
```xml
<?xml version="1.0"?>
<!DOCTYPE module PUBLIC "-//Checkstyle//DTD Checkstyle Configuration 1.3//EN"
  "https://checkstyle.org/dtds/configuration_1_3.dtd">
<module name="Checker">
  <module name="TreeWalker">
    <module name="MultipleVariableDeclarations"/>
  </module>
</module>
```

Input.java:
```java
public class Input {
    private int a, b, c;

    public void method() {
        int x, y, z;
        String s1 = "a", s2 = "b";
    }
}
```

---

### Task 35: Create UpperEll default fixture

**Directory:** `style/upper_ell/default/`

checkstyle.xml:
```xml
<?xml version="1.0"?>
<!DOCTYPE module PUBLIC "-//Checkstyle//DTD Checkstyle Configuration 1.3//EN"
  "https://checkstyle.org/dtds/configuration_1_3.dtd">
<module name="Checker">
  <module name="TreeWalker">
    <module name="UpperEll"/>
  </module>
</module>
```

Input.java:
```java
public class Input {
    private long a = 1l;
    private long b = 2l;
    private long c = 0x1Al;

    public void method() {
        long x = 100l;
        long y = 0xFFl;
    }
}
```

---

### Task 36: Create ArrayTypeStyle java fixture

**Directory:** `style/array_type_style/java/`

checkstyle.xml:
```xml
<?xml version="1.0"?>
<!DOCTYPE module PUBLIC "-//Checkstyle//DTD Checkstyle Configuration 1.3//EN"
  "https://checkstyle.org/dtds/configuration_1_3.dtd">
<module name="Checker">
  <module name="TreeWalker">
    <module name="ArrayTypeStyle">
      <property name="javaStyle" value="true"/>
    </module>
  </module>
</module>
```

Input.java:
```java
public class Input {
    private int arr[];
    private String strs[];

    public void method(int args[]) {
        int local[];
    }
}
```

---

### Task 37: Create ArrayTypeStyle c fixture

**Directory:** `style/array_type_style/c/`

checkstyle.xml:
```xml
<?xml version="1.0"?>
<!DOCTYPE module PUBLIC "-//Checkstyle//DTD Checkstyle Configuration 1.3//EN"
  "https://checkstyle.org/dtds/configuration_1_3.dtd">
<module name="Checker">
  <module name="TreeWalker">
    <module name="ArrayTypeStyle">
      <property name="javaStyle" value="false"/>
    </module>
  </module>
</module>
```

Input.java:
```java
public class Input {
    private int[] arr;
    private String[] strs;

    public void method(int[] args) {
        int[] local;
    }
}
```

---

### Task 38: Create Indentation default fixture

**Directory:** `whitespace/indentation/default/`

checkstyle.xml:
```xml
<?xml version="1.0"?>
<!DOCTYPE module PUBLIC "-//Checkstyle//DTD Checkstyle Configuration 1.3//EN"
  "https://checkstyle.org/dtds/configuration_1_3.dtd">
<module name="Checker">
  <module name="TreeWalker">
    <module name="Indentation">
      <property name="basicOffset" value="4"/>
      <property name="caseIndent" value="4"/>
    </module>
  </module>
</module>
```

Input.java:
```java
public class Input {
  public void method() {
      int x = 1;
    if (x > 0) {
          x++;
    }
  }
}
```

---

### Task 39: Update CI workflow for Java 25

**Files:**
- Modify: `.github/workflows/ci.yml`

**Step 1: Add Java setup to build job**

Add after `actions/checkout@v4`:
```yaml
      - name: Set up Java 25
        uses: actions/setup-java@v4
        with:
          distribution: 'temurin'
          java-version: '25-ea'
```

**Step 2: Add autofix roundtrip test to workflow**

Add a new job section:
```yaml
  autofix-tests:
    name: Autofix Roundtrip Tests
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Set up Java 25
        uses: actions/setup-java@v4
        with:
          distribution: 'temurin'
          java-version: '25-ea'

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Cache cargo registry
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
          restore-keys: |
            ${{ runner.os }}-cargo-

      - name: Build lintal
        run: cargo build

      - name: Run autofix roundtrip tests
        run: cargo test --package lintal_linter --test autofix_roundtrip -- --nocapture
```

**Step 3: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add Java 25 and autofix roundtrip tests"
```

---

### Task 40: Run full test suite and verify

**Step 1: Build lintal**

Run: `cargo build`

**Step 2: Run all autofix tests**

Run: `cargo test --package lintal_linter --test autofix_roundtrip -- --nocapture`
Expected: All ~38 fixtures pass

**Step 3: Run full test suite**

Run: `cargo test --all`
Expected: All tests pass

**Step 4: Final commit**

```bash
git add -A
git commit -m "test: complete autofix roundtrip test suite with 38 fixtures"
```

---

## Summary

| Phase | Tasks | Description |
|-------|-------|-------------|
| Setup | 1-2 | Dependencies and test harness |
| Whitespace | 3-20 | 18 fixtures for whitespace rules |
| Modifier | 21-24 | 4 fixtures for modifier rules |
| Imports | 25-26 | 2 fixtures for import rules |
| Blocks | 27-32 | 6 fixtures for block rules |
| Coding | 33-34 | 2 fixtures for coding rules |
| Style | 35-37 | 3 fixtures for style rules |
| Indentation | 38 | 1 fixture for indentation |
| CI | 39 | GitHub Actions integration |
| Verify | 40 | Final verification |

**Total: 40 tasks, ~38 fixtures**
