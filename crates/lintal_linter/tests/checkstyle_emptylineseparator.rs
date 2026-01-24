//! Checkstyle compatibility tests for EmptyLineSeparator rule.
//!
//! Tests all 49 checkstyle test fixtures and reports compatibility stats.

mod checkstyle_repo;

use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::rules::EmptyLineSeparator;
use lintal_linter::rules::whitespace::empty_line_separator::EmptyLineSeparatorToken;
use lintal_linter::{CheckContext, Rule};
use lintal_source_file::{LineIndex, SourceCode};
use regex::Regex;
use std::collections::HashSet;

/// Configuration parsed from fixture header.
#[derive(Debug, Clone)]
struct FixtureConfig {
    allow_no_empty_line_between_fields: bool,
    allow_multiple_empty_lines: bool,
    allow_multiple_empty_lines_inside_class_members: bool,
    tokens: HashSet<EmptyLineSeparatorToken>,
}

impl Default for FixtureConfig {
    fn default() -> Self {
        Self {
            allow_no_empty_line_between_fields: false,
            allow_multiple_empty_lines: true,
            allow_multiple_empty_lines_inside_class_members: true,
            tokens: EmptyLineSeparatorToken::default_tokens(),
        }
    }
}

/// Check if a config line has value true (handles both "= true" and 'value="true"')
fn has_value_true(line: &str) -> bool {
    line.contains("= true") || line.contains("value=\"true\"")
}

/// Check if a config line has value false (handles both "= false" and 'value="false"')
fn has_value_false(line: &str) -> bool {
    line.contains("= false") || line.contains("value=\"false\"")
}

fn parse_fixture_config(source: &str) -> FixtureConfig {
    let mut config = FixtureConfig::default();

    // Parse allowNoEmptyLineBetweenFields
    if let Some(line) = source
        .lines()
        .find(|l| l.contains("allowNoEmptyLineBetweenFields"))
        && has_value_true(line)
    {
        config.allow_no_empty_line_between_fields = true;
    }

    // Parse allowMultipleEmptyLines (careful: don't match allowMultipleEmptyLinesInsideClassMembers)
    for line in source.lines() {
        if line.contains("allowMultipleEmptyLines")
            && !line.contains("allowMultipleEmptyLinesInsideClassMembers")
        {
            if has_value_false(line) {
                config.allow_multiple_empty_lines = false;
            }
            break;
        }
    }

    // Parse allowMultipleEmptyLinesInsideClassMembers
    if let Some(line) = source
        .lines()
        .find(|l| l.contains("allowMultipleEmptyLinesInsideClassMembers"))
        && has_value_false(line)
    {
        config.allow_multiple_empty_lines_inside_class_members = false;
    }

    // Parse tokens (look for tokens = line)
    let tokens_line = source
        .lines()
        .find(|l| l.starts_with("tokens") && l.contains("="));
    if let Some(line) = tokens_line
        && !line.contains("(default)")
    {
        let mut tokens = HashSet::new();
        // Parse token list - may span multiple lines
        let start_idx = source.find("tokens").unwrap_or(0);
        let end_idx = source[start_idx..].find("\n\n").unwrap_or(source.len()) + start_idx;
        let tokens_section = &source[start_idx..end_idx];

        for token in [
            "PACKAGE_DEF",
            "IMPORT",
            "STATIC_IMPORT",
            "CLASS_DEF",
            "INTERFACE_DEF",
            "ENUM_DEF",
            "STATIC_INIT",
            "INSTANCE_INIT",
            "METHOD_DEF",
            "CTOR_DEF",
            "VARIABLE_DEF",
            "RECORD_DEF",
            "COMPACT_CTOR_DEF",
        ] {
            if tokens_section.contains(token)
                && let Some(t) = token_from_str(token)
            {
                tokens.insert(t);
            }
        }
        if !tokens.is_empty() {
            config.tokens = tokens;
        }
    }

    config
}

