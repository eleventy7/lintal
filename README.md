# jruff

A fast Java linter with auto-fix support, built in Rust.

jruff reads checkstyle.xml configuration files and can automatically fix many violations that checkstyle can only detect.

> **Attribution**: jruff is built on infrastructure from [Ruff](https://github.com/astral-sh/ruff), the extremely fast Python linter by [Astral](https://astral.sh/). We're grateful to the Ruff team for their excellent work that made this project possible.

## Features

- Reads existing checkstyle.xml configurations
- Auto-fixes many common violations (whitespace, braces, imports, modifiers)
- Fast parallel processing
- Optional TOML overlay for fix-specific settings

## Installation

```bash
cargo install --path crates/jruff
```

## Usage

```bash
# Check files for violations
jruff check src/

# Fix violations
jruff fix src/

# Use specific checkstyle config
jruff check src/ --config path/to/checkstyle.xml

# Show fixes without applying
jruff fix src/ --diff
```

## Supported Rules

### Auto-fixable (Safe)

- WhitespaceAround
- WhitespaceAfter
- NoWhitespaceBefore
- NoWhitespaceAfter
- SingleSpaceSeparator
- ParenPad
- TypecastParenPad
- LeftCurly
- RightCurly
- NeedBraces
- ModifierOrder
- RedundantModifier
- FinalParameters
- FinalLocalVariable
- UpperEll
- ArrayTypeStyle

### Check-only

All other checkstyle rules are supported for detection but require manual fixes.

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
