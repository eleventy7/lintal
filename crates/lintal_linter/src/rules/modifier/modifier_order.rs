//! ModifierOrder rule implementation.
//!
//! Checks that the order of modifiers conforms to the JLS suggestions.
//! This is a port of the checkstyle ModifierOrderCheck for 100% compatibility.

use lintal_diagnostics::{Diagnostic, Edit, Fix, FixAvailability, Violation};
use lintal_java_cst::CstNode;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Configuration for ModifierOrder rule.
#[derive(Debug, Clone)]
pub struct ModifierOrder;

const RELEVANT_KINDS: &[&str] = &["modifiers"];

impl Default for ModifierOrder {
    fn default() -> Self {
        Self
    }
}

impl FromConfig for ModifierOrder {
    const MODULE_NAME: &'static str = "ModifierOrder";

    fn from_config(_properties: &Properties) -> Self {
        Self
    }
}

/// Violation for modifier out of order.
#[derive(Debug, Clone)]
pub struct ModifierOutOfOrder {
    pub modifier: String,
    pub column: usize,
}

impl Violation for ModifierOutOfOrder {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    fn message(&self) -> String {
        format!(
            "'{}' modifier out of order with the JLS suggestions",
            self.modifier
        )
    }
}

/// Violation for annotation must come before other modifiers.
#[derive(Debug, Clone)]
pub struct AnnotationMustPrecedeModifiers {
    pub annotation: String,
    pub column: usize,
}

impl Violation for AnnotationMustPrecedeModifiers {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    fn message(&self) -> String {
        format!(
            "'{}' annotation modifier does not precede non-annotation modifiers",
            self.annotation
        )
    }
}

impl Rule for ModifierOrder {
    fn name(&self) -> &'static str {
        "ModifierOrder"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        if node.kind() != "modifiers" {
            return vec![];
        }

        let mut diagnostics = vec![];

        // Collect all modifiers (both annotations and regular modifiers)
        // Exclude comments which may appear between modifiers in some codebases
        let mods: Vec<CstNode> = node
            .children()
            .filter(|child| !matches!(child.kind(), "line_comment" | "block_comment"))
            .collect();

        if mods.is_empty() {
            return diagnostics;
        }

        // Check order according to checkstyle's algorithm
        if let Some(error) = self.check_order_suggested_by_jls(&mods, ctx, node) {
            diagnostics.push(error);
        }

        diagnostics
    }
}

impl ModifierOrder {
    /// Checks if the modifiers were added in the order suggested in the JLS.
    /// Returns Some(diagnostic) if there's a violation, None otherwise.
    fn check_order_suggested_by_jls(
        &self,
        modifiers: &[CstNode],
        ctx: &CheckContext,
        modifiers_node: &CstNode,
    ) -> Option<Diagnostic> {
        let mut iter = modifiers.iter();

        // Speed past all initial annotations
        let mut modifier = self.skip_annotations(&mut iter)?;

        // All modifiers are annotations, no problem
        if Self::is_annotation(&modifier) {
            return None;
        }

        let mut current_index = 0;

        loop {
            if Self::is_annotation(&modifier) {
                // Check if this is a type annotation (which should be skipped)
                if !self.is_annotation_on_type(&modifier) {
                    // Annotation not at start of modifiers, bad
                    let annotation_text = self.get_annotation_text(&modifier, ctx);
                    let fix = self.create_reorder_fix(ctx, modifiers_node);
                    return Some(
                        Diagnostic::new(
                            AnnotationMustPrecedeModifiers {
                                annotation: annotation_text,
                                column: Self::get_column(ctx, &modifier),
                            },
                            modifier.range(),
                        )
                        .with_fix(fix),
                    );
                }
                break;
            }

            // Get the modifier text
            let modifier_text = &ctx.source()[modifier.range()];

            // Find the index of this modifier in JLS order
            while current_index < super::common::JLS_MODIFIER_ORDER.len() {
                if super::common::JLS_MODIFIER_ORDER[current_index] == modifier_text {
                    break;
                }
                current_index += 1;
            }

            if current_index == super::common::JLS_MODIFIER_ORDER.len() {
                // Current modifier is out of JLS order
                let fix = self.create_reorder_fix(ctx, modifiers_node);
                return Some(
                    Diagnostic::new(
                        ModifierOutOfOrder {
                            modifier: modifier_text.to_string(),
                            column: Self::get_column(ctx, &modifier),
                        },
                        modifier.range(),
                    )
                    .with_fix(fix),
                );
            }

            // Move to next modifier
            match iter.next() {
                Some(next) => modifier = *next,
                None => break,
            }
        }

        None
    }

