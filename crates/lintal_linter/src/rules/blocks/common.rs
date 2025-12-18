//! Shared helpers for blocks rules.

use lintal_java_cst::CstNode;
use lintal_source_file::{LineIndex, SourceCode};
use lintal_text_size::TextSize;

/// Check if two nodes are on the same line.
pub fn are_on_same_line(source: &str, a: &CstNode, b: &CstNode) -> bool {
    let line_index = LineIndex::from_source_text(source);
    let source_code = SourceCode::new(source, &line_index);
    let a_line = source_code.line_column(a.range().start()).line;
    let b_line = source_code.line_column(b.range().start()).line;
    a_line == b_line
}

/// Check if a node is alone on its line (only whitespace before it).
pub fn is_alone_on_line(source: &str, node: &CstNode) -> bool {
    let line_index = LineIndex::from_source_text(source);
    let line_start = line_index.line_start(
        SourceCode::new(source, &line_index)
            .line_column(node.range().start())
            .line,
        source,
    );
    let before = &source[usize::from(line_start)..usize::from(node.range().start())];
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
pub fn get_column(source: &str, node: &CstNode) -> usize {
    let line_index = LineIndex::from_source_text(source);
    let source_code = SourceCode::new(source, &line_index);
    source_code.line_column(node.range().start()).column.get()
}
