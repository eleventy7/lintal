//! Dumps the tree-sitter AST for a Java file.
//!
//! Usage:
//!   cat MyClass.java | cargo run --bin dump_java_ast
//!   cargo run --bin dump_java_ast < MyClass.java
//!
//! Or after building:
//!   cat MyClass.java | ./target/release/dump_java_ast

use lintal_java_parser::JavaParser;
use std::io::{self, Read};

fn main() {
    // Read source from stdin
    let mut source = String::new();
    if let Err(e) = io::stdin().read_to_string(&mut source) {
        eprintln!("Error reading stdin: {}", e);
        std::process::exit(1);
    }

    if source.trim().is_empty() {
        eprintln!("Error: No input provided. Pipe a Java file to stdin.");
        eprintln!("Usage: cat MyClass.java | dump_java_ast");
        std::process::exit(1);
    }

    // Parse the source
    let mut parser = JavaParser::new();
    let Some(result) = parser.parse(&source) else {
        eprintln!("Error: Failed to parse Java source");
        std::process::exit(1);
    };

    // Print the AST
    print_tree(result.tree.root_node(), &source, 0);
}

fn print_tree(node: tree_sitter::Node, source: &str, depth: usize) {
    let indent = "  ".repeat(depth);
    let start = node.start_position();
    let end = node.end_position();

    // Get a preview of the node text (first 40 chars, no newlines)
    let text: String = node
        .utf8_text(source.as_bytes())
        .unwrap_or("")
        .chars()
        .take(40)
        .map(|c| if c == '\n' { '↵' } else { c })
        .collect();

    // Format: kind [start_row:start_col - end_row:end_col] "text preview..."
    if node.child_count() == 0 {
        // Leaf node - show text
        println!(
            "{}{} [{}:{}-{}:{}] \"{}\"",
            indent,
            node.kind(),
            start.row + 1,
            start.column,
            end.row + 1,
            end.column,
            text
        );
    } else {
        // Non-leaf - just show kind and position
        println!(
            "{}{} [{}:{}-{}:{}]",
            indent,
            node.kind(),
            start.row + 1,
            start.column,
            end.row + 1,
            end.column
        );
    }

    // Recursively print children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        print_tree(child, source, depth + 1);
    }
}
