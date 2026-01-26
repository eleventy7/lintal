//! FinalClass rule implementation.
//!
//! Checks that classes with only private constructors are declared as final.
//!
//! Checkstyle equivalent: FinalClassCheck

use lintal_diagnostics::{Diagnostic, Edit, Fix, FixAvailability, Violation};
use lintal_java_cst::CstNode;

use crate::{CheckContext, FromConfig, Properties, Rule};

use super::common::has_modifier;

/// Violation: class should be declared as final.
#[derive(Debug, Clone)]
pub struct ClassShouldBeFinalViolation {
    pub class_name: String,
}

impl Violation for ClassShouldBeFinalViolation {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    fn message(&self) -> String {
        format!("Class {} should be declared as final.", self.class_name)
    }
}

/// Configuration for FinalClass rule.
#[derive(Debug, Clone, Default)]
pub struct FinalClass;

const RELEVANT_KINDS: &[&str] = &["class_declaration"];

impl FromConfig for FinalClass {
    const MODULE_NAME: &'static str = "FinalClass";

    fn from_config(_properties: &Properties) -> Self {
        Self
    }
}

impl Rule for FinalClass {
    fn name(&self) -> &'static str {
        "FinalClass"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        if node.kind() != "class_declaration" {
            return vec![];
        }

        // Find modifiers - it's a child with kind "modifiers"
        let modifiers = node.children().find(|child| child.kind() == "modifiers");

        // Skip if already final
        if let Some(ref modifiers) = modifiers {
            if has_modifier(modifiers, "final") {
                return vec![];
            }
            // Skip abstract classes
            if has_modifier(modifiers, "abstract") {
                return vec![];
            }
        }

        // Get class name
        let Some(name_node) = node.child_by_field_name("name") else {
            return vec![];
        };
        let class_name = &ctx.source()[name_node.range()];

        // Get class body
        let Some(body) = node.child_by_field_name("body") else {
            return vec![];
        };

        // Find all constructors in the class body
        let constructors: Vec<CstNode> = body
            .children()
            .filter(|child| child.kind() == "constructor_declaration")
            .collect();

        // Determine if all constructors are effectively private
        let effectively_private = if constructors.is_empty() {
            // No explicit constructors - check if class itself is private
            // A private inner class has an implicit private constructor
            self.is_private_inner_class(&modifiers)
        } else {
            // Check if ALL explicit constructors are private
            constructors.iter().all(|ctor| self.is_private(ctor))
        };

        if !effectively_private {
            return vec![];
        }

        // Check edge case: nested class that extends outer class
        // This is a valid pattern where inner class uses private constructor
        if self.is_nested_class_extending_outer(ctx, node) {
            return vec![];
        }

        // Check if any nested class extends this class (valid pattern)
        if self.has_nested_subclass(ctx, node, class_name) {
            return vec![];
        }

        // Check if any anonymous class in the file extends this class
        if self.has_anonymous_subclass_in_file(ctx, node, class_name) {
            return vec![];
        }

        // The class has only private constructors and should be final
        let range = name_node.range();

        // Create fix: insert "final " before "class" keyword
        let insert_pos = self.find_class_keyword_position(node, &modifiers);

        vec![
            Diagnostic::new(
                ClassShouldBeFinalViolation {
                    class_name: class_name.to_string(),
                },
                range,
            )
            .with_fix(Fix::safe_edit(Edit::insertion(
                "final ".to_string(),
                insert_pos,
            ))),
        ]
    }
}

impl FinalClass {
    /// Check if a constructor has the private modifier.
    fn is_private(&self, ctor: &CstNode) -> bool {
        // Find modifiers - it's a child with kind "modifiers"
        let modifiers = ctor.children().find(|child| child.kind() == "modifiers");
        if let Some(modifiers) = modifiers {
            return has_modifier(&modifiers, "private");
        }
        // No modifiers means package-private (not private)
        false
    }

    /// Check if the class itself has private modifier (making it a private inner class).
    /// A private inner class with no explicit constructor effectively has a private constructor.
    fn is_private_inner_class(&self, modifiers: &Option<CstNode>) -> bool {
        if let Some(modifiers) = modifiers {
            return has_modifier(modifiers, "private");
        }
        false
    }

