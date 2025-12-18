# jruff: Java Linter with Auto-Fix

A Rust-based Java linter that reads checkstyle.xml configurations and fixes violations automatically.

## Goals

1. Parse existing checkstyle.xml configs directly
2. Auto-fix violations that checkstyle can only detect
3. Deliver value fast by focusing on fixable rules first

## Architecture

Fork Ruff, replace Python-specific crates with Java equivalents, keep the infrastructure.

```
jruff/
├── crates/
│   ├── jruff_java_parser/     # Tree-sitter Java wrapper
│   ├── jruff_java_cst/        # CST node types + traversal
│   ├── jruff_checkstyle/      # checkstyle.xml parser
│   ├── jruff_linter/          # Rule implementations
│   │   └── rules/
│   │       ├── whitespace/    # WhitespaceAround, NoWhitespaceBefore, etc.
│   │       ├── imports/       # UnusedImports, RedundantImport
│   │       ├── braces/        # LeftCurly, RightCurly, NeedBraces
│   │       ├── modifiers/     # ModifierOrder, RedundantModifier, Final*
│   │       └── style/         # UpperEll, ArrayTypeStyle, etc.
│   ├── jruff_diagnostics/     # Fix, Edit, Applicability (from Ruff)
│   ├── jruff_text_size/       # TextRange, offsets (from Ruff)
│   ├── jruff_source_file/     # Locator, line indexing (from Ruff)
│   └── jruff/                 # CLI entry point
└── tests/
    └── fixtures/              # Java files with known violations
```

**Data flow:**

```
.java files → tree-sitter parser → CST
    → rule checker (informed by checkstyle.xml)
    → diagnostics (with Fix objects)
    → fix applicator → fixed source
```

## Configuration

Two config sources, merged at runtime.

### checkstyle.xml (source of truth)

Parsed directly from existing files. Module names map to jruff rules:

```rust
"WhitespaceAround" → Rule::WhitespaceAround { allow_empty_lambdas: bool }
"LeftCurly"        → Rule::LeftCurly { option: NewLine | EndOfLine }
"UnusedImports"    → Rule::UnusedImports
```

Unknown modules log warnings and degrade gracefully.

### jruff.toml (optional overlay)

Controls fix behavior that checkstyle.xml cannot express:

```toml
[fix]
unsafe = false  # Don't apply unsafe fixes without --unsafe flag

[fix.rules]
WhitespaceAround = "fix"      # Auto-fix
LeftCurly = "check"           # Check only, don't fix
UnusedImports = "suggest"     # Show fix, require confirmation
MethodLength = "disabled"     # Skip entirely

[checkstyle]
config = "config/checkstyle/checkstyle.xml"
```

**Merge order:** checkstyle.xml defines *what* rules run and their parameters. jruff.toml defines *how* violations are handled.

## Parsing Layer

### jruff_java_parser

Wraps tree-sitter-java:

```rust
pub struct JavaParser {
    parser: tree_sitter::Parser,
    language: tree_sitter::Language,
}

impl JavaParser {
    pub fn parse(&mut self, source: &str) -> ParseResult {
        let tree = self.parser.parse(source, None)?;
        ParseResult { tree, source: source.into() }
    }
}
```

### jruff_java_cst

Typed traversal over raw tree-sitter nodes:

```rust
pub enum CstNode<'a> {
    ClassDeclaration(ClassDeclaration<'a>),
    MethodDeclaration(MethodDeclaration<'a>),
    Block(Block<'a>),
    IfStatement(IfStatement<'a>),
    BinaryExpression(BinaryExpression<'a>),
    // ...
}

pub struct IfStatement<'a> {
    node: tree_sitter::Node<'a>,
    source: &'a str,
}

impl<'a> IfStatement<'a> {
    pub fn condition(&self) -> Expression<'a> { ... }
    pub fn consequence(&self) -> Statement<'a> { ... }
    pub fn alternative(&self) -> Option<Statement<'a>> { ... }
    pub fn open_brace(&self) -> Option<Token<'a>> { ... }
    pub fn range(&self) -> TextRange { ... }
}
```