fn token_from_str(s: &str) -> Option<EmptyLineSeparatorToken> {
    match s {
        "PACKAGE_DEF" => Some(EmptyLineSeparatorToken::PackageDef),
        "IMPORT" => Some(EmptyLineSeparatorToken::Import),
        "STATIC_IMPORT" => Some(EmptyLineSeparatorToken::StaticImport),
        "CLASS_DEF" => Some(EmptyLineSeparatorToken::ClassDef),
        "INTERFACE_DEF" => Some(EmptyLineSeparatorToken::InterfaceDef),
        "ENUM_DEF" => Some(EmptyLineSeparatorToken::EnumDef),
        "STATIC_INIT" => Some(EmptyLineSeparatorToken::StaticInit),
        "INSTANCE_INIT" => Some(EmptyLineSeparatorToken::InstanceInit),
        "METHOD_DEF" => Some(EmptyLineSeparatorToken::MethodDef),
        "CTOR_DEF" => Some(EmptyLineSeparatorToken::CtorDef),
        "VARIABLE_DEF" => Some(EmptyLineSeparatorToken::VariableDef),
        "RECORD_DEF" => Some(EmptyLineSeparatorToken::RecordDef),
        "COMPACT_CTOR_DEF" => Some(EmptyLineSeparatorToken::CompactCtorDef),
        _ => None,
    }
}

/// Expected violation from test file comments.
#[derive(Debug, Clone)]
struct ExpectedViolation {
    line: usize,
    message_pattern: String,
}

/// Actual violation from our implementation.
#[derive(Debug, Clone)]
struct ActualViolation {
    line: usize,
    message: String,
}

