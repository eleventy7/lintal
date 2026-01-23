# Implementing New Checkstyle Checks in lintal

This guide covers the process for implementing new checkstyle-compatible rules in lintal.

## Overview

Each check follows a consistent pattern:
1. Create the rule implementation
2. Register the rule
3. Add checkstyle compatibility tests
4. Add auto-fix roundtrip tests (if applicable)
5. Validate against real-world codebases

## Step 1: Understand the Checkstyle Rule

Before implementing, thoroughly understand the checkstyle rule:

1. **Read the checkstyle documentation**: https://checkstyle.sourceforge.io/checks/
2. **Examine test fixtures**: Located in `target/checkstyle-tests/src/test/resources/com/puppycrawl/tools/checkstyle/checks/<category>/<rulename>/`
3. **Parse violation comments**: Test files contain expected violations as comments like:
   - `// violation ''operator' should be on a new line.'`
   - `// violation below, '...'`
4. **Understand config options**: Note all configurable properties and their defaults

## Step 2: Explore Tree-sitter AST Structure

Use `dump_java_ast` to understand how Java constructs are represented:

```bash
# Build the tool
cargo build --bin dump_java_ast

# Examine AST for specific constructs
echo 'class T { void m() { int x = 1 + 2; } }' | ./target/debug/dump_java_ast
```

Key things to identify:
- What node kinds contain the constructs you need to check
- How to find operators, keywords, or tokens within those nodes
- How to determine line/column positions

## Step 3: Create the Rule Implementation

### File Location

Create the rule in the appropriate category:
- `crates/lintal_linter/src/rules/whitespace/` - Whitespace rules
- `crates/lintal_linter/src/rules/blocks/` - Block structure rules
- `crates/lintal_linter/src/rules/modifier/` - Modifier rules
- `crates/lintal_linter/src/rules/style/` - Code style rules
- `crates/lintal_linter/src/rules/coding/` - Coding practice rules

### Rule Structure

```rust
//! RuleName rule implementation.
//!
//! <Brief description of what the rule checks>
//!
//! Checkstyle equivalent: RuleNameCheck

use lintal_diagnostics::{Diagnostic, Edit, Fix, FixAvailability, Violation};
use lintal_java_cst::CstNode;
use lintal_text_size::{TextRange, TextSize};
use std::collections::HashSet;

use crate::{CheckContext, FromConfig, Properties, Rule};

// 1. Define violation type(s)
#[derive(Debug, Clone)]
pub struct MyViolation {
    pub detail: String,
}

impl Violation for MyViolation {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Sometimes;

    fn message(&self) -> String {
        format!("Violation message: {}", self.detail)
    }
}

// 2. Define configuration enums/structs if needed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MyOption {
    #[default]
    OptionA,
    OptionB,
}

// 3. Define the rule struct
#[derive(Debug, Clone)]
pub struct MyRule {
    pub option: MyOption,
    pub tokens: HashSet<MyToken>,
}

impl Default for MyRule {
    fn default() -> Self {
        Self {
            option: MyOption::default(),
            tokens: MyToken::default_tokens(),
        }
    }
}

// 4. Implement FromConfig to parse checkstyle properties
impl FromConfig for MyRule {
    const MODULE_NAME: &'static str = "MyRule";

    fn from_config(properties: &Properties) -> Self {
        let option = properties
            .get("option")
            .map(|v| match *v {
                "optionB" => MyOption::OptionB,
                _ => MyOption::OptionA,
            })
            .unwrap_or_default();

        Self { option, ..Default::default() }
    }
}

// 5. Define relevant node kinds
const RELEVANT_KINDS: &[&str] = &[
    "binary_expression",
    "assignment_expression",
    // Add all node kinds this rule cares about
];

// 6. Implement the Rule trait
impl Rule for MyRule {
    fn name(&self) -> &'static str {
        "MyRule"  // Must match checkstyle module name
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        match node.kind() {
            "binary_expression" => self.check_binary(ctx, node),
            "assignment_expression" => self.check_assignment(ctx, node),
            _ => vec![],
        }
    }
}

// 7. Implement check methods
impl MyRule {
    fn check_binary(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let ts_node = node.inner();
        let source = ctx.source();
        let source_code = ctx.source_code();

        // Get children, filter extras (comments)
        let mut cursor = ts_node.walk();
        let children: Vec<_> = ts_node
            .children(&mut cursor)
            .filter(|n| !n.is_extra())
            .collect();

        // Analyze and return diagnostics
        vec![]
    }
}

// 8. Add unit tests
#[cfg(test)]
mod tests {
    use super::*;
    use lintal_java_cst::TreeWalker;
    use lintal_java_parser::JavaParser;

    fn check_source(source: &str) -> Vec<Diagnostic> {
        let mut parser = JavaParser::new();
        let result = parser.parse(source).unwrap();
        let ctx = CheckContext::new(source);
        let rule = MyRule::default();

        let mut diagnostics = vec![];
        for node in TreeWalker::new(result.tree.root_node(), source) {
            diagnostics.extend(rule.check(&ctx, &node));
        }
        diagnostics
    }

    #[test]
    fn test_basic_violation() {
        let source = r#"class Test { void m() { /* violation case */ } }"#;
        let diagnostics = check_source(source);
        assert!(!diagnostics.is_empty());
    }
}
```

