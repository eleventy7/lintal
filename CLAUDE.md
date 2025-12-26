# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with this repository.

## Project Overview

lintal is a fast Java linter with auto-fix support, built in Rust. It reads checkstyle.xml configuration files and can automatically fix many violations that checkstyle can only detect.

## Build Commands

```bash
# Build
cargo build
cargo build --release

# Run tests
cargo test --all

# Run specific test suite (checkstyle compatibility tests)
cargo test --package lintal_linter --test checkstyle_finallocalvariable

# Lint and format
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings

# Run on a Java project
./target/release/lintal check /path/to/java/src
./target/release/lintal fix /path/to/java/src
```

## Architecture

```
crates/
├── lintal/                  # CLI entry point
├── lintal_java_parser/      # Tree-sitter Java wrapper
├── lintal_java_cst/         # CST node types + traversal
├── lintal_checkstyle/       # checkstyle.xml parser
├── lintal_linter/           # Rule implementations
│   └── rules/
│       ├── whitespace/      # WhitespaceAround, ParenPad, etc.
│       ├── blocks/          # LeftCurly, RightCurly, NeedBraces
│       ├── modifier/        # ModifierOrder, FinalParameters, etc.
│       └── style/           # UpperEll, ArrayTypeStyle
├── lintal_diagnostics/      # Fix, Edit, Applicability (from Ruff)
├── lintal_text_size/        # TextRange, offsets (from Ruff)
└── lintal_source_file/      # Line indexing (from Ruff)
```

## Testing

- Checkstyle test fixtures are cloned to `target/checkstyle-tests/` during test runs
- Real-world validation against: artio, aeron, agrona (cloned to `target/`)
- All rules aim for 100% compatibility with checkstyle's own test suite

## Key Patterns

### Adding a New Rule

1. Create rule file in appropriate `crates/lintal_linter/src/rules/<category>/`
2. Implement `Rule` trait with `name()` and `check()` methods
3. Register in `crates/lintal_linter/src/registry.rs`
4. Add checkstyle compatibility tests in `crates/lintal_linter/tests/`

### Rule Implementation

```rust
impl Rule for MyRule {
    fn name(&self) -> &'static str {
        "MyRule"  // Must match checkstyle module name
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        // Return diagnostics with optional Fix for auto-fix support
    }
}
```

### Suppression Support

- `@SuppressWarnings("RuleName")` or `@SuppressWarnings("checkstyle:RuleName")`
- `// CHECKSTYLE:OFF:RuleName` comments
- File-based suppressions via `suppressions.xml`

## Config Discovery

lintal auto-discovers config from:
1. `--config` flag
2. `config/checkstyle/checkstyle.xml` relative to target directory
3. `checkstyle.xml` in current directory

## Development Tools

### Dump Java AST

To inspect the tree-sitter AST for a Java file (useful when implementing rules):

```bash
# Build the tool
cargo build --bin dump_java_ast

# Pipe a Java file to see its AST
cat MyClass.java | ./target/debug/dump_java_ast

# Or use stdin redirection
./target/debug/dump_java_ast < MyClass.java

# Example output:
# program [1:0-6:0]
#   class_declaration [1:0-5:1]
#     class [1:0-1:5] "class"
#     identifier [1:6-1:9] "Foo"
#     class_body [1:10-5:1]
#       ...
```

Output format: `node_kind [start_line:start_col-end_line:end_col] "text preview"`