/// Parse expected violations from checkstyle test file comments.
fn parse_expected_violations(source: &str) -> Vec<ExpectedViolation> {
    let mut violations = vec![];

    // Patterns for violation comments
    let inline_re = Regex::new(r"//\s*violation\s+'([^']*)'").unwrap();
    let inline_re2 = Regex::new(r"//\s*violation\s+''([^']*)'").unwrap();
    let below_re = Regex::new(r"//\s*violation\s+below").unwrap();
    let above_re = Regex::new(r"//\s*violation\s+above").unwrap();
    let n_lines_above_re = Regex::new(r"//\s*violation\s+(\d+)\s+lines?\s+above").unwrap();
    // Double-quoted message pattern (e.g., "message")
    let double_quote_msg_re = Regex::new(r#"//\s*violation.*"([^"]*)""#).unwrap();
    // Multiple violations pattern (e.g., "// 2 violations")
    let n_violations_re = Regex::new(r"//\s*(\d+)\s+violations?").unwrap();

    for (i, line) in source.lines().enumerate() {
        let line_num = i + 1;

        // Check for "violation N lines above" pattern
        if let Some(caps) = n_lines_above_re.captures(line) {
            let offset: usize = caps.get(1).unwrap().as_str().parse().unwrap_or(1);
            let msg = if let Some(msg_caps) = double_quote_msg_re.captures(line) {
                msg_caps
                    .get(1)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default()
            } else if let Some(msg_caps) = inline_re.captures(line) {
                msg_caps
                    .get(1)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default()
            } else {
                String::new()
            };
            violations.push(ExpectedViolation {
                line: line_num.saturating_sub(offset),
                message_pattern: msg,
            });
            continue;
        }

        // Check for "violation above this line" pattern - different from "violation above"
        // "violation above this line" means the violation is ON this line, describing that the empty
        // lines are above it. "violation above" means the violation is 1 line above.
        if line.contains("violation above this line") {
            violations.push(ExpectedViolation {
                line: line_num,
                message_pattern: String::new(),
            });
            continue;
        }

        // Check for "violation above" pattern (simple, 1 line above)
        if above_re.is_match(line) {
            // Extract message if present
            let msg = if let Some(msg_caps) = double_quote_msg_re.captures(line) {
                msg_caps
                    .get(1)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default()
            } else if let Some(caps) = inline_re.captures(line) {
                caps.get(1)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default()
            } else if let Some(caps) = inline_re2.captures(line) {
                caps.get(1)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default()
            } else {
                String::new()
            };
            violations.push(ExpectedViolation {
                line: line_num - 1,
                message_pattern: msg,
            });
            continue;
        }

        // Check for "violation below" pattern
        if below_re.is_match(line) {
            // Extract message if present
            let msg = if let Some(caps) = inline_re.captures(line) {
                caps.get(1)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default()
            } else {
                String::new()
            };
            violations.push(ExpectedViolation {
                line: line_num + 1,
                message_pattern: msg,
            });
            continue;
        }

        // Check for inline violation with double quote pattern ''X'
        if let Some(caps) = inline_re2.captures(line) {
            let msg = caps
                .get(1)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            violations.push(ExpectedViolation {
                line: line_num,
                message_pattern: msg,
            });
            continue;
        }

        // Check for inline violation with single quote pattern 'X'
        if let Some(caps) = inline_re.captures(line) {
            let msg = caps
                .get(1)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            violations.push(ExpectedViolation {
                line: line_num,
                message_pattern: msg,
            });
            continue;
        }

        // Check for inline violation with double quote pattern "X"
        // This handles cases like: // violation "'CLASS_DEF' should be separated..."
        if let Some(caps) = double_quote_msg_re.captures(line) {
            let msg = caps
                .get(1)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            violations.push(ExpectedViolation {
                line: line_num,
                message_pattern: msg,
            });
            continue;
        }

        // Check for "N violations" pattern (e.g., "// 2 violations")
        if let Some(caps) = n_violations_re.captures(line) {
            let count: usize = caps.get(1).unwrap().as_str().parse().unwrap_or(1);
            for _ in 0..count {
                violations.push(ExpectedViolation {
                    line: line_num,
                    message_pattern: String::new(),
                });
            }
        }
    }

    violations
}

/// Run EmptyLineSeparator rule on source and collect violations.
fn check_empty_line_separator(source: &str, config: &FixtureConfig) -> Vec<ActualViolation> {
    let mut parser = JavaParser::new();
    let Some(result) = parser.parse(source) else {
        return vec![];
    };

    let rule = EmptyLineSeparator {
        allow_no_empty_line_between_fields: config.allow_no_empty_line_between_fields,
        allow_multiple_empty_lines: config.allow_multiple_empty_lines,
        allow_multiple_empty_lines_inside_class_members: config
            .allow_multiple_empty_lines_inside_class_members,
        tokens: config.tokens.clone(),
    };

    let ctx = CheckContext::new(source);
    let line_index = LineIndex::from_source_text(source);
    let source_code = SourceCode::new(source, &line_index);

    let mut violations = vec![];

    for node in TreeWalker::new(result.tree.root_node(), source) {
        let diagnostics = rule.check(&ctx, &node);
        for diagnostic in diagnostics {
            let loc = source_code.line_column(diagnostic.range.start());
            violations.push(ActualViolation {
                line: loc.line.get(),
                message: diagnostic.kind.body.clone(),
            });
        }
    }

    violations
}

/// Load a checkstyle test input file.
fn load_fixture(file_name: &str) -> Option<String> {
    let path = checkstyle_repo::whitespace_test_input("emptylineseparator", file_name)?;
    std::fs::read_to_string(&path).ok()
}

/// Test result for a single fixture.
#[derive(Debug, Default)]
struct FixtureResult {
    expected: usize,
    found: usize,
    correct: usize,
    missing: usize,
    false_positives: usize,
}

/// Run a single fixture and compare results.
fn test_fixture(file_name: &str) -> Option<FixtureResult> {
    let source = load_fixture(file_name)?;
    let config = parse_fixture_config(&source);
    let expected = parse_expected_violations(&source);
    let actual = check_empty_line_separator(&source, &config);

    // Match violations by line number, accounting for multiple violations on the same line.
    let mut expected_counts = std::collections::HashMap::new();
    for v in &expected {
        *expected_counts.entry(v.line).or_insert(0usize) += 1;
    }
    let mut actual_counts = std::collections::HashMap::new();
    for v in &actual {
        *actual_counts.entry(v.line).or_insert(0usize) += 1;
    }

    let mut correct = 0usize;
    for (line, expected_count) in &expected_counts {
        let actual_count = actual_counts.get(line).copied().unwrap_or(0);
        correct += (*expected_count).min(actual_count);
    }
    let expected_total: usize = expected_counts.values().sum();
    let actual_total: usize = actual_counts.values().sum();
    let missing: usize = expected_total.saturating_sub(correct);
    let false_positives: usize = actual_total.saturating_sub(correct);

    Some(FixtureResult {
        expected: expected.len(),
        found: actual.len(),
        correct,
        missing,
        false_positives,
    })
}

/// List all fixture files.
fn list_fixtures() -> Vec<String> {
    let Some(base_path) = checkstyle_repo::whitespace_test_input(
        "emptylineseparator",
        "InputEmptyLineSeparator.java",
    ) else {
        return vec![];
    };

    let dir = base_path.parent().unwrap();
    let mut fixtures = vec![];

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str()
                && name.ends_with(".java")
            {
                fixtures.push(name.to_string());
            }
        }
    }

    fixtures.sort();
    fixtures
}