    /// Find the position of the "class" keyword to insert "final " before it.
    fn find_class_keyword_position(
        &self,
        node: &CstNode,
        _modifiers: &Option<CstNode>,
    ) -> lintal_text_size::TextSize {
        // Always find the "class" keyword and insert "final " before it
        // This correctly handles the spacing regardless of existing modifiers
        for child in node.children() {
            if child.kind() == "class" {
                return child.range().start();
            }
        }

        // Fallback to the start of the node
        node.range().start()
    }

    /// Check if this class is a nested class that extends its outer class.
    fn is_nested_class_extending_outer(&self, ctx: &CheckContext, node: &CstNode) -> bool {
        // Check if we're inside another class
        let Some(parent) = node.parent() else {
            return false;
        };

        // Walk up to find containing class
        let mut current = parent;
        let mut outer_class_name: Option<&str> = None;

        loop {
            if current.kind() == "class_body"
                && let Some(class_decl) = current.parent()
                && class_decl.kind() == "class_declaration"
            {
                if let Some(name) = class_decl.child_by_field_name("name") {
                    outer_class_name = Some(&ctx.source()[name.range()]);
                }
                break;
            }
            match current.parent() {
                Some(p) => current = p,
                None => break,
            }
        }

        let Some(outer_name) = outer_class_name else {
            return false;
        };

        // Check if this class extends the outer class
        if let Some(superclass) = node.child_by_field_name("superclass") {
            // The superclass node contains the "extends" keyword and type
            for child in superclass.children() {
                let type_text = &ctx.source()[child.range()];
                if type_text == outer_name {
                    return true;
                }
            }
        }

        false
    }

