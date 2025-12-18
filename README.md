# lintal

A fast Java linter with auto-fix support, built in Rust.

lintal reads checkstyle.xml configuration files and can automatically fix many violations that checkstyle can only detect.

> **Attribution**: lintal is built on infrastructure from [Ruff](https://github.com/astral-sh/ruff), the extremely fast Python linter by [Astral](https://astral.sh/). We're grateful to the Ruff team for their excellent work that made this project possible.

## Current Status

The project is focused on safely autofixable cases to begin with, and aims for 100% compatibility with Checkstyle. Checkstyle test cases are downloaded during the testing build phase to validate compatibility.

## Features

- Reads existing checkstyle.xml configurations
- Auto-fixes many common violations (whitespace, brace placement, imports, modifiers)
- Fast parallel processing
- Suppression support:
  - `@SuppressWarnings("checkstyle:RuleName")` annotations
  - `SuppressWithPlainTextCommentFilter` (`// CHECKSTYLE:OFF:RuleName` comments)
  - `SuppressWarningsFilter`
- Optional TOML overlay for fix-specific settings

## Installation

```bash
cargo install --path crates/lintal
```

## Usage

```bash
# Check files for violations
lintal check src/

# Fix violations
lintal fix src/

# Use specific checkstyle config
lintal check src/ --config path/to/checkstyle.xml

# Show fixes without applying
lintal fix src/ --diff
```

## Supported Rules

lintal currently implements 16 checkstyle rules with 100% compatibility against checkstyle's own test suite.

### Whitespace (10 rules)

| Rule | Auto-fix | Status |
|------|----------|--------|
| WhitespaceAround | ✅ | 100% compatible |
| WhitespaceAfter | ✅ | 100% compatible |
| NoWhitespaceAfter | ✅ | 100% compatible |
| NoWhitespaceBefore | ✅ | 100% compatible |
| SingleSpaceSeparator | ✅ | 100% compatible |
| ParenPad | ✅ | 100% compatible |
| TypecastParenPad | ✅ | 100% compatible |
| MethodParamPad | ✅ | 100% compatible |
| EmptyForInitializerPad | ✅ | 100% compatible |
| FileTabCharacter | ✅ | 100% compatible |

### Blocks (6 rules)

| Rule | Auto-fix | Status |
|------|----------|--------|
| LeftCurly | ✅ (partial) | 100% compatible |
| RightCurly | ✅ (partial) | 100% compatible |
| NeedBraces | ❌ | 100% compatible |
| EmptyBlock | ❌ | 100% compatible |
| EmptyCatchBlock | ❌ | 100% compatible |
| AvoidNestedBlocks | ❌ | 100% compatible |

## Development

```bash
# Build
cargo build

# Run tests
cargo test

# Run on a Java project
cargo run -- check /path/to/java/src
```

## Acknowledgments

This project builds on the excellent work of:

- [Ruff](https://github.com/astral-sh/ruff) - The core infrastructure (diagnostics, text handling, fix application) is derived from Ruff's codebase
- [tree-sitter-java](https://github.com/tree-sitter/tree-sitter-java) - Java parsing via tree-sitter
- [Checkstyle](https://checkstyle.org/) - The original Java style checker whose configurations we support

## License

MIT - See [LICENSE](LICENSE) and [NOTICE](NOTICE) for details.