// =============================================================================
// Main compatibility test
// =============================================================================

#[test]
fn test_all_fixtures() {
    let fixtures = list_fixtures();
    if fixtures.is_empty() {
        eprintln!("Skipping: checkstyle repo not available");
        return;
    }

    println!("\n=== EmptyLineSeparator Compatibility Test ===\n");

    let mut total = FixtureResult::default();
    let mut fixture_results: Vec<(String, FixtureResult)> = vec![];

    for fixture in &fixtures {
        if let Some(result) = test_fixture(fixture) {
            println!(
                "{}: Expected={}, Found={}, Correct={}, Missing={}, FP={}",
                fixture,
                result.expected,
                result.found,
                result.correct,
                result.missing,
                result.false_positives
            );

            total.expected += result.expected;
            total.found += result.found;
            total.correct += result.correct;
            total.missing += result.missing;
            total.false_positives += result.false_positives;

            fixture_results.push((fixture.clone(), result));
        }
    }

    println!("\n=== TOTALS ===");
    println!("Fixtures tested: {}", fixture_results.len());
    println!("Expected violations: {}", total.expected);
    println!("Found violations: {}", total.found);
    println!("Correct matches: {}", total.correct);
    println!("Missing (false negatives): {}", total.missing);
    println!("False positives: {}", total.false_positives);

    if total.expected > 0 {
        let detection_rate = (total.correct as f64 / total.expected as f64) * 100.0;
        println!("Detection rate: {:.1}%", detection_rate);
    }

    // Print fixtures with issues for debugging
    println!("\n=== Fixtures with Missing Violations ===");
    for (name, result) in &fixture_results {
        if result.missing > 0 {
            println!("  {} - {} missing", name, result.missing);
        }
    }

    println!("\n=== Fixtures with False Positives ===");
    for (name, result) in &fixture_results {
        if result.false_positives > 0 {
            println!("  {} - {} false positives", name, result.false_positives);
        }
    }
}

// =============================================================================
// Detailed test for specific fixture
// =============================================================================

#[test]
fn test_input_empty_line_separator_detailed() {
    let fixture = std::env::var("FIXTURE")
        .ok()
        .unwrap_or_else(|| "InputEmptyLineSeparator.java".to_string());
    let Some(source) = load_fixture(&fixture) else {
        eprintln!("Skipping: checkstyle repo not available");
        return;
    };

    let config = parse_fixture_config(&source);
    println!("Fixture: {}", fixture);
    println!("Config: {:?}", config);

    let expected = parse_expected_violations(&source);
    println!("\nExpected violations ({}):", expected.len());
    for v in &expected {
        println!("  Line {}: {}", v.line, v.message_pattern);
    }

    let actual = check_empty_line_separator(&source, &config);
    println!("\nActual violations ({}):", actual.len());
    for v in &actual {
        println!("  Line {}: {}", v.line, v.message);
    }

    let expected_lines: HashSet<usize> = expected.iter().map(|v| v.line).collect();
    let actual_lines: HashSet<usize> = actual.iter().map(|v| v.line).collect();

    let missing: Vec<_> = expected_lines.difference(&actual_lines).collect();
    let false_pos: Vec<_> = actual_lines.difference(&expected_lines).collect();

    if !missing.is_empty() {
        println!("\nMissing (lines): {:?}", missing);
    }
    if !false_pos.is_empty() {
        println!("\nFalse positives (lines): {:?}", false_pos);
    }
}

