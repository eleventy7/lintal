//! Shared helpers for whitespace rules.

use lintal_diagnostics::{Diagnostic, Edit, Fix, FixAvailability, Violation};
use lintal_java_cst::CstNode;
use lintal_text_size::{TextRange, TextSize};

/// Check if character before position is whitespace.
pub fn has_whitespace_before(source: &str, pos: TextSize) -> bool {
    if pos == TextSize::new(0) {
        return true; // Start of file counts as whitespace
    }
    let idx = usize::from(pos);
    source[..idx]
        .chars()
        .last()
        .is_some_and(|c| c.is_whitespace())
}

/// Check if character after position is whitespace.
pub fn has_whitespace_after(source: &str, pos: TextSize) -> bool {
    let idx = usize::from(pos);
    source[idx..]
        .chars()
        .next()
        .is_some_and(|c| c.is_whitespace())
}

/// Get the character before a position.
pub fn char_before(source: &str, pos: TextSize) -> Option<char> {
    if pos == TextSize::new(0) {
        return None;
    }
    let idx = usize::from(pos);
    source[..idx].chars().last()
}

/// Get the character after a position.
pub fn char_after(source: &str, pos: TextSize) -> Option<char> {
    let idx = usize::from(pos);
    source[idx..].chars().next()
}

/// Check if character before position is a newline.
pub fn has_newline_before(source: &str, pos: TextSize) -> bool {
    char_before(source, pos).is_some_and(|c| c == '\n')
}

/// Check if character after position is a newline.
pub fn has_newline_after(source: &str, pos: TextSize) -> bool {
    char_after(source, pos).is_some_and(|c| c == '\n')
}

/// Find the range of whitespace before a position.
/// Returns None if no whitespace before.
pub fn whitespace_range_before(source: &str, pos: TextSize) -> Option<TextRange> {
    let idx = usize::from(pos);
    let before = &source[..idx];

    let ws_len = before
        .chars()
        .rev()
        .take_while(|c| c.is_whitespace())
        .count();
    if ws_len == 0 {
        return None;
    }

    // Count bytes, not chars
    let ws_bytes: usize = before
        .chars()
        .rev()
        .take(ws_len)
        .map(|c| c.len_utf8())
        .sum();
    let start = TextSize::new((idx - ws_bytes) as u32);
    Some(TextRange::new(start, pos))
}

/// Find the range of whitespace after a position.
/// Returns None if no whitespace after.
pub fn whitespace_range_after(source: &str, pos: TextSize) -> Option<TextRange> {
    let idx = usize::from(pos);
    let after = &source[idx..];

    let ws_len = after.chars().take_while(|c| c.is_whitespace()).count();
    if ws_len == 0 {
        return None;
    }

    // Count bytes, not chars
    let ws_bytes: usize = after.chars().take(ws_len).map(|c| c.len_utf8()).sum();
    let end = TextSize::new((idx + ws_bytes) as u32);
    Some(TextRange::new(pos, end))
}

// ============================================================================
// Violation types shared across whitespace rules
// ============================================================================

/// Violation: token is not followed by whitespace.
#[derive(Debug, Clone)]
pub struct NotFollowed {
    pub token: String,
}

impl Violation for NotFollowed {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    fn message(&self) -> String {
        format!("'{}' is not followed by whitespace", self.token)
    }
}

/// Violation: token is not preceded by whitespace.
#[derive(Debug, Clone)]
pub struct NotPreceded {
    pub token: String,
}

impl Violation for NotPreceded {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    fn message(&self) -> String {
        format!("'{}' is not preceded by whitespace", self.token)
    }
}

/// Violation: token is followed by whitespace (when it shouldn't be).
#[derive(Debug, Clone)]
pub struct Followed {
    pub token: String,
}

impl Violation for Followed {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    fn message(&self) -> String {
        format!("'{}' is followed by whitespace", self.token)
    }
}

/// Violation: token is preceded by whitespace (when it shouldn't be).
#[derive(Debug, Clone)]
pub struct Preceded {
    pub token: String,
}

impl Violation for Preceded {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    fn message(&self) -> String {
        format!("'{}' is preceded by whitespace", self.token)
    }
}

// ============================================================================
// Diagnostic builders
// ============================================================================

/// Create diagnostic for missing whitespace after token.
pub fn diag_not_followed(token: &CstNode) -> Diagnostic {
    let range = token.range();
    let text = token.text().to_string();
    Diagnostic::new(NotFollowed { token: text }, range).with_fix(Fix::safe_edit(Edit::insertion(
        " ".to_string(),
        range.end(),
    )))
}

/// Create diagnostic for missing whitespace before token.
pub fn diag_not_preceded(token: &CstNode) -> Diagnostic {
    let range = token.range();
    let text = token.text().to_string();
    Diagnostic::new(NotPreceded { token: text }, range).with_fix(Fix::safe_edit(Edit::insertion(
        " ".to_string(),
        range.start(),
    )))
}

/// Create diagnostic for unexpected whitespace after token.
pub fn diag_followed(token: &CstNode, ws_range: TextRange) -> Diagnostic {
    let range = token.range();
    let text = token.text().to_string();
    Diagnostic::new(Followed { token: text }, range)
        .with_fix(Fix::safe_edit(Edit::range_deletion(ws_range)))
}

/// Create diagnostic for unexpected whitespace before token.
pub fn diag_preceded(token: &CstNode, ws_range: TextRange) -> Diagnostic {
    let range = token.range();
    let text = token.text().to_string();
    Diagnostic::new(Preceded { token: text }, range)
        .with_fix(Fix::safe_edit(Edit::range_deletion(ws_range)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_whitespace_before() {
        assert!(has_whitespace_before("a b", TextSize::new(2)));
        assert!(!has_whitespace_before("ab", TextSize::new(1)));
        assert!(has_whitespace_before("a", TextSize::new(0))); // start of file
    }

    #[test]
    fn test_has_whitespace_after() {
        assert!(has_whitespace_after("a b", TextSize::new(1)));
        assert!(!has_whitespace_after("ab", TextSize::new(1)));
    }

    #[test]
    fn test_whitespace_range_before() {
        let range = whitespace_range_before("a  b", TextSize::new(3));
        assert!(range.is_some());
        let r = range.unwrap();
        assert_eq!(r.start(), TextSize::new(1));
        assert_eq!(r.end(), TextSize::new(3));
    }

    #[test]
    fn test_whitespace_range_after() {
        let range = whitespace_range_after("a  b", TextSize::new(1));
        assert!(range.is_some());
        let r = range.unwrap();
        assert_eq!(r.start(), TextSize::new(1));
        assert_eq!(r.end(), TextSize::new(3));
    }
}