Wrapping tree-sitter nodes (rather than converting to a separate AST) preserves exact source positions needed for fixes.

## Rule Implementation

Each rule detects violations and optionally emits fixes:

```rust
pub trait Rule: Send + Sync {
    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic>;
}

pub enum FixAvailability {
    Always,
    Sometimes,
    Never,
}
```

### Example: WhitespaceAround

```rust
pub struct WhitespaceAround {
    pub allow_empty_lambdas: bool,
}

impl Rule for WhitespaceAround {
    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let CstNode::BinaryExpression(expr) = node else { return vec![] };

        let operator = expr.operator();
        let before = ctx.text_before(operator.range().start());
        let after = ctx.text_after(operator.range().end());

        let mut diagnostics = vec![];

        if !before.ends_with(' ') {
            diagnostics.push(
                Diagnostic::new(MissingWhitespaceBefore, operator.range())
                    .with_fix(Fix::safe_edit(
                        Edit::insertion(" ".into(), operator.range().start())
                    ))
            );
        }

        if !after.starts_with(' ') && !after.starts_with('\n') {
            diagnostics.push(
                Diagnostic::new(MissingWhitespaceAfter, operator.range())
                    .with_fix(Fix::safe_edit(
                        Edit::insertion(" ".into(), operator.range().end())
                    ))
            );
        }

        diagnostics
    }
}
```

### Example: LeftCurly (option="nl")

```rust
pub struct LeftCurly {
    pub option: BraceStyle,
}

impl Rule for LeftCurly {
    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let open_brace = match node {
            CstNode::ClassDeclaration(c) => c.open_brace(),
            CstNode::MethodDeclaration(m) => m.body()?.open_brace(),
            CstNode::IfStatement(i) => i.open_brace(),
            _ => return vec![],
        };

        let Some(brace) = open_brace else { return vec![] };

        if self.option == BraceStyle::NewLine {
            let text_before = ctx.text_between_prev_token_and(brace.range().start());
            if !text_before.contains('\n') {
                return vec![
                    Diagnostic::new(LeftCurlyMustBeOnNewLine, brace.range())
                        .with_fix(Fix::safe_edit(
                            Edit::replacement(
                                format!("\n{}", ctx.indent_at(brace)),
                                ctx.whitespace_range_before(brace)
                            )
                        ))
                ];
            }
        }

        vec![]
    }
}
```

## Rule Tiers

### Tier 1: Auto-fixable (Safe) — 20 rules

| Rule | Fix Strategy |
|------|--------------|
| FileTabCharacter | Replace tabs with spaces |
| Indentation | Adjust leading whitespace |
| WhitespaceAround | Insert/remove spaces around operators |
| WhitespaceAfter | Insert space after comma/semi/keywords |
| NoWhitespaceBefore | Remove space before token |
| NoWhitespaceAfter | Remove space after token |
| SingleSpaceSeparator | Collapse multiple spaces to one |
| EmptyForInitializerPad | Adjust spacing in `for(;` |
| MethodParamPad | Adjust spacing before `(` |
| ParenPad | Adjust spacing inside `()` |
| TypecastParenPad | Adjust spacing in casts |
| LeftCurly | Move `{` to new line |
| RightCurly | Move `}` to own line |
| NeedBraces | Wrap single statements in `{}` |
| ModifierOrder | Reorder modifiers |
| RedundantModifier | Remove redundant modifiers |
| FinalParameters | Add `final` to parameters |
| FinalLocalVariable | Add `final` to local variables |
| UpperEll | Change `1l` to `1L` |
| ArrayTypeStyle | Change `String args[]` to `String[] args` |

### Tier 2: Auto-fixable (Sometimes/Unsafe) — 8 rules