#[test]
fn test_multiple_empty_lines_detailed() {
    let Some(source) = load_fixture("InputEmptyLineSeparatorMultipleEmptyLines.java") else {
        eprintln!("Skipping: checkstyle repo not available");
        return;
    };

    let config = parse_fixture_config(&source);
    println!(
        "Config: allow_multiple_empty_lines={}",
        config.allow_multiple_empty_lines
    );

    let expected = parse_expected_violations(&source);
    println!("\nExpected violations ({}):", expected.len());
    for v in &expected {
        println!("  Line {}: {}", v.line, v.message_pattern);
    }

    let actual = check_empty_line_separator(&source, &config);
    println!("\nActual violations ({}):", actual.len());
    for v in &actual {
        println!("  Line {}: {}", v.line, v.message);
    }
}

#[test]
fn test_multiple_empty_lines_inside_detailed() {
    let Some(source) = load_fixture("InputEmptyLineSeparatorMultipleEmptyLinesInside.java") else {
        eprintln!("Skipping: checkstyle repo not available");
        return;
    };

    let config = parse_fixture_config(&source);
    println!(
        "Config: allow_multiple_empty_lines_inside_class_members={}",
        config.allow_multiple_empty_lines_inside_class_members
    );

    let expected = parse_expected_violations(&source);
    println!("\nExpected violations ({}):", expected.len());
    for v in &expected {
        println!("  Line {}: {}", v.line, v.message_pattern);
    }

    let actual = check_empty_line_separator(&source, &config);
    println!("\nActual violations ({}):", actual.len());
    for v in &actual {
        println!("  Line {}: {}", v.line, v.message);
    }
}

#[test]
fn test_multiple_inside_details() {
    let Some(source) = load_fixture("InputEmptyLineSeparatorMultipleEmptyLinesInside.java") else {
        eprintln!("Skipping: checkstyle repo not available");
        return;
    };

    let config = parse_fixture_config(&source);
    println!(
        "Config: allow_multiple_empty_lines={}, allow_multiple_empty_lines_inside={}",
        config.allow_multiple_empty_lines, config.allow_multiple_empty_lines_inside_class_members
    );

    let expected = parse_expected_violations(&source);
    println!("\nExpected violations ({}):", expected.len());
    for v in &expected {
        println!("  Line {}: {}", v.line, v.message_pattern);
    }

    let actual = check_empty_line_separator(&source, &config);
    println!("\nActual violations ({}):", actual.len());
    for v in &actual {
        println!("  Line {}: {}", v.line, v.message);
    }

    let expected_lines: HashSet<usize> = expected.iter().map(|v| v.line).collect();
    let actual_lines: HashSet<usize> = actual.iter().map(|v| v.line).collect();

    let mut missing: Vec<_> = expected_lines.difference(&actual_lines).copied().collect();
    missing.sort();
    let mut false_pos: Vec<_> = actual_lines.difference(&expected_lines).copied().collect();
    false_pos.sort();

    if !missing.is_empty() {
        println!("\nMissing (lines): {:?}", missing);
    }
    if !false_pos.is_empty() {
        println!("\nFalse positives (lines): {:?}", false_pos);
    }
}

#[test]
fn test_postfix_corner_cases_detailed() {
    let Some(source) = load_fixture("InputEmptyLineSeparatorPostFixCornerCases.java") else {
        eprintln!("Skipping: checkstyle repo not available");
        return;
    };

    let config = parse_fixture_config(&source);
    println!(
        "Config: allow_multiple_empty_lines={}, allow_multiple_empty_lines_inside={}",
        config.allow_multiple_empty_lines, config.allow_multiple_empty_lines_inside_class_members
    );

    let expected = parse_expected_violations(&source);
    println!("\nExpected violations ({}):", expected.len());
    for v in &expected {
        println!("  Line {}: {}", v.line, v.message_pattern);
    }

    let actual = check_empty_line_separator(&source, &config);
    println!("\nActual violations ({}):", actual.len());
    for v in &actual {
        println!("  Line {}: {}", v.line, v.message);
    }

    let expected_lines: HashSet<usize> = expected.iter().map(|v| v.line).collect();
    let actual_lines: HashSet<usize> = actual.iter().map(|v| v.line).collect();

    let mut missing: Vec<_> = expected_lines.difference(&actual_lines).copied().collect();
    missing.sort();
    let mut false_pos: Vec<_> = actual_lines.difference(&expected_lines).copied().collect();
    false_pos.sort();

    if !missing.is_empty() {
        println!("\nMissing (lines): {:?}", missing);
    }
    if !false_pos.is_empty() {
        println!("\nFalse positives (lines): {:?}", false_pos);
    }
}

