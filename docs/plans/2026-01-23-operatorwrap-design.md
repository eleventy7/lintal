# OperatorWrap Check Implementation Design

## Overview

Add comprehensive OperatorWrap support to lintal with 100% checkstyle compatibility and auto-fix capability.

## Gap Analysis

Current implementation only handles `binary_expression` nodes. Missing:
- Configurable `tokens` parameter (checkstyle supports ~35 operator types)
- Auto-fix support
- Non-binary operators: ternary `?:`, assignment `=`/`+=`, instanceof, method reference `::`, enhanced-for `:`, type bounds `&`

## Operators to Support

| Category | Tokens | Tree-sitter Node Types |
|----------|--------|------------------------|
| Arithmetic | PLUS, MINUS, STAR, DIV, MOD | `binary_expression` |
| Comparison | EQUAL, NOT_EQUAL, GT, GE, LT, LE | `binary_expression` |
| Logical | LAND, LOR | `binary_expression` |
| Bitwise | BAND, BOR, BXOR, SL, SR, BSR | `binary_expression` |
| Ternary | QUESTION, COLON | `ternary_expression` |
| Assignment | ASSIGN, PLUS_ASSIGN, etc. | `assignment_expression`, `variable_declarator` |
| Type bounds | TYPE_EXTENSION_AND | `type_bound` |
| instanceof | LITERAL_INSTANCEOF | `instanceof_expression` |
| Method ref | METHOD_REF | `method_reference` |
| Enhanced for | COLON | `enhanced_for_statement` |

## Test Harness

Parse expected violations from checkstyle test file comments. Report:
1. Correct matches
2. Missing matches (false negatives)
3. False positives

Test all 14 fixture files with their specific configs.

## Auto-Fix Strategy

- NL: Move operator from end of line to start of next line
- EOL: Move operator from start of line to end of previous line
- Skip fix when comments exist between operands

## Implementation Order

1. Validate tree-sitter nodes with dump_java_ast
2. Build test harness
3. Implement token enum and config parsing
4. Implement each operator category incrementally
5. Add auto-fix support
6. Final compatibility pass