| Rule | Fix Strategy | Notes |
|------|--------------|-------|
| RedundantImport | Remove duplicate import | Safe |
| UnusedImports | Remove import | Needs usage analysis |
| EmptyLineSeparator | Insert/remove blank lines | Context-dependent |
| OperatorWrap | Move operator to end of line | May affect readability |
| MultipleVariableDeclarations | Split declarations | Safe but verbose |
| OneStatementPerLine | Insert line breaks | Safe |
| SimplifyBooleanReturn | Simplify return patterns | Semantic change |
| DeclarationOrder | Reorder class members | Large diff |

### Tier 3: Check-only — 22 rules

Detection only (naming, complexity, semantic issues):

ConstantName, LocalVariableName, MemberName, MethodName, PackageName, ParameterName, StaticVariableName, TypeName, MethodLength, NestedTryDepth, CovariantEquals, HiddenField, InnerAssignment, MissingSwitchDefault, EqualsHashCode, DefaultComesLast, StringLiteralEquality, FallThrough, FinalClass, HideUtilityClassConstructor, MutableException, IllegalType, EmptyStatement, EmptyBlock, EmptyCatchBlock, AvoidNestedBlocks, JavadocMethod, TodoComment, PackageDeclaration, DescendantToken

## CLI Interface

```bash
# Check files
jruff check src/
jruff check src/main/java/com/example/

# Fix files
jruff fix src/
jruff fix src/ --unsafe      # Include unsafe fixes
jruff fix src/ --diff        # Preview without applying

# Filter rules
jruff check src/ --select WhitespaceAround,LeftCurly
jruff check src/ --ignore FinalParameters

# Config
jruff check src/ --config path/to/checkstyle.xml

# Output formats
jruff check src/ --output-format text
jruff check src/ --output-format json
jruff check src/ --output-format github
```

**Exit codes:**
- `0` — No violations (or all fixed)
- `1` — Violations found
- `2` — Configuration/parse error

**Example output:**

```
src/main/java/com/example/Service.java:42:15: WS001 Missing whitespace before `+`
src/main/java/com/example/Service.java:42:16: WS002 Missing whitespace after `+`
src/main/java/com/example/Service.java:58:5: LC001 `{` must be on new line
Found 3 violations (3 fixable)
```

## Implementation Phases

### Phase 1: Foundation
- Fork Ruff, strip Python-specific crates
- Integrate tree-sitter-java parser
- Build jruff_java_cst with typed node wrappers
- Basic CLI: `jruff check <path>` with single hardcoded rule
- Verify end-to-end: parse → check → report

### Phase 2: Config System
- jruff_checkstyle crate: parse checkstyle.xml
- Map module names to rule enum variants
- Extract properties into typed config structs
- Add jruff.toml overlay parsing
- Config merging logic

### Phase 3: Whitespace Rules (10 rules)
- WhitespaceAround, WhitespaceAfter, NoWhitespaceBefore, NoWhitespaceAfter
- SingleSpaceSeparator, ParenPad, TypecastParenPad
- EmptyForInitializerPad, MethodParamPad
- FileTabCharacter

### Phase 4: Brace/Block Rules (4 rules)
- LeftCurly (nl option)
- RightCurly (alone option)
- NeedBraces
- Indentation

### Phase 5: Modifier Rules (4 rules)
- ModifierOrder
- RedundantModifier
- FinalParameters
- FinalLocalVariable

### Phase 6: Remaining Tier 1 (2 rules + polish)
- UpperEll
- ArrayTypeStyle
- Test suite with fixtures from real violations
- Performance profiling

### Phase 7: Tier 2 Rules + Check-only Detection
- Import rules (UnusedImports, RedundantImport)
- Remaining Tier 2 fixes
- Detection-only rules for Tier 3
- CI integration guide

### Phase 8: Polish
- Watch mode
- Caching (skip unchanged files)
- Gradle/Maven plugin