    /// Create a fix that reorders modifiers to match JLS order.
    fn create_reorder_fix(&self, ctx: &CheckContext, modifiers_node: &CstNode) -> Fix {
        let source = ctx.source();

        // Collect all modifiers with their text and ranges
        let mut annotations = Vec::new();
        let mut keyword_modifiers = Vec::new();

        for child in modifiers_node.children() {
            let text = &source[child.range()];
            if Self::is_annotation(&child) {
                annotations.push((text.to_string(), child));
            } else {
                keyword_modifiers.push((text.to_string(), child));
            }
        }

        // Sort keyword modifiers by JLS order
        keyword_modifiers
            .sort_by_key(|(text, _)| super::common::jls_order_index(text).unwrap_or(usize::MAX));

        // Build the correctly ordered modifier string
        let mut ordered_parts = Vec::new();
        for (text, _) in &annotations {
            ordered_parts.push(text.clone());
        }
        for (text, _) in &keyword_modifiers {
            ordered_parts.push(text.clone());
        }
        let ordered_text = ordered_parts.join(" ");

        // Add trailing space if needed (to maintain spacing with what follows)
        let replacement = if ordered_text.is_empty() {
            ordered_text
        } else {
            format!("{} ", ordered_text)
        };

        Fix::safe_edit(Edit::range_replacement(replacement, modifiers_node.range()))
    }

    /// Skip all annotations in modifier block.
    /// Returns the first non-annotation modifier (or last annotation if all are annotations).
    fn skip_annotations<'a, I>(&self, iter: &mut I) -> Option<CstNode<'a>>
    where
        I: Iterator<Item = &'a CstNode<'a>>,
    {
        let mut current = iter.next()?;
        while Self::is_annotation(current) {
            match iter.next() {
                Some(next) => current = next,
                None => break,
            }
        }
        Some(*current)
    }

    /// Check if a node is an annotation.
    fn is_annotation(node: &CstNode) -> bool {
        node.kind() == "marker_annotation"
            || node.kind() == "annotation"
            || node.kind() == "normal_annotation"
    }

    /// Checks whether annotation on type takes place.
    fn is_annotation_on_type(&self, modifier: &CstNode) -> bool {
        let Some(modifiers) = modifier.parent() else {
            return false;
        };
        let Some(definition) = modifiers.parent() else {
            return false;
        };

        match definition.kind() {
            "field_declaration"
            | "local_variable_declaration"
            | "formal_parameter"
            | "catch_formal_parameter"
            | "constructor_declaration" => true,
            "method_declaration" => {
                // Check if method has non-void return type
                if let Some(type_node) = definition.child_by_field_name("type") {
                    // If there's a type and it's not void, this could be a type annotation
                    type_node.kind() != "void_type"
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Get the text of an annotation (e.g., "@MyAnnotation").
    fn get_annotation_text(&self, annotation: &CstNode, ctx: &CheckContext) -> String {
        ctx.source()[annotation.range()].to_string()
    }

    /// Get column number (1-indexed) for a node.
    fn get_column(ctx: &CheckContext, node: &CstNode) -> usize {
        ctx.source_code()
            .line_column(node.range().start())
            .column
            .get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lintal_java_cst::TreeWalker;
    use lintal_java_parser::JavaParser;

    fn check_source(source: &str) -> Vec<Diagnostic> {
        let mut parser = JavaParser::new();
        let result = parser.parse(source).unwrap();
        let ctx = CheckContext::new(source);
        let rule = ModifierOrder;

        let mut diagnostics = vec![];
        for node in TreeWalker::new(result.tree.root_node(), source) {
            diagnostics.extend(rule.check(&ctx, &node));
        }
        diagnostics
    }

    #[test]
    fn test_correct_order() {
        let source = "class Foo { public static final void test() {} }";
        let diagnostics = check_source(source);
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_incorrect_order_final_before_static() {
        let source = "class Foo { final static void test() {} }";
        let diagnostics = check_source(source);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_annotation_before_modifiers_ok() {
        let source = "class Foo { @Override public void test() {} }";
        let diagnostics = check_source(source);
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_annotation_after_modifiers_error() {
        let source = "class Foo { public @Override void test() {} }";
        let diagnostics = check_source(source);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_catch_type_annotation_ok() {
        // Type annotations on catch parameters are valid
        let source = r#"
            @interface DoNotSub {}
            class Foo {
                void test() {
                    try {} catch (final @DoNotSub Exception e) {}
                }
            }
        "#;
        let diagnostics = check_source(source);
        assert_eq!(diagnostics.len(), 0);
    }
}
