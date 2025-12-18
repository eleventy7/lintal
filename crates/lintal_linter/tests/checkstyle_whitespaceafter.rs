//! WhitespaceAfter checkstyle compatibility tests.

mod checkstyle_repo;

/// Load a checkstyle whitespace test input file.
fn load_fixture(file_name: &str) -> Option<String> {
    let path = checkstyle_repo::whitespace_test_input("whitespaceafter", file_name)?;
    std::fs::read_to_string(&path).ok()
}

/// Extract expected violations from inline comments in test file.
/// Format: // violation '',' is not followed by whitespace'
fn extract_expected_violations(source: &str) -> Vec<(usize, String)> {
    let mut violations = vec![];
    for (line_num, line) in source.lines().enumerate() {
        if let Some(comment_start) = line.find("// violation") {
            let comment = &line[comment_start..];
            // Extract token from pattern: ''X' is not followed'
            if let Some(start) = comment.find("''") {
                let after_quote = &comment[start + 2..];
                if let Some(end) = after_quote.find("'") {
                    let token = after_quote[..end].to_string();
                    violations.push((line_num + 1, token)); // 1-indexed
                }
            }
        }
    }
    violations
}

// =============================================================================
// Test: testDefaultConfig
// File: InputWhitespaceAfterDefaultConfig.java
// =============================================================================

#[test]
fn test_whitespace_after_default_config() {
    let Some(source) = load_fixture("InputWhitespaceAfterDefaultConfig.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let expected = extract_expected_violations(&source);
    println!("Expected violations: {:?}", expected);

    // TODO: Implement WhitespaceAfter rule and uncomment
    // let violations = check_whitespace_after(&source);
    // verify_violations(&violations, &expected);

    // For now, just verify we can parse expected violations
    assert!(
        !expected.is_empty(),
        "Should find expected violations in comments"
    );
}
