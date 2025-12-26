//! IndentLevel type for tracking multiple acceptable indentation levels.
//!
//! This is a port of checkstyle's IndentLevel class which uses a BitSet to track
//! multiple acceptable indentation levels. We use a sorted Vec for simplicity.

use std::fmt;

/// Encapsulates representation of expected indentation levels.
/// Provides a way to have multiple acceptable levels.
/// This type is immutable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndentLevel {
    /// Sorted set of acceptable indentation levels.
    levels: Vec<i32>,
}

impl IndentLevel {
    /// Creates a new instance with one acceptable indentation level.
    pub fn new(indent: i32) -> Self {
        Self {
            levels: vec![indent],
        }
    }

    /// Creates a new instance for nested structure.
    /// Adds offsets to each level in the base.
    pub fn with_offset(&self, offset: i32) -> Self {
        let mut levels: Vec<i32> = self.levels.iter().map(|&l| l + offset).collect();
        levels.sort_unstable();
        levels.dedup();
        Self { levels }
    }

    /// Creates a new instance with multiple offsets applied.
    pub fn with_offsets(&self, offsets: &[i32]) -> Self {
        let mut levels: Vec<i32> = self
            .levels
            .iter()
            .flat_map(|&l| offsets.iter().map(move |&o| l + o))
            .collect();
        levels.sort_unstable();
        levels.dedup();
        Self { levels }
    }

    /// Adds one or more acceptable indentation levels.
    pub fn add_acceptable(&self, additions: &[i32]) -> Self {
        let mut levels = self.levels.clone();
        levels.extend_from_slice(additions);
        levels.sort_unstable();
        levels.dedup();
        Self { levels }
    }

    /// Combines two IndentLevel instances.
    pub fn combine(&self, other: &IndentLevel) -> Self {
        let mut levels = self.levels.clone();
        levels.extend_from_slice(&other.levels);
        levels.sort_unstable();
        levels.dedup();
        Self { levels }
    }

    /// Checks whether we have more than one level.
    pub fn is_multi_level(&self) -> bool {
        self.levels.len() > 1
    }

    /// Checks if given indentation is acceptable (strict check - exact match).
    pub fn is_acceptable(&self, indent: i32) -> bool {
        self.levels.contains(&indent)
    }

    /// Checks if given indentation is acceptable with lenient checking.
    /// When force_strict=false, accepts any indent >= minimum expected level.
    pub fn is_acceptable_with_force_strict(&self, indent: i32, force_strict: bool) -> bool {
        if force_strict {
            self.levels.contains(&indent)
        } else {
            // Lenient mode: actual >= minimum expected is acceptable
            self.levels.first().is_some_and(|&min| indent >= min)
        }
    }

    /// Returns true if indent is less than the minimal acceptable level.
    pub fn is_greater_than(&self, indent: i32) -> bool {
        self.levels.first().is_some_and(|&min| min > indent)
    }

    /// Returns the first (minimum) indentation level.
    pub fn first_level(&self) -> i32 {
        self.levels.first().copied().unwrap_or(0)
    }

    /// Returns the last (maximum) indentation level.
    pub fn last_level(&self) -> i32 {
        self.levels.last().copied().unwrap_or(0)
    }

    /// Returns all acceptable levels.
    pub fn levels(&self) -> &[i32] {
        &self.levels
    }
}

impl fmt::Display for IndentLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.levels.len() == 1 {
            write!(f, "{}", self.levels[0])
        } else {
            let s: Vec<String> = self.levels.iter().map(|l| l.to_string()).collect();
            write!(f, "{}", s.join(", "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_level() {
        let level = IndentLevel::new(4);
        assert!(level.is_acceptable(4));
        assert!(!level.is_acceptable(0));
        assert!(!level.is_acceptable(8));
        assert!(!level.is_multi_level());
        assert_eq!(level.first_level(), 4);
        assert_eq!(level.to_string(), "4");
    }

    #[test]
    fn test_with_offset() {
        let level = IndentLevel::new(4).with_offset(4);
        assert!(level.is_acceptable(8));
        assert!(!level.is_acceptable(4));
        assert_eq!(level.first_level(), 8);
    }

    #[test]
    fn test_multi_level() {
        let level = IndentLevel::new(4).add_acceptable(&[8, 12]);
        assert!(level.is_acceptable(4));
        assert!(level.is_acceptable(8));
        assert!(level.is_acceptable(12));
        assert!(!level.is_acceptable(6));
        assert!(level.is_multi_level());
        assert_eq!(level.first_level(), 4);
        assert_eq!(level.last_level(), 12);
        assert_eq!(level.to_string(), "4, 8, 12");
    }

    #[test]
    fn test_is_greater_than() {
        let level = IndentLevel::new(8);
        assert!(level.is_greater_than(4));
        assert!(level.is_greater_than(0));
        assert!(!level.is_greater_than(8));
        assert!(!level.is_greater_than(12));
    }

    #[test]
    fn test_combine() {
        let level1 = IndentLevel::new(4);
        let level2 = IndentLevel::new(8);
        let combined = level1.combine(&level2);
        assert!(combined.is_acceptable(4));
        assert!(combined.is_acceptable(8));
        assert!(combined.is_multi_level());
    }

    #[test]
    fn test_with_offsets() {
        let level = IndentLevel::new(4).with_offsets(&[0, 4]);
        assert!(level.is_acceptable(4));
        assert!(level.is_acceptable(8));
        assert!(level.is_multi_level());
    }

    #[test]
    fn test_deduplication() {
        let level = IndentLevel::new(4).add_acceptable(&[4, 8, 4]);
        assert_eq!(level.levels().len(), 2);
        assert!(level.is_acceptable(4));
        assert!(level.is_acceptable(8));
    }
}
