//! Merged configuration from checkstyle.xml and lintal.toml.
//!
//! checkstyle.xml defines *what* rules run and their parameters.
//! lintal.toml defines *how* violations are handled.

use std::collections::HashMap;
use std::path::Path;

use crate::{CheckstyleConfig, CheckstyleError, LintalConfig, LintalConfigError, RuleMode};

/// Error during config loading.
#[derive(Debug)]
pub enum ConfigError {
    /// Error reading/parsing checkstyle.xml.
    Checkstyle(CheckstyleError),
    /// Error reading/parsing lintal.toml.
    Lintal(LintalConfigError),
    /// No configuration found.
    NoConfig,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Checkstyle(e) => write!(f, "Checkstyle config error: {}", e),
            ConfigError::Lintal(e) => write!(f, "Lintal config error: {}", e),
            ConfigError::NoConfig => write!(f, "No configuration found"),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<CheckstyleError> for ConfigError {
    fn from(e: CheckstyleError) -> Self {
        ConfigError::Checkstyle(e)
    }
}

impl From<LintalConfigError> for ConfigError {
    fn from(e: LintalConfigError) -> Self {
        ConfigError::Lintal(e)
    }
}

/// A configured rule with its properties and mode.
#[derive(Debug, Clone)]
pub struct ConfiguredRule {
    /// The rule name (checkstyle module name).
    pub name: String,
    /// Properties from checkstyle.xml.
    pub properties: HashMap<String, String>,
    /// How to handle violations (from lintal.toml).
    pub mode: RuleMode,
}

impl ConfiguredRule {
    /// Get a property value by name.
    pub fn property(&self, name: &str) -> Option<&str> {
        self.properties.get(name).map(String::as_str)
    }

    /// Get properties as a reference map (for FromConfig).
    pub fn properties_ref(&self) -> HashMap<&str, &str> {
        self.properties
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect()
    }

    /// Check if this rule is enabled.
    pub fn is_enabled(&self) -> bool {
        self.mode != RuleMode::Disabled
    }

    /// Check if this rule should auto-fix.
    pub fn should_fix(&self) -> bool {
        self.mode == RuleMode::Fix
    }
}

/// Merged configuration combining checkstyle.xml and lintal.toml.
#[derive(Debug, Clone)]
pub struct MergedConfig {
    /// All configured rules.
    pub rules: Vec<ConfiguredRule>,
    /// Whether to apply unsafe fixes.
    pub unsafe_fixes: bool,
}

impl MergedConfig {
    /// Create a merged config from checkstyle.xml and optional lintal.toml.
    pub fn new(checkstyle: &CheckstyleConfig, lintal: Option<&LintalConfig>) -> Self {
        let lintal = lintal.cloned().unwrap_or_default();

        let rules = checkstyle
            .rules()
            .iter()
            .map(|module| ConfiguredRule {
                name: module.name.clone(),
                properties: module
                    .properties_map()
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect(),
                mode: lintal.rule_mode(&module.name),
            })
            .collect();

        Self {
            rules,
            unsafe_fixes: lintal.fix.unsafe_fixes,
        }
    }

    /// Get enabled rules (not disabled).
    pub fn enabled_rules(&self) -> impl Iterator<Item = &ConfiguredRule> {
        self.rules.iter().filter(|r| r.is_enabled())
    }

    /// Get a specific rule by name.
    pub fn get_rule(&self, name: &str) -> Option<&ConfiguredRule> {
        self.rules.iter().find(|r| r.name == name)
    }

    /// Check if a rule is enabled.
    pub fn is_rule_enabled(&self, name: &str) -> bool {
        self.get_rule(name).map(|r| r.is_enabled()).unwrap_or(false)
    }
}

/// Builder for loading configuration from files.
pub struct ConfigLoader {
    checkstyle_path: Option<std::path::PathBuf>,
    lintal_path: Option<std::path::PathBuf>,
}

impl ConfigLoader {
    /// Create a new config loader.
    pub fn new() -> Self {
        Self {
            checkstyle_path: None,
            lintal_path: None,
        }
    }

    /// Set the checkstyle.xml path.
    pub fn checkstyle(mut self, path: impl AsRef<Path>) -> Self {
        self.checkstyle_path = Some(path.as_ref().to_path_buf());
        self
    }

    /// Set the lintal.toml path.
    pub fn lintal(mut self, path: impl AsRef<Path>) -> Self {
        self.lintal_path = Some(path.as_ref().to_path_buf());
        self
    }

