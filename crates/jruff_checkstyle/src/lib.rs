//! Parser for checkstyle.xml configuration files.

use quick_xml::de::from_str;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CheckstyleError {
    #[error("Failed to read config file: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to parse XML: {0}")]
    Xml(#[from] quick_xml::DeError),
}

/// A property in a checkstyle module.
#[derive(Debug, Clone, Deserialize)]
pub struct Property {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@value")]
    pub value: String,
}

/// A checkstyle module (rule or container).
#[derive(Debug, Clone, Deserialize)]
pub struct Module {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(default, rename = "property")]
    pub properties: Vec<Property>,
    #[serde(default, rename = "module")]
    pub modules: Vec<Module>,
}

impl Module {
    /// Get a property value by name.
    pub fn property(&self, name: &str) -> Option<&str> {
        self.properties
            .iter()
            .find(|p| p.name == name)
            .map(|p| p.value.as_str())
    }

    /// Get properties as a map.
    pub fn properties_map(&self) -> HashMap<&str, &str> {
        self.properties
            .iter()
            .map(|p| (p.name.as_str(), p.value.as_str()))
            .collect()
    }
}

/// Root checkstyle configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "module")]
pub struct CheckstyleConfig {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(default, rename = "property")]
    pub properties: Vec<Property>,
    #[serde(default, rename = "module")]
    pub modules: Vec<Module>,
}

impl CheckstyleConfig {
    /// Parse a checkstyle.xml file.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, CheckstyleError> {
        let content = std::fs::read_to_string(path)?;
        Self::from_str(&content)
    }

    /// Parse checkstyle XML content.
    pub fn from_str(content: &str) -> Result<Self, CheckstyleError> {
        Ok(from_str(content)?)
    }

    /// Find the TreeWalker module.
    pub fn tree_walker(&self) -> Option<&Module> {
        self.modules.iter().find(|m| m.name == "TreeWalker")
    }

    /// Get all enabled rules from TreeWalker.
    pub fn rules(&self) -> Vec<&Module> {
        self.tree_walker()
            .map(|tw| tw.modules.iter().collect())
            .unwrap_or_default()
    }

    /// Get file-level modules (not in TreeWalker).
    pub fn file_modules(&self) -> Vec<&Module> {
        self.modules
            .iter()
            .filter(|m| m.name != "TreeWalker")
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_config() {
        let xml = r#"<?xml version="1.0"?>
<!DOCTYPE module PUBLIC "-//Puppy Crawl//DTD Check Configuration 1.3//EN"
        "https://checkstyle.org/dtds/configuration_1_3.dtd">
<module name="Checker">
    <module name="TreeWalker">
        <module name="WhitespaceAround">
            <property name="allowEmptyLambdas" value="true"/>
        </module>
        <module name="LeftCurly">
            <property name="option" value="nl"/>
        </module>
    </module>
</module>"#;

        let config = CheckstyleConfig::from_str(xml).unwrap();
        assert_eq!(config.name, "Checker");

        let rules = config.rules();
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].name, "WhitespaceAround");
        assert_eq!(rules[0].property("allowEmptyLambdas"), Some("true"));
        assert_eq!(rules[1].name, "LeftCurly");
        assert_eq!(rules[1].property("option"), Some("nl"));
    }

    #[test]
    fn test_parse_file_modules() {
        let xml = r#"<?xml version="1.0"?>
<module name="Checker">
    <module name="FileTabCharacter">
        <property name="eachLine" value="true"/>
    </module>
    <module name="LineLength">
        <property name="max" value="120"/>
    </module>
    <module name="TreeWalker">
        <module name="WhitespaceAround"/>
    </module>
</module>"#;

        let config = CheckstyleConfig::from_str(xml).unwrap();
        let file_modules = config.file_modules();
        assert_eq!(file_modules.len(), 2);
        assert_eq!(file_modules[0].name, "FileTabCharacter");
        assert_eq!(file_modules[1].name, "LineLength");
    }
}
