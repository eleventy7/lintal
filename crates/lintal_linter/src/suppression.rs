//! Suppression support for lintal.
//!
//! This module provides support for checkstyle-style suppressions:
//! - `// CHECKSTYLE:OFF:RuleName` / `// CHECKSTYLE:ON:RuleName` comments
//! - `/* CHECKSTYLE:OFF:RuleName */` block comments
//! - `@SuppressWarnings("checkstyle:RuleName")` annotations
//!
//! Suppressions work by tracking ranges where specific rules are disabled.

use lintal_java_cst::CstNode;
use lintal_text_size::TextSize;
use regex::Regex;
use std::collections::HashMap;

/// A suppression region where a specific rule is disabled.
#[derive(Debug, Clone)]
pub struct SuppressionRegion {
    /// The rule name being suppressed (or "*" for all rules).
    pub rule: String,
    /// Start offset in the source.
    pub start: TextSize,
    /// End offset in the source (None means until end of file).
    pub end: Option<TextSize>,
}

/// Configuration for a plain text comment filter.
#[derive(Debug, Clone)]
pub struct PlainTextCommentFilterConfig {
    /// Regex pattern for "off" comments.
    pub off_pattern: Regex,
    /// Regex pattern for "on" comments.
    pub on_pattern: Regex,
    /// Capture group index for the rule name (1-indexed, 0 means no capture).
    pub check_format_group: usize,
}

impl PlainTextCommentFilterConfig {
    /// Create a new filter config from checkstyle properties.
    ///
    /// - `off_comment_format`: Regex for off comments, e.g., `CHECKSTYLE\:OFF\:(\w+)`
    /// - `on_comment_format`: Regex for on comments, e.g., `CHECKSTYLE\:ON\:(\w+)`
    /// - `check_format`: The format for matching rule names, e.g., `$1` for first capture group
    pub fn new(
        off_comment_format: &str,
        on_comment_format: &str,
        check_format: Option<&str>,
    ) -> Option<Self> {
        let off_pattern = Regex::new(off_comment_format).ok()?;
        let on_pattern = Regex::new(on_comment_format).ok()?;

        // Parse check_format to determine which capture group to use
        // "$1" means first capture group, "$2" means second, etc.
        let check_format_group = check_format
            .and_then(|fmt| fmt.strip_prefix('$').and_then(|s| s.parse::<usize>().ok()))
            .unwrap_or(0);

        Some(Self {
            off_pattern,
            on_pattern,
            check_format_group,
        })
    }

    /// Create the default checkstyle suppression filter.
    pub fn checkstyle_default() -> Self {
        Self::new(r"CHECKSTYLE:OFF:(\w+)", r"CHECKSTYLE:ON:(\w+)", Some("$1"))
            .expect("Default patterns should be valid")
    }
}

/// Manages suppressions for a source file.
#[derive(Debug)]
pub struct SuppressionContext {
    /// Suppression regions indexed by rule name.
    /// Key "*" matches all rules.
    regions: HashMap<String, Vec<SuppressionRegion>>,
}

impl SuppressionContext {
    /// Create a new empty suppression context.
    pub fn new() -> Self {
        Self {
            regions: HashMap::new(),
        }
    }

    /// Parse suppressions from source code using the given filter configs.
    pub fn from_source(source: &str, filters: &[PlainTextCommentFilterConfig]) -> Self {
        let mut ctx = Self::new();

        for filter in filters {
            ctx.parse_with_filter(source, filter);
        }

        ctx
    }