    /// Check if any nested class extends this class.
    fn has_nested_subclass(&self, ctx: &CheckContext, node: &CstNode, class_name: &str) -> bool {
        let Some(body) = node.child_by_field_name("body") else {
            return false;
        };

        // Look for nested classes that extend this class
        for child in body.children() {
            if child.kind() == "class_declaration"
                && let Some(superclass) = child.child_by_field_name("superclass")
            {
                for sc_child in superclass.children() {
                    let type_text = &ctx.source()[sc_child.range()];
                    if type_text == class_name {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Check if any anonymous class in the entire file extends this class.
    /// Anonymous classes are created via `new ClassName() { ... }` with a class body.
    fn has_anonymous_subclass_in_file(
        &self,
        ctx: &CheckContext,
        node: &CstNode,
        class_name: &str,
    ) -> bool {
        // Walk up to find the root (program) node
        let mut root = *node;
        while let Some(parent) = root.parent() {
            root = parent;
        }

        // Get the byte offset of our target class
        let target_offset = node.range().start();

        // Now search the entire tree for anonymous class instantiations
        self.find_anonymous_subclass_recursive(ctx, &root, node, class_name, target_offset)
    }

    /// Recursively search for anonymous class instantiations that extend the given class.
    /// Only counts if our target class is the one in scope (not shadowed by a closer class).
    fn find_anonymous_subclass_recursive(
        &self,
        ctx: &CheckContext,
        node: &CstNode,
        target_class: &CstNode,
        class_name: &str,
        target_offset: lintal_text_size::TextSize,
    ) -> bool {
        // Check if this is an object_creation_expression with a class_body (anonymous class)
        if node.kind() == "object_creation_expression" {
            // Check if it has a class_body child (making it anonymous)
            let has_class_body = node.children().any(|c| c.kind() == "class_body");

            if has_class_body {
                // Check if this is a chained constructor call (like "new A().new B()")
                if self.is_chained_constructor_call(node) {
                    // For chained calls, match the full type path
                    if self.chained_call_matches_target(ctx, node, target_class, class_name) {
                        return true;
                    }
                } else if let Some(type_text) = self.get_simple_type_name(ctx, node)
                    && type_text == class_name
                    && self.is_target_in_scope_at(ctx, node, class_name, target_offset)
                {
                    // Simple constructor call: "new Foo() {}"
                    return true;
                }
            }
        }

        // Recurse into children
        for child in node.children() {
            if self.find_anonymous_subclass_recursive(
                ctx,
                &child,
                target_class,
                class_name,
                target_offset,
            ) {
                return true;
            }
        }

        false
    }

    /// Check if a chained constructor call (new A().new B().new C()) extends our target class.
    fn chained_call_matches_target(
        &self,
        ctx: &CheckContext,
        node: &CstNode,
        target_class: &CstNode,
        target_name: &str,
    ) -> bool {
        // Get the type chain from the chained call: [A, B, C]
        let call_chain = self.get_chained_type_names(ctx, node);
        if call_chain.is_empty() {
            return false;
        }

        // The last name in chain should match target's name
        if call_chain.last() != Some(&target_name) {
            return false;
        }

        // Get the target's containing class chain
        let target_chain = self.get_class_nesting_chain(ctx, target_class);

        // Build full target chain including the target itself
        let target_full_chain: Vec<&str> = target_chain
            .iter()
            .map(|s| s.as_str())
            .chain(std::iter::once(target_name))
            .collect();

        if call_chain.len() > target_full_chain.len() {
            return false;
        }

        // Check if call_chain matches the suffix of target_full_chain (by name)
        let offset = target_full_chain.len() - call_chain.len();
        for (i, call_name) in call_chain.iter().enumerate() {
            if target_full_chain[offset + i] != *call_name {
                return false;
            }
        }

        // CRITICAL: Verify the root of call_chain actually resolves to the
        // corresponding class in target's chain, not a different class with the same name.
        if !call_chain.is_empty() {
            let root_type_name = call_chain[0];

            // Find what class the root type resolves to from the anonymous location
            if let Some(resolved_offset) = self.resolve_type_name(ctx, node, root_type_name) {
                // Find the expected class.
                // target_full_chain[offset] is the root of the call chain in the target's hierarchy.
                // That class is at depth (target_full_chain.len() - 2 - offset) from the target.
                // target_full_chain.len() - 1 is the target itself (not an ancestor).
                // target_full_chain.len() - 2 is the immediate parent (depth 0).
                let depth = target_full_chain
                    .len()
                    .saturating_sub(2)
                    .saturating_sub(offset);
                if let Some(expected_offset) = self.get_ancestor_class_offset(target_class, depth) {
                    // The resolved class must be the same as the expected
                    return resolved_offset == expected_offset;
                }
            }

            // Couldn't verify - assume no match to avoid false exclusions
            return false;
        }

        true
    }

    /// Resolve a type name to its class declaration offset from a given location.
    fn resolve_type_name(
        &self,
        ctx: &CheckContext,
        location: &CstNode,
        type_name: &str,
    ) -> Option<lintal_text_size::TextSize> {
        // Walk up through scopes looking for a class with this name
        let mut current = *location;

        while let Some(parent) = current.parent() {
            if parent.kind() == "class_body" || parent.kind() == "program" {
                for sibling in parent.children() {
                    if sibling.kind() == "class_declaration"
                        && let Some(name_node) = sibling.child_by_field_name("name")
                    {
                        let name = &ctx.source()[name_node.range()];
                        if name == type_name {
                            return Some(sibling.range().start());
                        }
                    }
                }
            }
            current = parent;
        }

        None
    }

    /// Get the offset of the ancestor class at a certain depth from the target.
    /// depth 0 = target's immediate parent, depth 1 = grandparent, etc.
    fn get_ancestor_class_offset(
        &self,
        target_class: &CstNode,
        depth: usize,
    ) -> Option<lintal_text_size::TextSize> {
        let mut current = target_class.parent();
        let mut class_count = 0;

        while let Some(parent) = current {
            if parent.kind() == "class_body"
                && let Some(class_decl) = parent.parent()
                && class_decl.kind() == "class_declaration"
            {
                if class_count == depth {
                    return Some(class_decl.range().start());
                }
                class_count += 1;
                current = class_decl.parent();
                continue;
            }
            current = parent.parent();
        }

        None
    }

    /// Get the chain of type names from a chained constructor call.
    /// For "new A().new B().new C()", returns ["A", "B", "C"].
    fn get_chained_type_names<'a>(&self, ctx: &'a CheckContext, node: &CstNode) -> Vec<&'a str> {
        let mut chain = Vec::new();

        // Check for nested object_creation_expression first
        for child in node.children() {
            if child.kind() == "object_creation_expression" {
                chain.extend(self.get_chained_type_names(ctx, &child));
            }
        }

        // Add this node's type
        if let Some(type_name) = self.get_simple_type_name(ctx, node) {
            chain.push(type_name);
        }

        chain
    }

    /// Get the chain of containing class names for a class.
    /// For a class "Inner" inside "Middle" inside "Outer", returns ["Outer", "Middle"].
    fn get_class_nesting_chain(&self, ctx: &CheckContext, node: &CstNode) -> Vec<String> {
        let mut chain = Vec::new();
        let mut current = node.parent();

        while let Some(parent) = current {
            if parent.kind() == "class_body"
                && let Some(class_decl) = parent.parent()
                && class_decl.kind() == "class_declaration"
            {
                if let Some(name_node) = class_decl.child_by_field_name("name") {
                    let name = ctx.source()[name_node.range()].to_string();
                    chain.push(name);
                }
                current = class_decl.parent();
                continue;
            }
            current = parent.parent();
        }

        chain.reverse();
        chain
    }

    /// Check if this is a chained constructor call (e.g., "new A().new B()").
    fn is_chained_constructor_call(&self, node: &CstNode) -> bool {
        // A chained call has an object_creation_expression as a child (the preceding "new A()")
        for child in node.children() {
            if child.kind() == "object_creation_expression" {
                return true;
            }
        }
        false
    }

    /// Get the type name for a simple (non-chained) constructor call.
    fn get_simple_type_name<'a>(&self, ctx: &'a CheckContext, node: &CstNode) -> Option<&'a str> {
        for child in node.children() {
            match child.kind() {
                "type_identifier" => {
                    return Some(&ctx.source()[child.range()]);
                }
                "scoped_type_identifier" => {
                    // For qualified types like "com.example.Outer", get the rightmost identifier
                    return self.get_rightmost_type_identifier(ctx, &child);
                }
                "generic_type" => {
                    // For types like "List<String>"
                    for gc in child.children() {
                        if gc.kind() == "type_identifier" {
                            return Some(&ctx.source()[gc.range()]);
                        }
                        if gc.kind() == "scoped_type_identifier" {
                            return self.get_rightmost_type_identifier(ctx, &gc);
                        }
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// Get the rightmost type_identifier from a scoped_type_identifier.
    /// For "com.example.pkg.Outer", returns "Outer".
    fn get_rightmost_type_identifier<'a>(
        &self,
        ctx: &'a CheckContext,
        node: &CstNode,
    ) -> Option<&'a str> {
        // Look for the last type_identifier child (rightmost in the qualified name)
        let mut last_ident: Option<&'a str> = None;
        for child in node.children() {
            if child.kind() == "type_identifier" {
                last_ident = Some(&ctx.source()[child.range()]);
            } else if child.kind() == "scoped_type_identifier" {
                // Recursively get from nested scoped_type_identifier
                if let Some(inner) = self.get_rightmost_type_identifier(ctx, &child) {
                    last_ident = Some(inner);
                }
            }
        }
        last_ident
    }

    /// Check if the target class (at target_offset) is in scope at the given location,
    /// i.e., when Java resolves the class_name at this location, it would resolve to our target.
    ///
    /// Java name resolution: walk outward from current location, finding the first class
    /// with the matching name. That's what gets resolved.
    fn is_target_in_scope_at(
        &self,
        ctx: &CheckContext,
        location: &CstNode,
        class_name: &str,
        target_offset: lintal_text_size::TextSize,
    ) -> bool {
        // Resolve: starting from location, walk up through scopes.
        // At each scope (class_body, program), look for sibling class declarations with matching name.
        // The first match found is what would be resolved.

        let mut current = *location;

        while let Some(parent) = current.parent() {
            if parent.kind() == "class_body" || parent.kind() == "program" {
                // Look for class declarations at this level
                for sibling in parent.children() {
                    if sibling.kind() == "class_declaration"
                        && let Some(name_node) = sibling.child_by_field_name("name")
                    {
                        let name = &ctx.source()[name_node.range()];
                        if name == class_name {
                            // Found a class with matching name.
                            // This is what Java would resolve to.
                            let resolved_offset = sibling.range().start();
                            return resolved_offset == target_offset;
                        }
                    }
                }
            }
            current = parent;
        }

        // No class with matching name found in scope - shouldn't happen if we got here
        false
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
        let rule = FinalClass;

        let mut diagnostics = vec![];
        for node in TreeWalker::new(result.tree.root_node(), source) {
            diagnostics.extend(rule.check(&ctx, &node));
        }
        diagnostics
    }

    #[test]
    fn test_class_with_private_constructor() {
        let source = r#"
class Singleton {
    private Singleton() {}
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(
            diagnostics.len(),
            1,
            "Class with private constructor should be flagged"
        );
    }

    #[test]
    fn test_class_already_final() {
        let source = r#"
final class Singleton {
    private Singleton() {}
}
"#;
        let diagnostics = check_source(source);
        assert!(
            diagnostics.is_empty(),
            "Already final class should not be flagged"
        );
    }

    #[test]
    fn test_class_with_public_constructor() {
        let source = r#"
class Normal {
    public Normal() {}
}
"#;
        let diagnostics = check_source(source);
        assert!(
            diagnostics.is_empty(),
            "Class with public constructor should not be flagged"
        );
    }

    #[test]
    fn test_class_with_no_constructor() {
        let source = r#"
class Default {
    void method() {}
}
"#;
        let diagnostics = check_source(source);
        assert!(
            diagnostics.is_empty(),
            "Class with no constructor (implicit public) should not be flagged"
        );
    }

    #[test]
    fn test_abstract_class() {
        let source = r#"
abstract class Base {
    private Base() {}
}
"#;
        let diagnostics = check_source(source);
        assert!(
            diagnostics.is_empty(),
            "Abstract class should not be flagged"
        );
    }

    #[test]
    fn test_class_with_mixed_constructors() {
        let source = r#"
class Mixed {
    private Mixed(int x) {}
    public Mixed() {}
}
"#;
        let diagnostics = check_source(source);
        assert!(
            diagnostics.is_empty(),
            "Class with mixed constructor visibility should not be flagged"
        );
    }

    #[test]
    fn test_class_with_multiple_private_constructors() {
        let source = r#"
class AllPrivate {
    private AllPrivate() {}
    private AllPrivate(int x) {}
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(
            diagnostics.len(),
            1,
            "Class with all private constructors should be flagged"
        );
    }

    #[test]
    fn test_nested_class_extending_outer() {
        // This is a valid pattern - inner class can use outer's private constructor
        let source = r#"
class Outer {
    private Outer() {}

    static class Inner extends Outer {
        Inner() {
            super();
        }
    }
}
"#;
        let diagnostics = check_source(source);
        // The outer class should not be flagged because it has a nested subclass
        // We check that we don't flag line 2 which is where Outer is defined
        let outer_line = 2;
        let line_index = lintal_source_file::LineIndex::from_source_text(source);
        let source_code = lintal_source_file::SourceCode::new(source, &line_index);
        let flagged_outer = diagnostics
            .iter()
            .any(|d| source_code.line_column(d.range.start()).line.get() == outer_line);
        assert!(
            !flagged_outer,
            "Outer class with nested subclass should not be flagged"
        );
    }

    #[test]
    fn test_utility_class_pattern() {
        let source = r#"
public class Utils {
    private Utils() {
        throw new UnsupportedOperationException();
    }

    public static void helper() {}
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(
            diagnostics.len(),
            1,
            "Utility class with private constructor should be flagged"
        );
    }

    #[test]
    fn test_protected_constructor_not_flagged() {
        let source = r#"
class Base {
    protected Base() {}
}
"#;
        let diagnostics = check_source(source);
        assert!(
            diagnostics.is_empty(),
            "Class with protected constructor should not be flagged"
        );
    }

    #[test]
    fn test_package_private_constructor_not_flagged() {
        let source = r#"
class PackageAccess {
    PackageAccess() {}  // package-private (no modifier)
}
"#;
        let diagnostics = check_source(source);
        assert!(
            diagnostics.is_empty(),
            "Class with package-private constructor should not be flagged"
        );
    }
}
