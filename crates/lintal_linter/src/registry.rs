//! Rule registry for mapping checkstyle module names to rule implementations.

use std::collections::HashMap;

use crate::Rule;

/// Properties from a checkstyle module configuration.
pub type Properties<'a> = HashMap<&'a str, &'a str>;

/// Trait for rules that can be constructed from checkstyle config properties.
pub trait FromConfig: Rule + Sized {
    /// The checkstyle module name this rule corresponds to.
    const MODULE_NAME: &'static str;

    /// Create a rule instance from config properties.
    /// Properties are key-value pairs from the checkstyle module.
    fn from_config(properties: &Properties) -> Self;
}

/// A factory function that creates a boxed rule from properties.
type RuleFactory = fn(&Properties) -> Box<dyn Rule>;

/// Registry mapping checkstyle module names to rule factories.
pub struct RuleRegistry {
    factories: HashMap<&'static str, RuleFactory>,
}

impl RuleRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
        }
    }

    /// Create a registry with all built-in rules registered.
    pub fn builtin() -> Self {
        let mut registry = Self::new();
        registry.register_builtins();
        registry
    }

    /// Register a rule type that implements FromConfig.
    pub fn register<R: FromConfig + 'static>(&mut self) {
        self.factories
            .insert(R::MODULE_NAME, |props| Box::new(R::from_config(props)));
    }

    /// Register all built-in rules.
    fn register_builtins(&mut self) {
        use crate::rules::{
            ArrayTypeStyle, AvoidNestedBlocks, EmptyBlock, EmptyCatchBlock, EmptyForInitializerPad,
            EmptyLineSeparator, FileTabCharacter, FinalLocalVariable, FinalParameters, Indentation,
            LeftCurly, MethodParamPad, ModifierOrder, MultipleVariableDeclarations, NeedBraces,
            NoWhitespaceAfter, NoWhitespaceBefore, OneStatementPerLine, OperatorWrap, ParenPad,
            RedundantImport, RedundantModifier, RightCurly, SimplifyBooleanReturn,
            SingleSpaceSeparator, TypecastParenPad, UnusedImports, UpperEll, WhitespaceAfter,
            WhitespaceAround,
        };
        // Whitespace rules
        self.register::<WhitespaceAround>();
        self.register::<WhitespaceAfter>();
        self.register::<NoWhitespaceAfter>();
        self.register::<NoWhitespaceBefore>();
        self.register::<ParenPad>();
        self.register::<SingleSpaceSeparator>();
        self.register::<MethodParamPad>();
        self.register::<EmptyForInitializerPad>();
        self.register::<TypecastParenPad>();
        self.register::<FileTabCharacter>();
        self.register::<OperatorWrap>();
        self.register::<EmptyLineSeparator>();
        self.register::<Indentation>();
        // Block rules
        self.register::<LeftCurly>();
        self.register::<RightCurly>();
        self.register::<NeedBraces>();
        self.register::<EmptyBlock>();
        self.register::<EmptyCatchBlock>();
        self.register::<AvoidNestedBlocks>();
        // Modifier rules
        self.register::<ModifierOrder>();
        self.register::<FinalParameters>();
        self.register::<RedundantModifier>();
        self.register::<FinalLocalVariable>();
        // Style rules
        self.register::<UpperEll>();
        self.register::<ArrayTypeStyle>();
        // Import rules
        self.register::<RedundantImport>();
        self.register::<UnusedImports>();
        // Coding rules
        self.register::<OneStatementPerLine>();
        self.register::<MultipleVariableDeclarations>();
        self.register::<SimplifyBooleanReturn>();
    }

    /// Create a rule from a module name and properties.
    /// Returns None if the module name is not recognized.
    pub fn create_rule(&self, module_name: &str, properties: &Properties) -> Option<Box<dyn Rule>> {
        self.factories
            .get(module_name)
            .map(|factory| factory(properties))
    }

    /// Check if a module name is registered.
    pub fn has_rule(&self, module_name: &str) -> bool {
        self.factories.contains_key(module_name)
    }

    /// Get all registered module names.
    pub fn module_names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.factories.keys().copied()
    }
}

impl Default for RuleRegistry {
    fn default() -> Self {
        Self::builtin()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creates_whitespace_around() {
        let registry = RuleRegistry::builtin();

        let props = HashMap::new();
        let rule = registry.create_rule("WhitespaceAround", &props);

        assert!(rule.is_some());
        assert_eq!(rule.unwrap().name(), "WhitespaceAround");
    }

    #[test]
    fn test_registry_with_properties() {
        let registry = RuleRegistry::builtin();

        let mut props = HashMap::new();
        props.insert("allowEmptyLambdas", "true");

        let rule = registry.create_rule("WhitespaceAround", &props);
        assert!(rule.is_some());
    }

    #[test]
    fn test_registry_unknown_module() {
        let registry = RuleRegistry::builtin();

        let props = HashMap::new();
        let rule = registry.create_rule("UnknownRule", &props);

        assert!(rule.is_none());
    }
}