    /// Parse suppressions using a specific filter configuration.
    fn parse_with_filter(&mut self, source: &str, filter: &PlainTextCommentFilterConfig) {
        // Track open suppressions: rule -> start offset
        let mut open_suppressions: HashMap<String, TextSize> = HashMap::new();

        // Use char_indices for UTF-8 safe iteration
        let bytes = source.as_bytes();
        let mut pos = 0;
        while pos < bytes.len() {
            // Check for line comment (// is always ASCII)
            if pos + 1 < bytes.len() && bytes[pos] == b'/' && bytes[pos + 1] == b'/' {
                // Find end of line
                let line_end = bytes[pos..]
                    .iter()
                    .position(|&b| b == b'\n')
                    .map(|i| pos + i)
                    .unwrap_or(bytes.len());
                let comment = &source[pos..line_end];

                self.process_comment(
                    comment,
                    TextSize::new(pos as u32),
                    filter,
                    &mut open_suppressions,
                );

                pos = line_end + 1;
                continue;
            }

            // Check for block comment (/* and */ are always ASCII)
            if pos + 1 < bytes.len() && bytes[pos] == b'/' && bytes[pos + 1] == b'*' {
                // Find end of block comment
                let mut end_pos = pos + 2;
                while end_pos + 1 < bytes.len() {
                    if bytes[end_pos] == b'*' && bytes[end_pos + 1] == b'/' {
                        let comment_end = end_pos + 2;
                        let comment = &source[pos..comment_end];

                        self.process_comment(
                            comment,
                            TextSize::new(pos as u32),
                            filter,
                            &mut open_suppressions,
                        );

                        pos = comment_end;
                        break;
                    }
                    end_pos += 1;
                }
                if end_pos + 1 >= bytes.len() {
                    // Unclosed comment, skip to end
                    break;
                }
                continue;
            }

            pos += 1;
        }

        // Close any remaining open suppressions at end of file
        let end_pos = TextSize::new(source.len() as u32);
        for (rule, start) in open_suppressions {
            self.add_region(SuppressionRegion {
                rule,
                start,
                end: Some(end_pos),
            });
        }
    }

    /// Process a single comment for suppression directives.
    fn process_comment(
        &mut self,
        comment: &str,
        comment_pos: TextSize,
        filter: &PlainTextCommentFilterConfig,
        open_suppressions: &mut HashMap<String, TextSize>,
    ) {
        // Check for OFF pattern
        if let Some(captures) = filter.off_pattern.captures(comment) {
            let rule = if filter.check_format_group > 0 {
                captures
                    .get(filter.check_format_group)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_else(|| "*".to_string())
            } else {
                "*".to_string()
            };

            // Start a new suppression region
            open_suppressions.insert(rule, comment_pos);
        }

        // Check for ON pattern
        if let Some(captures) = filter.on_pattern.captures(comment) {
            let rule = if filter.check_format_group > 0 {
                captures
                    .get(filter.check_format_group)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_else(|| "*".to_string())
            } else {
                "*".to_string()
            };

            // Close the suppression region
            if let Some(start) = open_suppressions.remove(&rule) {
                self.add_region(SuppressionRegion {
                    rule,
                    start,
                    end: Some(comment_pos),
                });
            }
        }
    }

    /// Add a suppression region.
    fn add_region(&mut self, region: SuppressionRegion) {
        self.regions
            .entry(region.rule.clone())
            .or_default()
            .push(region);
    }

    /// Check if a diagnostic at the given position for the given rule is suppressed.
    pub fn is_suppressed(&self, rule_name: &str, pos: TextSize) -> bool {
        // Check rule-specific suppressions
        if let Some(regions) = self.regions.get(rule_name) {
            for region in regions {
                if pos >= region.start {
                    match region.end {
                        Some(end) if pos < end => return true,
                        None => return true,
                        _ => {}
                    }
                }
            }
        }

        // Check wildcard suppressions
        if let Some(regions) = self.regions.get("*") {
            for region in regions {
                if pos >= region.start {
                    match region.end {
                        Some(end) if pos < end => return true,
                        None => return true,
                        _ => {}
                    }
                }
            }
        }

        false
    }

    /// Check if there are any suppressions.
    pub fn has_suppressions(&self) -> bool {
        !self.regions.is_empty()
    }

    /// Parse @SuppressWarnings annotations from a CST tree.
    /// Looks for annotations like:
    /// - `@SuppressWarnings("checkstyle:RuleName")`
    /// - `@SuppressWarnings({"checkstyle:Rule1", "checkstyle:Rule2"})`
    pub fn parse_suppress_warnings(&mut self, source: &str, root: &CstNode) {
        self.visit_for_annotations(source, root);
    }

