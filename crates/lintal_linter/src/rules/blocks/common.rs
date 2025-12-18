//! Shared helpers for blocks rules.

use lintal_java_cst::CstNode;
use lintal_text_size::TextSize;

use crate::CheckContext;

/// Check if two nodes are on the same line.
pub fn are_on_same_line(ctx: &CheckContext, a: &CstNode, b: &CstNode) -> bool {
    let source_code = ctx.source_code();
    let a_line = source_code.line_column(a.range().start()).line;
    let b_line = source_code.line_column(b.range().start()).line;
    a_line == b_line
}

/// Check if a node is alone on its line (only whitespace before it).
pub fn is_alone_on_line(ctx: &CheckContext, node: &CstNode) -> bool {
    let line_index = ctx.line_index();
    let source_code = ctx.source_code();
    let line_start = line_index.line_start(
        source_code.line_column(node.range().start()).line,
        ctx.source(),
    );
    let before = &ctx.source()[usize::from(line_start)..usize::from(node.range().start())];
    before.chars().all(|c| c.is_whitespace())
}

/// Check if there's a line break before a position.
pub fn has_line_break_before(source: &str, pos: TextSize) -> bool {
    let before = &source[..usize::from(pos)];
    before
        .chars()
        .rev()
        .take_while(|c| *c != '\n')
        .all(|c| c.is_whitespace())
        && before.contains('\n')
}

/// Get column number (1-indexed) for a node.
pub fn get_column(ctx: &CheckContext, node: &CstNode) -> usize {
    ctx.source_code()
        .line_column(node.range().start())
        .column
        .get()
}