#[test]
fn test_with_comments_detailed() {
    let Some(source) = load_fixture("InputEmptyLineSeparatorWithComments.java") else {
        eprintln!("Skipping: checkstyle repo not available");
        return;
    };

    let config = parse_fixture_config(&source);
    println!(
        "Config: allow_multiple_empty_lines={}, allow_no_empty_line_between_fields={}",
        config.allow_multiple_empty_lines, config.allow_no_empty_line_between_fields
    );

    let expected = parse_expected_violations(&source);
    println!("\nExpected violations ({}):", expected.len());
    for v in &expected {
        println!("  Line {}: {}", v.line, v.message_pattern);
    }

    let actual = check_empty_line_separator(&source, &config);
    println!("\nActual violations ({}):", actual.len());
    for v in &actual {
        println!("  Line {}: {}", v.line, v.message);
    }

    let expected_lines: HashSet<usize> = expected.iter().map(|v| v.line).collect();
    let actual_lines: HashSet<usize> = actual.iter().map(|v| v.line).collect();

    let mut missing: Vec<_> = expected_lines.difference(&actual_lines).copied().collect();
    missing.sort();
    let mut false_pos: Vec<_> = actual_lines.difference(&expected_lines).copied().collect();
    false_pos.sort();

    if !missing.is_empty() {
        println!("\nMissing (lines): {:?}", missing);
    }
    if !false_pos.is_empty() {
        println!("\nFalse positives (lines): {:?}", false_pos);
    }
}

#[test]
fn test_with_javadoc_detailed() {
    let Some(source) = load_fixture("InputEmptyLineSeparatorWithJavadoc.java") else {
        eprintln!("Skipping: checkstyle repo not available");
        return;
    };

    let config = parse_fixture_config(&source);
    println!(
        "Config: allow_multiple_empty_lines={}, allow_no_empty_line_between_fields={}",
        config.allow_multiple_empty_lines, config.allow_no_empty_line_between_fields
    );

    let expected = parse_expected_violations(&source);
    println!("\nExpected violations ({}):", expected.len());
    for v in &expected {
        println!("  Line {}: {}", v.line, v.message_pattern);
    }

    let actual = check_empty_line_separator(&source, &config);
    println!("\nActual violations ({}):", actual.len());
    for v in &actual {
        println!("  Line {}: {}", v.line, v.message);
    }

    let expected_lines: HashSet<usize> = expected.iter().map(|v| v.line).collect();
    let actual_lines: HashSet<usize> = actual.iter().map(|v| v.line).collect();

    let mut missing: Vec<_> = expected_lines.difference(&actual_lines).copied().collect();
    missing.sort();
    let mut false_pos: Vec<_> = actual_lines.difference(&expected_lines).copied().collect();
    false_pos.sort();

    if !missing.is_empty() {
        println!("\nMissing (lines): {:?}", missing);
    }
    if !false_pos.is_empty() {
        println!("\nFalse positives (lines): {:?}", false_pos);
    }
}

#[test]
fn test_with_javadoc2_detailed() {
    let Some(source) = load_fixture("InputEmptyLineSeparatorWithJavadoc2.java") else {
        eprintln!("Skipping: checkstyle repo not available");
        return;
    };

    let config = parse_fixture_config(&source);
    println!(
        "Config: allow_multiple_empty_lines={}, allow_no_empty_line_between_fields={}",
        config.allow_multiple_empty_lines, config.allow_no_empty_line_between_fields
    );

    let expected = parse_expected_violations(&source);
    println!("\nExpected violations ({}):", expected.len());
    for v in &expected {
        println!("  Line {}: {}", v.line, v.message_pattern);
    }

    let actual = check_empty_line_separator(&source, &config);
    println!("\nActual violations ({}):", actual.len());
    for v in &actual {
        println!("  Line {}: {}", v.line, v.message);
    }

    let expected_lines: HashSet<usize> = expected.iter().map(|v| v.line).collect();
    let actual_lines: HashSet<usize> = actual.iter().map(|v| v.line).collect();

    let mut missing: Vec<_> = expected_lines.difference(&actual_lines).copied().collect();
    missing.sort();
    let mut false_pos: Vec<_> = actual_lines.difference(&expected_lines).copied().collect();
    false_pos.sort();

    if !missing.is_empty() {
        println!("\nMissing (lines): {:?}", missing);
    }
    if !false_pos.is_empty() {
        println!("\nFalse positives (lines): {:?}", false_pos);
    }
}

