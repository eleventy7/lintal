//! Test harness for checkstyle compatibility testing.
//!
//! Provides structured comparison of expected vs actual violations with
//! detailed reporting of exact matches, missing items (false negatives),
//! and false positives.

use std::collections::HashSet;

/// Result of comparing expected vs actual violations.
#[derive(Debug, Clone)]
pub struct TestResult {
    /// Lines where checkstyle expects violations
    pub expected: Vec<usize>,
    /// Lines where we found violations
    pub actual: Vec<usize>,
    /// Lines where both found violations (exact matches)
    pub exact_matches: Vec<usize>,
    /// False negatives: checkstyle found, we didn't
    pub missing: Vec<usize>,
    /// False positives: we found, checkstyle didn't
    pub false_positives: Vec<usize>,
}

impl TestResult {
    /// Compare expected and actual violation lines.
    pub fn compare(expected: Vec<usize>, actual: Vec<usize>) -> Self {
        let expected_set: HashSet<usize> = expected.iter().copied().collect();
        let actual_set: HashSet<usize> = actual.iter().copied().collect();

        let exact_matches: Vec<usize> = expected_set.intersection(&actual_set).copied().collect();

        let missing: Vec<usize> = expected_set.difference(&actual_set).copied().collect();

        let false_positives: Vec<usize> = actual_set.difference(&expected_set).copied().collect();

        let mut result = Self {
            expected,
            actual,
            exact_matches,
            missing,
            false_positives,
        };

        // Sort for consistent output
        result.exact_matches.sort();
        result.missing.sort();
        result.false_positives.sort();

        result
    }

    /// Print a detailed report of the test result.
    pub fn print_report(&self, test_name: &str) {
        println!("\n=== {} ===", test_name);
        println!(
            "Expected: {} violations, Found: {} violations",
            self.expected.len(),
            self.actual.len()
        );
        println!(
            "Exact matches: {} ({:.1}%)",
            self.exact_matches.len(),
            self.detection_rate()
        );

        if !self.missing.is_empty() {
            println!(
                "Missing (false negatives): {} - lines {:?}",
                self.missing.len(),
                self.missing
            );
        }

        if !self.false_positives.is_empty() {
            println!(
                "False positives: {} - lines {:?}",
                self.false_positives.len(),
                self.false_positives
            );
        }

        if self.is_perfect() {
            println!("PERFECT: All violations matched with no false positives!");
        }
    }

    /// Calculate detection rate as percentage.
    pub fn detection_rate(&self) -> f64 {
        if self.expected.is_empty() {
            return 100.0;
        }
        (self.exact_matches.len() as f64 / self.expected.len() as f64) * 100.0
    }

    /// Assert there are no false positives.
    pub fn assert_no_false_positives(&self) {
        assert!(
            self.false_positives.is_empty(),
            "Found {} false positives at lines: {:?}",
            self.false_positives.len(),
            self.false_positives
        );
    }

    /// Assert detection rate meets minimum threshold.
    pub fn assert_detection_rate(&self, min_percent: f64) {
        let rate = self.detection_rate();
        assert!(
            rate >= min_percent,
            "Detection rate {:.1}% below minimum {:.1}% - missing lines: {:?}",
            rate,
            min_percent,
            self.missing
        );
    }

    /// Check if result is perfect (100% detection, 0 false positives).
    pub fn is_perfect(&self) -> bool {
        self.missing.is_empty() && self.false_positives.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perfect_match() {
        let result = TestResult::compare(vec![1, 2, 3], vec![1, 2, 3]);
        assert!(result.is_perfect());
        assert_eq!(result.detection_rate(), 100.0);
    }

    #[test]
    fn test_missing_violations() {
        let result = TestResult::compare(vec![1, 2, 3], vec![1, 2]);
        assert_eq!(result.missing, vec![3]);
        // Use approximate comparison for floating point
        let rate = result.detection_rate();
        assert!(
            (rate - 66.666666).abs() < 0.001,
            "Expected ~66.67%, got {}",
            rate
        );
    }

    #[test]
    fn test_false_positives() {
        let result = TestResult::compare(vec![1, 2], vec![1, 2, 3]);
        assert_eq!(result.false_positives, vec![3]);
        assert_eq!(result.detection_rate(), 100.0);
    }

    #[test]
    fn test_mixed() {
        let result = TestResult::compare(vec![1, 2, 3], vec![2, 3, 4]);
        assert_eq!(result.exact_matches, vec![2, 3]);
        assert_eq!(result.missing, vec![1]);
        assert_eq!(result.false_positives, vec![4]);
    }

    #[test]
    fn test_empty_expected() {
        let result = TestResult::compare(vec![], vec![1, 2]);
        assert_eq!(result.detection_rate(), 100.0);
        assert_eq!(result.false_positives, vec![1, 2]);
    }
}