    /// Recursively visit nodes to find @SuppressWarnings annotations.
    fn visit_for_annotations(&mut self, source: &str, node: &CstNode) {
        // Check if this node has annotations (modifiers that contain annotations)
        if matches!(
            node.kind(),
            "class_declaration"
                | "interface_declaration"
                | "enum_declaration"
                | "method_declaration"
                | "constructor_declaration"
                | "field_declaration"
                | "annotation_type_declaration"
                | "record_declaration"
        ) {
            // Look for modifiers containing annotations
            // In tree-sitter-java, modifiers is a child node with kind "modifiers"
            let modifiers = node.children().find(|c| c.kind() == "modifiers");
            if let Some(modifiers) = modifiers {
                for child in modifiers.children() {
                    if child.kind() == "annotation" || child.kind() == "marker_annotation" {
                        self.process_annotation(source, &child, node);
                    }
                }
            }

            // Also check direct children - annotations might be direct children
            for child in node.children() {
                if child.kind() == "annotation" || child.kind() == "marker_annotation" {
                    self.process_annotation(source, &child, node);
                }
            }
        }

        // Also check for annotations directly on local variables
        if node.kind() == "local_variable_declaration" {
            // Local variables can have annotations before the type
            for child in node.children() {
                if child.kind() == "annotation" || child.kind() == "marker_annotation" {
                    self.process_annotation(source, &child, node);
                }
            }
        }

        // Recurse into children
        for child in node.named_children() {
            self.visit_for_annotations(source, &child);
        }
    }

    /// Process a single annotation to check if it's @SuppressWarnings.
    fn process_annotation(&mut self, source: &str, annotation: &CstNode, target: &CstNode) {
        // Get the annotation name - try both "name" field and looking for identifier
        let name = annotation
            .child_by_field_name("name")
            .or_else(|| {
                // Some tree-sitter versions use a direct identifier child
                annotation.children().find(|c| c.kind() == "identifier")
            })
            .map(|n| &source[n.range()])
            .unwrap_or("");

        if name != "SuppressWarnings" {
            return;
        }

        // Get the annotation arguments
        if let Some(args) = annotation.child_by_field_name("arguments") {
            // Extract the string values from the annotation
            let rules = self.extract_suppress_warnings_rules(source, &args);
            for rule in rules {
                self.add_region(SuppressionRegion {
                    rule,
                    start: target.range().start(),
                    end: Some(target.range().end()),
                });
            }
        }
    }

    /// Extract rule names from @SuppressWarnings annotation arguments.
    /// Handles both single strings and arrays: "checkstyle:Rule" or {"checkstyle:Rule1", "rule2"}
    fn extract_suppress_warnings_rules(&self, source: &str, args: &CstNode) -> Vec<String> {
        let mut rules = Vec::new();

        // Look for string literals or array initializers
        // In tree-sitter-java, the structure can be:
        // - annotation_argument_list > element_value_pair > string_literal
        // - annotation_argument_list > element_value_pair > element_value_array_initializer > string_literal
        // - annotation_argument_list > string_literal (direct value)
        // - annotation_argument_list > element_value_array_initializer > string_literal (direct array)

        self.extract_rules_recursive(source, args, &mut rules);

        rules
    }

    /// Recursively extract string values from annotation arguments.
    fn extract_rules_recursive(&self, source: &str, node: &CstNode, rules: &mut Vec<String>) {
        match node.kind() {
            "string_literal" => {
                if let Some(rule) = self.parse_suppress_warning_value(source, node) {
                    rules.push(rule);
                }
            }
            "element_value_array_initializer" | "array_initializer" => {
                // Array of values
                for child in node.named_children() {
                    self.extract_rules_recursive(source, &child, rules);
                }
            }
            "element_value_pair" => {
                // key=value pair - extract the value
                if let Some(value) = node.child_by_field_name("value") {
                    self.extract_rules_recursive(source, &value, rules);
                }
            }
            "annotation_argument_list" => {
                // Recurse into all children
                for child in node.named_children() {
                    self.extract_rules_recursive(source, &child, rules);
                }
            }
            _ => {
                // Try children for other node types
                for child in node.named_children() {
                    self.extract_rules_recursive(source, &child, rules);
                }
            }
        }
    }

    /// Parse a single @SuppressWarnings string value.
    /// Returns the rule name if it matches "checkstyle:RuleName".
    fn parse_suppress_warning_value(&self, source: &str, string_lit: &CstNode) -> Option<String> {
        let text = &source[string_lit.range()];
        // Remove quotes
        let content = text.trim_matches('"');

        // Check for checkstyle prefix
        content
            .strip_prefix("checkstyle:")
            .map(|rule| rule.to_string())
    }
}