#[test]
fn test_with_emoji_detailed() {
    let Some(source) = load_fixture("InputEmptyLineSeparatorWithEmoji.java") else {
        eprintln!("Skipping: checkstyle repo not available");
        return;
    };

    let config = parse_fixture_config(&source);
    println!(
        "Config: allow_multiple_empty_lines={}, allow_multiple_empty_lines_inside={}",
        config.allow_multiple_empty_lines, config.allow_multiple_empty_lines_inside_class_members
    );

    let expected = parse_expected_violations(&source);
    println!("\nExpected violations ({}):", expected.len());
    for v in &expected {
        println!("  Line {}: {}", v.line, v.message_pattern);
    }

    let actual = check_empty_line_separator(&source, &config);
    println!("\nActual violations ({}):", actual.len());
    for v in &actual {
        println!("  Line {}: {}", v.line, v.message);
    }

    let expected_lines: HashSet<usize> = expected.iter().map(|v| v.line).collect();
    let actual_lines: HashSet<usize> = actual.iter().map(|v| v.line).collect();

    let mut missing: Vec<_> = expected_lines.difference(&actual_lines).copied().collect();
    missing.sort();
    let mut false_pos: Vec<_> = actual_lines.difference(&expected_lines).copied().collect();
    false_pos.sort();

    if !missing.is_empty() {
        println!("\nMissing (lines): {:?}", missing);
    }
    if !false_pos.is_empty() {
        println!("\nFalse positives (lines): {:?}", false_pos);
    }
}

// =============================================================================
// Basic unit tests
// =============================================================================

#[test]
fn test_basic_separation() {
    let source = r#"
class Test {
    void method1() {}
    void method2() {}
}
"#;
    let config = FixtureConfig::default();
    let violations = check_empty_line_separator(source, &config);

    assert_eq!(
        violations.len(),
        1,
        "method2 should need blank line before it"
    );
    assert!(violations[0].message.contains("METHOD_DEF"));
}

#[test]
fn test_with_blank_line() {
    let source = r#"
class Test {
    void method1() {}

    void method2() {}
}
"#;
    let config = FixtureConfig::default();
    let violations = check_empty_line_separator(source, &config);

    assert!(
        violations.is_empty(),
        "method2 has blank line, should be OK"
    );
}

#[test]
fn test_field_to_field_allowed() {
    let source = r#"
class Test {
    private int x;
    private int y;
}
"#;
    let config = FixtureConfig {
        allow_no_empty_line_between_fields: true,
        ..Default::default()
    };
    let violations = check_empty_line_separator(source, &config);

    assert!(
        violations.is_empty(),
        "fields without blank lines should be OK when allowNoEmptyLineBetweenFields=true"
    );
}

#[test]
fn test_recursive_detailed() {
    let Some(source) = load_fixture("InputEmptyLineSeparatorRecursive.java") else {
        eprintln!("Skipping: checkstyle repo not available");
        return;
    };

    let config = parse_fixture_config(&source);
    println!(
        "Config: allow_multiple_empty_lines={}, allow_multiple_empty_lines_inside={}",
        config.allow_multiple_empty_lines, config.allow_multiple_empty_lines_inside_class_members
    );

    let expected = parse_expected_violations(&source);
    println!("\nExpected violations ({}):", expected.len());
    for v in &expected {
        println!("  Line {}: {}", v.line, v.message_pattern);
    }

    let actual = check_empty_line_separator(&source, &config);
    println!("\nActual violations ({}):", actual.len());
    for v in &actual {
        println!("  Line {}: {}", v.line, v.message);
    }

    let expected_lines: HashSet<usize> = expected.iter().map(|v| v.line).collect();
    let actual_lines: HashSet<usize> = actual.iter().map(|v| v.line).collect();

    let mut missing: Vec<_> = expected_lines.difference(&actual_lines).copied().collect();
    missing.sort();
    let mut false_pos: Vec<_> = actual_lines.difference(&expected_lines).copied().collect();
    false_pos.sort();

    if !missing.is_empty() {
        println!("\nMissing (lines): {:?}", missing);
    }
    if !false_pos.is_empty() {
        println!("\nFalse positives (lines): {:?}", false_pos);
    }
}