    /// Try to find lintal.toml in common locations.
    pub fn find_lintal(mut self) -> Self {
        let candidates = ["lintal.toml", ".lintal.toml", "config/lintal.toml"];
        for candidate in candidates {
            if Path::new(candidate).exists() {
                self.lintal_path = Some(std::path::PathBuf::from(candidate));
                break;
            }
        }
        self
    }

    /// Try to find checkstyle.xml from lintal.toml or common locations.
    pub fn find_checkstyle(mut self, lintal: Option<&LintalConfig>) -> Self {
        // First check if lintal.toml specifies the path
        if let Some(lintal) = lintal
            && let Some(path) = &lintal.checkstyle.config
            && Path::new(path).exists()
        {
            self.checkstyle_path = Some(std::path::PathBuf::from(path));
            return self;
        }

        // Try common locations
        let candidates = [
            "checkstyle.xml",
            "config/checkstyle/checkstyle.xml",
            "config/checkstyle.xml",
            ".checkstyle.xml",
        ];
        for candidate in candidates {
            if Path::new(candidate).exists() {
                self.checkstyle_path = Some(std::path::PathBuf::from(candidate));
                break;
            }
        }
        self
    }

    /// Load and merge the configuration.
    pub fn load(self) -> Result<MergedConfig, ConfigError> {
        // Load lintal.toml if specified
        let lintal = match &self.lintal_path {
            Some(path) if path.exists() => Some(LintalConfig::from_file(path)?),
            _ => None,
        };

        // Try to find checkstyle.xml from lintal config
        let checkstyle_path = self.checkstyle_path.or_else(|| {
            lintal
                .as_ref()
                .and_then(|l| l.checkstyle.config.as_ref().map(std::path::PathBuf::from))
        });

        // Load checkstyle.xml
        let checkstyle = match checkstyle_path {
            Some(path) if path.exists() => CheckstyleConfig::from_file(&path)?,
            Some(path) => {
                return Err(ConfigError::Checkstyle(CheckstyleError::Io(
                    std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!("Checkstyle config not found: {}", path.display()),
                    ),
                )));
            }
            None => return Err(ConfigError::NoConfig),
        };

        Ok(MergedConfig::new(&checkstyle, lintal.as_ref()))
    }
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_checkstyle() -> CheckstyleConfig {
        let xml = r#"<?xml version="1.0"?>
<module name="Checker">
    <module name="TreeWalker">
        <module name="WhitespaceAround">
            <property name="allowEmptyMethods" value="true"/>
        </module>
        <module name="LeftCurly">
            <property name="option" value="nl"/>
        </module>
        <module name="NeedBraces"/>
    </module>
</module>"#;
        CheckstyleConfig::parse(xml).unwrap()
    }

    #[test]
    fn test_merged_config_without_lintal() {
        let checkstyle = sample_checkstyle();
        let merged = MergedConfig::new(&checkstyle, None);

        assert_eq!(merged.rules.len(), 3);
        assert!(!merged.unsafe_fixes);

        // All rules default to Fix mode
        for rule in &merged.rules {
            assert_eq!(rule.mode, RuleMode::Fix);
            assert!(rule.is_enabled());
        }

        // Check properties are preserved
        let ws = merged.get_rule("WhitespaceAround").unwrap();
        assert_eq!(ws.property("allowEmptyMethods"), Some("true"));
    }

    #[test]
    fn test_merged_config_with_lintal() {
        let checkstyle = sample_checkstyle();
        let lintal = LintalConfig::parse(
            r#"
[fix]
unsafe_fixes = true

[fix.rules]
WhitespaceAround = "fix"
LeftCurly = "check"
NeedBraces = "disabled"
"#,
        )
        .unwrap();

        let merged = MergedConfig::new(&checkstyle, Some(&lintal));

        assert_eq!(merged.rules.len(), 3);
        assert!(merged.unsafe_fixes);

        let ws = merged.get_rule("WhitespaceAround").unwrap();
        assert_eq!(ws.mode, RuleMode::Fix);
        assert!(ws.is_enabled());
        assert!(ws.should_fix());

        let lc = merged.get_rule("LeftCurly").unwrap();
        assert_eq!(lc.mode, RuleMode::Check);
        assert!(lc.is_enabled());
        assert!(!lc.should_fix());

        let nb = merged.get_rule("NeedBraces").unwrap();
        assert_eq!(nb.mode, RuleMode::Disabled);
        assert!(!nb.is_enabled());

        // enabled_rules should exclude disabled rules
        let enabled: Vec<_> = merged.enabled_rules().collect();
        assert_eq!(enabled.len(), 2);
    }
}