## Step 4: Register the Rule

### Export from module

In `crates/lintal_linter/src/rules/<category>/mod.rs`:
```rust
pub mod my_rule;
pub use my_rule::MyRule;
```

### Register in registry

In `crates/lintal_linter/src/registry.rs`, add to `register_builtins()`:
```rust
self.register::<MyRule>();
```

## Step 5: Create Checkstyle Compatibility Tests

Create `crates/lintal_linter/tests/checkstyle_myrule.rs`:

```rust
//! MyRule checkstyle compatibility tests.

mod checkstyle_repo;

use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::rules::MyRule;
use lintal_linter::{CheckContext, Rule};
use regex::Regex;
use std::collections::HashSet;

// Expected violation from test file comments
#[derive(Debug, Clone, PartialEq, Eq)]
struct ExpectedViolation {
    line: usize,
    // Add other fields as needed
}

// Actual violation from our implementation
#[derive(Debug, Clone)]
struct ActualViolation {
    line: usize,
    column: usize,
    message: String,
}

// Parse expected violations from checkstyle test file comments
fn parse_expected_violations(source: &str) -> Vec<ExpectedViolation> {
    let mut violations = vec![];
    let inline_re = Regex::new(r"//\s*violation\s+'[^']*'").unwrap();
    let below_re = Regex::new(r"//\s*violation\s+below").unwrap();

    for (i, line) in source.lines().enumerate() {
        let line_num = i + 1;
        if below_re.is_match(line) {
            violations.push(ExpectedViolation { line: line_num + 1 });
        } else if inline_re.is_match(line) {
            violations.push(ExpectedViolation { line: line_num });
        }
    }
    violations
}

fn load_fixture(file_name: &str) -> Option<String> {
    let path = checkstyle_repo::<category>_test_input("myrule", file_name)?;
    std::fs::read_to_string(&path).ok()
}

#[test]
fn test_comprehensive() {
    let Some(source) = load_fixture("InputMyRule.java") else {
        eprintln!("Skipping: checkstyle repo not available");
        return;
    };

    let expected = parse_expected_violations(&source);
    // Run rule and compare...
}
```

## Step 6: Implement Auto-Fix (If Applicable)

When adding auto-fix support:

```rust
fn create_fix(
    &self,
    ctx: &CheckContext,
    // ... parameters
) -> Option<Fix> {
    let source = ctx.source();

    // Check for conditions that make fix unsafe (comments, etc.)
    if unsafe_to_fix {
        return None;
    }

    // Create edits
    let deletion = Edit::deletion(start, end);
    let insertion = Edit::insertion(new_text, position);

    Some(Fix::unsafe_edits(deletion, vec![insertion]))
}
```

Use `Fix::unsafe_edits` for fixes that may change behavior, `Fix::safe_edits` for guaranteed-safe fixes.

