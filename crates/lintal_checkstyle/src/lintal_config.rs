//! Parser for lintal.toml configuration files.
//!
//! lintal.toml is an optional overlay configuration that controls fix behavior
//! and points to the checkstyle.xml file. Example:
//!
//! ```toml
//! [fix]
//! unsafe = false
//!
//! [fix.rules]
//! WhitespaceAround = "fix"
//! LeftCurly = "check"
//! UnusedImports = "suggest"
//! MethodLength = "disabled"
//!
//! [checkstyle]
//! config = "config/checkstyle/checkstyle.xml"
//! ```

use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LintalConfigError {
    #[error("Failed to read config file: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to parse TOML: {0}")]
    Toml(#[from] toml::de::Error),
}

/// How a rule should handle violations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RuleMode {
    /// Auto-fix violations (default for fixable rules).
    #[default]
    Fix,
    /// Only check/report violations, don't fix.
    Check,
    /// Show fix suggestion, require confirmation.
    Suggest,
    /// Skip the rule entirely.
    Disabled,
}

impl<'de> Deserialize<'de> for RuleMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.to_lowercase().as_str() {
            "fix" => Ok(RuleMode::Fix),
            "check" => Ok(RuleMode::Check),
            "suggest" => Ok(RuleMode::Suggest),
            "disabled" | "disable" | "off" => Ok(RuleMode::Disabled),
            _ => Err(serde::de::Error::custom(format!(
                "Invalid rule mode: {}. Expected fix, check, suggest, or disabled",
                s
            ))),
        }
    }
}

/// Fix-related configuration.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct FixConfig {
    /// Whether to apply unsafe fixes without --unsafe flag.
    #[serde(default)]
    pub unsafe_fixes: bool,

    /// Per-rule fix mode overrides.
    #[serde(default)]
    pub rules: HashMap<String, RuleMode>,
}

/// Checkstyle-related configuration.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct CheckstyleReference {
    /// Path to checkstyle.xml config file.
    pub config: Option<String>,
}

/// Root lintal.toml configuration.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct LintalConfig {
    /// Fix behavior configuration.
    #[serde(default)]
    pub fix: FixConfig,

    /// Reference to checkstyle.xml.
    #[serde(default)]
    pub checkstyle: CheckstyleReference,
}

impl LintalConfig {
    /// Parse a lintal.toml file.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, LintalConfigError> {
        let content = std::fs::read_to_string(path)?;
        Self::parse(&content)
    }

    /// Parse lintal.toml content.
    pub fn parse(content: &str) -> Result<Self, LintalConfigError> {
        Ok(toml::from_str(content)?)
    }

    /// Get the fix mode for a specific rule.
    /// Returns the configured mode or the default (Fix).
    pub fn rule_mode(&self, rule_name: &str) -> RuleMode {
        self.fix
            .rules
            .get(rule_name)
            .copied()
            .unwrap_or(RuleMode::Fix)
    }

    /// Check if a rule is enabled.
    pub fn is_rule_enabled(&self, rule_name: &str) -> bool {
        self.rule_mode(rule_name) != RuleMode::Disabled
    }

    /// Check if a rule should be auto-fixed.
    pub fn should_fix(&self, rule_name: &str) -> bool {
        self.rule_mode(rule_name) == RuleMode::Fix
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_config() {
        let config = LintalConfig::parse("").unwrap();
        assert!(!config.fix.unsafe_fixes);
        assert!(config.fix.rules.is_empty());
        assert!(config.checkstyle.config.is_none());
    }

    #[test]
    fn test_parse_full_config() {
        let toml = r#"
[fix]
unsafe_fixes = true

[fix.rules]
WhitespaceAround = "fix"
LeftCurly = "check"
UnusedImports = "suggest"
MethodLength = "disabled"

[checkstyle]
config = "config/checkstyle/checkstyle.xml"
"#;

        let config = LintalConfig::parse(toml).unwrap();

        assert!(config.fix.unsafe_fixes);
        assert_eq!(config.rule_mode("WhitespaceAround"), RuleMode::Fix);
        assert_eq!(config.rule_mode("LeftCurly"), RuleMode::Check);
        assert_eq!(config.rule_mode("UnusedImports"), RuleMode::Suggest);
        assert_eq!(config.rule_mode("MethodLength"), RuleMode::Disabled);
        assert_eq!(config.rule_mode("UnknownRule"), RuleMode::Fix); // Default

        assert!(config.is_rule_enabled("WhitespaceAround"));
        assert!(!config.is_rule_enabled("MethodLength"));

        assert!(config.should_fix("WhitespaceAround"));
        assert!(!config.should_fix("LeftCurly"));

        assert_eq!(
            config.checkstyle.config,
            Some("config/checkstyle/checkstyle.xml".to_string())
        );
    }

    #[test]
    fn test_partial_config() {
        let toml = r#"
[checkstyle]
config = "checkstyle.xml"
"#;

        let config = LintalConfig::parse(toml).unwrap();
        assert!(!config.fix.unsafe_fixes);
        assert!(config.fix.rules.is_empty());
        assert_eq!(config.checkstyle.config, Some("checkstyle.xml".to_string()));
    }

    #[test]
    fn test_rule_mode_case_insensitive() {
        let toml = r#"
[fix.rules]
Rule1 = "FIX"
Rule2 = "CHECK"
Rule3 = "DISABLED"
Rule4 = "off"
"#;

        let config = LintalConfig::parse(toml).unwrap();
        assert_eq!(config.rule_mode("Rule1"), RuleMode::Fix);
        assert_eq!(config.rule_mode("Rule2"), RuleMode::Check);
        assert_eq!(config.rule_mode("Rule3"), RuleMode::Disabled);
        assert_eq!(config.rule_mode("Rule4"), RuleMode::Disabled);
    }
}
