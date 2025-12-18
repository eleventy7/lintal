# Brace Autofix Design (LeftCurly, RightCurly)

## Goal
Add auto-fix support for `LeftCurly` and `RightCurly` rules, preserving inline comments (`//` and `/* */`) while keeping fixes localized and safe. Support all existing rule options:

- `LeftCurly`: `EOL`, `NL`, `NLOW`
- `RightCurly`: `SAME`, `ALONE`, `ALONE_OR_SINGLELINE`

## Scope
- Apply targeted edits only to the brace line(s) and the insertion point line.
- Preserve inline comments on the brace line if the line is otherwise whitespace + brace + optional single comment.
- Skip fixes when the brace line contains additional non-whitespace tokens beyond the brace and a single inline comment.
- Mark fixes as `Applicability::Safe` when conditions are met; otherwise omit fix but keep diagnostic.

## Architecture
Both rules already compute diagnostics based on CST nodes and line/column helpers. We will extend those rule methods to attach a `Fix` via `Diagnostic::with_fix(...)` when the move is unambiguous.

Key building blocks:
- Reuse `LineIndex`/`SourceCode` for line boundaries and column computation.
- Add helpers to extract the brace line segment and determine whether it is safe to move (only whitespace + brace + optional inline comment).
- Add helpers to compute target insertion position and indentation at the target line.

## Fix Strategy

### LeftCurly
- `NL`: move `{` (plus inline comment) to a new line after the start token line, using the target line indentation.
- `EOL`: move `{` (plus inline comment) to end of the start token line; append inline comment after the brace.
- `EOL` line-break-after: insert newline and indentation after `{` when needed, unless empty block `{}`.
- `NLOW`: apply move only when the option expects a newline and the move is unambiguous; otherwise skip fix.

### RightCurly
- `SAME`: when `}` should be on the same line as `else/catch/finally`, move `}` (plus inline comment) to the line containing the next clause token, inserting a single space if needed.
- `ALONE`/`ALONE_OR_SINGLELINE`: move `}` (plus inline comment) to its own line with correct indentation. For `ALONE_OR_SINGLELINE`, allow single-line blocks as-is.

## Comment Preservation
Treat the brace line as fixable only if it matches:

- Leading whitespace
- Brace token (`{` or `}`)
- Optional whitespace
- Optional single inline comment (`// ...` or `/* ... */`)
- Optional trailing whitespace/newline

If additional tokens exist, the fix is omitted to avoid unintended changes.

## Data Flow
1. Rule identifies a violation and the brace CST node.
2. Helpers determine:
   - Brace line bounds (start/end indices).
   - Whether the line is safely movable (brace + optional comment only).
   - The insertion point (line start or end) and indentation.
3. Build a `Fix`:
   - `Edit::range_deletion` for the original brace line segment.
   - `Edit::insertion` or `Edit::range_replacement` at the target position.
4. Attach `Fix::safe_edit` (or `safe_edits` if both delete and insert are needed).

## Error Handling
- If the brace line is ambiguous (multiple tokens, mixed inline content, or complex trailing code), skip fix.
- If insertion would require changes beyond the target line (e.g., crossing a semicolon in an unknown context), skip fix.

## Testing
Add tests for both rules and all options:
- Each diagnostic includes a fix when conditions are met.
- Inline comment preservation for both `//` and `/* */`.
- Empty blocks `{}`.
- Single-line blocks allowed for `ALONE_OR_SINGLELINE`.
- Multi-block chains: `if/else`, `try/catch/finally`.

## Next Steps
- Implement helper functions for brace-line extraction and comment detection.
- Add fixes in `left_curly.rs` and `right_curly.rs`.
- Extend rule tests to validate fix output and comment preservation.