## Step 7: Create Auto-Fix Roundtrip Tests

If your rule includes auto-fix support, add roundtrip tests in `crates/lintal_linter/tests/fixtures/autofix/<category>/<rule_name>/`.

### Fixture Structure

Each test variant needs three files:
```
tests/fixtures/autofix/whitespace/my_rule/
├── default/
│   ├── checkstyle.xml    # Rule configuration
│   ├── Input.java        # File with violations
│   └── Expected.java     # Correctly fixed output
└── option_b/
    ├── checkstyle.xml
    ├── Input.java
    └── Expected.java
```

### checkstyle.xml
```xml
<?xml version="1.0"?>
<!DOCTYPE module PUBLIC "-//Checkstyle//DTD Checkstyle Configuration 1.3//EN"
  "https://checkstyle.org/dtds/configuration_1_3.dtd">
<module name="Checker">
  <module name="TreeWalker">
    <module name="MyRule">
      <property name="option" value="optionB"/>
    </module>
  </module>
</module>
```

### Input.java
```java
public class Input {
    // Code with violations that will be fixed
    int x = 1 +
        2;  // Violation that fix will correct
}
```

### Expected.java
```java
public class Input {
    // Code after fix is applied
    int x = 1
        + 2;  // Now correct
}
```

### Test Behavior

The `autofix_roundtrip.rs` test automatically:
1. Compiles the original `Input.java` (must succeed)
2. Runs `lintal fix` with the specified config
3. Compiles the fixed file (must succeed)
4. Runs `lintal check` (must have zero violations)
5. Compares output with `Expected.java` (byte-level match)

### Important Notes

- `Input.java` and `Expected.java` must differ (otherwise the test is useless)
- Only test operators/tokens that are in the configured tokens list
- Use `Fix::safe_edit` for fixes that don't need `--unsafe` flag to apply
- The test uses the release binary if present, otherwise debug

## Step 8: Validate Against Real-World Codebases

Run against aeron/agrona/artio to check for false positives:

```bash
# Build release
cargo build --release

# Check each repo (should have 0 violations if checkstyle-compliant)
./target/release/lintal check target/agrona 2>&1 | grep -c "MyRule"
./target/release/lintal check target/artio 2>&1 | grep -c "MyRule"
./target/release/lintal check target/aeron 2>&1 | grep -c "MyRule"
```

## Step 9: Run CI Checks

Before submitting:

```bash
# Format
cargo fmt --all

# Clippy
cargo clippy --all-targets --all-features -- -D warnings

# Tests
cargo test --all
```

## Common Patterns

### Getting Line/Column Info

```rust
let source_code = ctx.source_code();
let pos = TextSize::from(node.start_byte() as u32);
let loc = source_code.line_column(pos);
let line = loc.line.get();
let column = loc.column.get();
```

### Iterating Node Children

```rust
let ts_node = node.inner();
let mut cursor = ts_node.walk();
let children: Vec<_> = ts_node
    .children(&mut cursor)
    .filter(|n| !n.is_extra())  // Filter out comments
    .collect();
```

### Getting Node Text

```rust
let text = node.utf8_text(source.as_bytes()).unwrap_or("");
```

### Creating Diagnostics

```rust
let range = TextRange::new(
    TextSize::from(start as u32),
    TextSize::from(end as u32),
);

let diagnostic = Diagnostic::new(MyViolation { detail: "...".to_string() }, range);
// Optionally add fix:
let diagnostic = diagnostic.with_fix(fix);
```

## Checklist

- [ ] Rule struct with configuration
- [ ] `FromConfig` implementation
- [ ] `Rule` trait implementation
- [ ] Unit tests in the rule module
- [ ] Exported from category `mod.rs`
- [ ] Registered in `registry.rs`
- [ ] Checkstyle compatibility tests
- [ ] Auto-fix (if applicable)
- [ ] Auto-fix roundtrip tests (if applicable)
- [ ] Zero false positives on aeron/agrona/artio
- [ ] Passes `cargo fmt`, `cargo clippy`, `cargo test`