impl Default for SuppressionContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_line_comment_suppression() {
        let source = r#"
class Foo {
    // CHECKSTYLE:OFF:WhitespaceAround
    void method( int x ) { }
    // CHECKSTYLE:ON:WhitespaceAround
    void other() { }
}
"#;

        let filter = PlainTextCommentFilterConfig::checkstyle_default();
        let ctx = SuppressionContext::from_source(source, &[filter]);

        assert!(ctx.has_suppressions());

        // Position inside suppression region
        let suppressed_pos = TextSize::new(source.find("void method").unwrap() as u32);
        assert!(ctx.is_suppressed("WhitespaceAround", suppressed_pos));

        // Position outside suppression region
        let not_suppressed_pos = TextSize::new(source.find("void other").unwrap() as u32);
        assert!(!ctx.is_suppressed("WhitespaceAround", not_suppressed_pos));

        // Different rule not suppressed
        assert!(!ctx.is_suppressed("ParenPad", suppressed_pos));
    }

    #[test]
    fn test_parse_block_comment_suppression() {
        let source = r#"
class Foo {
    /* CHECKSTYLE:OFF:EmptyBlock */
    void method() { }
    /* CHECKSTYLE:ON:EmptyBlock */
}
"#;

        let filter = PlainTextCommentFilterConfig::checkstyle_default();
        let ctx = SuppressionContext::from_source(source, &[filter]);

        let suppressed_pos = TextSize::new(source.find("void method").unwrap() as u32);
        assert!(ctx.is_suppressed("EmptyBlock", suppressed_pos));
    }

    #[test]
    fn test_unclosed_suppression() {
        let source = r#"
class Foo {
    // CHECKSTYLE:OFF:WhitespaceAround
    void method( int x ) { }
}
"#;

        let filter = PlainTextCommentFilterConfig::checkstyle_default();
        let ctx = SuppressionContext::from_source(source, &[filter]);

        // Everything after OFF should be suppressed
        let pos = TextSize::new(source.find("void method").unwrap() as u32);
        assert!(ctx.is_suppressed("WhitespaceAround", pos));

        // End of file should also be suppressed
        let end_pos = TextSize::new((source.len() - 5) as u32);
        assert!(ctx.is_suppressed("WhitespaceAround", end_pos));
    }

    #[test]
    fn test_custom_pattern() {
        let source = r#"
class Foo {
    // @suppress:ParenPad
    void method( int x ) { }
    // @unsuppress:ParenPad
}
"#;

        let filter =
            PlainTextCommentFilterConfig::new(r"@suppress:(\w+)", r"@unsuppress:(\w+)", Some("$1"))
                .unwrap();

        let ctx = SuppressionContext::from_source(source, &[filter]);

        let suppressed_pos = TextSize::new(source.find("void method").unwrap() as u32);
        assert!(ctx.is_suppressed("ParenPad", suppressed_pos));
    }

    #[test]
    fn test_suppress_warnings_annotation() {
        use lintal_java_parser::JavaParser;

        let source = r#"
class Foo {
    @SuppressWarnings({"checkstyle:AvoidNestedBlocks", "checkstyle:MethodLength"})
    void method() {
        { int x = 1; }
    }
}
"#;

        let mut parser = JavaParser::new();
        let result = parser.parse(source).expect("Failed to parse");
        let root = CstNode::new(result.tree.root_node(), source);

        let mut ctx = SuppressionContext::new();
        ctx.parse_suppress_warnings(source, &root);

        // Debug: print what regions we found
        eprintln!("Suppression regions: {:?}", ctx.regions);

        assert!(ctx.has_suppressions(), "Should have suppressions");

        // Position inside the method
        let nested_block_pos = TextSize::new(source.find("{ int x").unwrap() as u32);
        assert!(
            ctx.is_suppressed("AvoidNestedBlocks", nested_block_pos),
            "AvoidNestedBlocks should be suppressed"
        );
        assert!(
            ctx.is_suppressed("MethodLength", nested_block_pos),
            "MethodLength should be suppressed"
        );
    }
}