#[test]
fn test_new_method_def_detailed() {
    let Some(source) = load_fixture("InputEmptyLineSeparatorNewMethodDef.java") else {
        eprintln!("Skipping: checkstyle repo not available");
        return;
    };

    let config = parse_fixture_config(&source);
    println!(
        "Config: allow_multiple_empty_lines={}, allow_multiple_empty_lines_inside={}",
        config.allow_multiple_empty_lines, config.allow_multiple_empty_lines_inside_class_members
    );

    let expected = parse_expected_violations(&source);
    println!("\nExpected violations ({}):", expected.len());
    for v in &expected {
        println!("  Line {}: {}", v.line, v.message_pattern);
    }

    let actual = check_empty_line_separator(&source, &config);
    println!("\nActual violations ({}):", actual.len());
    for v in &actual {
        println!("  Line {}: {}", v.line, v.message);
    }

    let expected_lines: HashSet<usize> = expected.iter().map(|v| v.line).collect();
    let actual_lines: HashSet<usize> = actual.iter().map(|v| v.line).collect();

    let mut missing: Vec<_> = expected_lines.difference(&actual_lines).copied().collect();
    missing.sort();
    let mut false_pos: Vec<_> = actual_lines.difference(&expected_lines).copied().collect();
    false_pos.sort();

    if !missing.is_empty() {
        println!("\nMissing (lines): {:?}", missing);
    }
    if !false_pos.is_empty() {
        println!("\nFalse positives (lines): {:?}", false_pos);
    }
}

// =============================================================================
// Parity tests for known edge-case fixtures
// =============================================================================

fn assert_fixture_parity(file_name: &str) {
    let Some(result) = test_fixture(file_name) else {
        eprintln!("Skipping: checkstyle repo not available");
        return;
    };

    assert_eq!(
        result.missing, 0,
        "{file_name}: expected no missing violations, found {}",
        result.missing
    );
    assert_eq!(
        result.false_positives, 0,
        "{file_name}: expected no false positives, found {}",
        result.false_positives
    );
}

#[test]
fn test_parity_class_package_separation() {
    assert_fixture_parity("InputEmptyLineSeparatorClassPackageSeparation.java");
}

#[test]
fn test_parity_interface_fields() {
    assert_fixture_parity("InputEmptyLineSeparatorInterfaceFields.java");
}

#[test]
fn test_parity_multiple_lines2() {
    assert_fixture_parity("InputEmptyLineSeparatorMultipleLines2.java");
}

#[test]
fn test_parity_multiple_import_empty_class() {
    assert_fixture_parity("InputEmptyLineSeparatorMultipleImportEmptyClass.java");
}

#[test]
fn test_parity_one_line() {
    assert_fixture_parity("InputEmptyLineSeparatorOneLine.java");
}

#[test]
fn test_parity_package_import_class_in_one_line() {
    assert_fixture_parity("InputEmptyLineSeparatorPackageImportClassInOneLine.java");
}

#[test]
fn test_parity_with_comments() {
    assert_fixture_parity("InputEmptyLineSeparatorWithComments.java");
}

#[test]
fn test_parity_with_emoji() {
    assert_fixture_parity("InputEmptyLineSeparatorWithEmoji.java");
}

#[test]
fn test_parity_with_javadoc() {
    assert_fixture_parity("InputEmptyLineSeparatorWithJavadoc.java");
}

#[test]
fn test_parity_single_line_comment_after_package() {
    assert_fixture_parity("InputEmptyLineSeparatorSingleLineCommentAfterPackage.java");
}
