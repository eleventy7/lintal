//! lintal - A fast Java linter with auto-fix support.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use lintal_checkstyle::{CheckstyleConfig, ConfiguredRule, LintalConfig, MergedConfig};
use lintal_diagnostics::{Applicability, Diagnostic, Edit};
use lintal_java_cst::{CstNode, TreeWalker};
use lintal_java_parser::JavaParser;
use lintal_linter::{
    CheckContext, PlainTextCommentFilterConfig, Rule, RuleRegistry, SuppressionContext,
};
use lintal_text_size::Ranged;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Parser)]
#[command(name = "lintal")]
#[command(about = "A fast Java linter with auto-fix support", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Check files for violations
    Check {
        /// Paths to check
        #[arg(required = true)]
        paths: Vec<PathBuf>,

        /// Path to checkstyle.xml config
        #[arg(short, long)]
        config: Option<PathBuf>,
    },
    /// Fix violations in files
    Fix {
        /// Paths to fix
        #[arg(required = true)]
        paths: Vec<PathBuf>,

        /// Path to checkstyle.xml config
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// Show diff without applying fixes
        #[arg(long)]
        diff: bool,

        /// Apply unsafe fixes
        #[arg(long)]
        r#unsafe: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Check { paths, config } => run_check(&paths, config.as_deref()),
        Commands::Fix {
            paths,
            config,
            diff,
            r#unsafe: allow_unsafe,
        } => run_fix(&paths, config.as_deref(), diff, allow_unsafe),
    }
}

/// Run the check command.
fn run_check(paths: &[PathBuf], config_path: Option<&Path>) -> Result<()> {
    // Load configuration
    let (rules, merged_config, suppression_filters) = load_rules(config_path)?;

    if rules.is_empty() {
        eprintln!("{}", "Warning: No rules configured".yellow());
    } else {
        let rule_names: Vec<_> = merged_config
            .as_ref()
            .map(|c| c.enabled_rules().map(|r| r.name.as_str()).collect())
            .unwrap_or_else(|| rules.iter().map(|r| r.name()).collect());
        eprintln!(
            "Checking with {} rule(s): {}",
            rule_names.len(),
            rule_names.join(", ")
        );
    }

    let mut total_violations = 0;
    let mut total_fixable = 0;

    for path in collect_java_files(paths) {
        let (violations, fixable) = check_file(&path, &rules, &suppression_filters)?;
        total_violations += violations;
        total_fixable += fixable;
    }

    if total_violations > 0 {
        println!(
            "\nFound {} violations ({} fixable)",
            total_violations.to_string().red(),
            total_fixable.to_string().yellow()
        );
        std::process::exit(1);
    } else {
        println!("{}", "No violations found".green());
    }

    Ok(())
}

/// Run the fix command.
fn run_fix(
    paths: &[PathBuf],
    config_path: Option<&Path>,
    diff_only: bool,
    allow_unsafe: bool,
) -> Result<()> {
    let (rules, merged_config, suppression_filters) = load_rules(config_path)?;

    if rules.is_empty() {
        eprintln!("{}", "Warning: No rules configured".yellow());
        return Ok(());
    }

    let rule_names: Vec<_> = merged_config
        .as_ref()
        .map(|c| c.enabled_rules().map(|r| r.name.as_str()).collect())
        .unwrap_or_else(|| rules.iter().map(|r| r.name()).collect());
    eprintln!(
        "Fixing with {} rule(s): {}",
        rule_names.len(),
        rule_names.join(", ")
    );

    let applicability = if allow_unsafe {
        Applicability::Unsafe
    } else {
        Applicability::Safe
    };

    let mut total_fixed = 0;
    let mut total_unfixable = 0;
    let mut files_changed = 0;

    for path in collect_java_files(paths) {
        let (fixed, unfixable, changed) = fix_file(
            &path,
            &rules,
            &suppression_filters,
            applicability,
            diff_only,
        )?;
        total_fixed += fixed;
        total_unfixable += unfixable;
        if changed {
            files_changed += 1;
        }
    }

    if diff_only {
        println!(
            "\n{} fix(es) available in {} file(s)",
            total_fixed.to_string().green(),
            files_changed
        );
    } else if total_fixed > 0 {
        println!(
            "\n{} fix(es) applied in {} file(s)",
            total_fixed.to_string().green(),
            files_changed
        );
    } else {
        println!("{}", "No fixes to apply".green());
    }

    if total_unfixable > 0 {
        eprintln!(
            "{} violation(s) could not be fixed automatically",
            total_unfixable.to_string().yellow()
        );
    }

    Ok(())
}

/// Fix violations in a single file.
fn fix_file(
    path: &PathBuf,
    rules: &[Box<dyn Rule>],
    suppression_filters: &[PlainTextCommentFilterConfig],
    applicability: Applicability,
    diff_only: bool,
) -> Result<(usize, usize, bool)> {
    let source = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;

    let mut parser = JavaParser::new();
    let Some(result) = parser.parse(&source) else {
        eprintln!("{}: Failed to parse", path.display());
        return Ok((0, 0, false));
    };

    let ctx = CheckContext::new(&source);
    let mut suppression_ctx = SuppressionContext::from_source(&source, suppression_filters);

    // Parse @SuppressWarnings annotations for additional suppressions
    let root = CstNode::new(result.tree.root_node(), &source);
    suppression_ctx.parse_suppress_warnings(&source, &root);

    // Collect all diagnostics, filtering out suppressed ones
    let mut diagnostics: Vec<Diagnostic> = Vec::new();
    for node in TreeWalker::new(root.inner(), &source) {
        for rule in rules {
            for diagnostic in rule.check(&ctx, &node) {
                if !suppression_ctx.is_suppressed(rule.name(), diagnostic.range.start()) {
                    diagnostics.push(diagnostic);
                }
            }
        }
    }

    if diagnostics.is_empty() {
        return Ok((0, 0, false));
    }

    // Collect applicable fixes
    let mut edits: Vec<Edit> = Vec::new();
    let mut fixed = 0;
    let mut unfixable = 0;

    for diagnostic in &diagnostics {
        if let Some(fix) = &diagnostic.fix {
            if fix.applies(applicability) {
                edits.extend(fix.edits().iter().cloned());
                fixed += 1;
            } else {
                unfixable += 1;
            }
        } else {
            unfixable += 1;
        }
    }

    if edits.is_empty() {
        return Ok((0, unfixable, false));
    }

    // Sort edits by position (descending) to apply from end to start
    edits.sort_by_key(|e| std::cmp::Reverse(e.start()));

    // Remove overlapping edits (keep first one, which is the one with highest start)
    let edits = remove_overlapping_edits(edits);

    // Apply edits to source
    let fixed_source = apply_edits(&source, &edits);

    if diff_only {
        // Show diff
        print_diff(path, &source, &fixed_source);
    } else {
        // Write fixed source
        std::fs::write(path, &fixed_source)
            .with_context(|| format!("Failed to write {}", path.display()))?;
        eprintln!("{}: {} fix(es) applied", path.display(), fixed);
    }

    Ok((fixed, unfixable, true))
}

/// Remove overlapping edits, keeping the first one (highest start position).
fn remove_overlapping_edits(edits: Vec<Edit>) -> Vec<Edit> {
    let mut result: Vec<Edit> = Vec::new();

    for edit in edits {
        // Check if this edit overlaps with any already accepted edit
        let overlaps = result.iter().any(|existing| {
            // Since edits are sorted descending by start, existing edits have higher starts
            // An overlap occurs if edit.end > existing.start
            edit.end() > existing.start()
        });

        if !overlaps {
            result.push(edit);
        }
    }

    result
}

/// Apply edits to source text.
fn apply_edits(source: &str, edits: &[Edit]) -> String {
    let mut result = source.to_string();

    // Edits are sorted descending by start position, so we can apply them in order
    for edit in edits {
        let start = usize::from(edit.start());
        let end = usize::from(edit.end());
        let content = edit.content().unwrap_or("");

        result.replace_range(start..end, content);
    }

    result
}

/// Print a unified diff between original and fixed source.
fn print_diff(path: &Path, original: &str, fixed: &str) {
    use std::fmt::Write;

    let mut output = String::new();
    writeln!(output, "--- a/{}", path.display()).unwrap();
    writeln!(output, "+++ b/{}", path.display()).unwrap();

    let original_lines: Vec<&str> = original.lines().collect();
    let fixed_lines: Vec<&str> = fixed.lines().collect();

    // Simple line-by-line diff
    let mut i = 0;
    let mut j = 0;
    let mut in_hunk = false;
    let mut hunk_start_orig = 0;
    let mut hunk_start_fixed = 0;
    let mut hunk_lines: Vec<String> = Vec::new();

    while i < original_lines.len() || j < fixed_lines.len() {
        let orig_line = original_lines.get(i);
        let fixed_line = fixed_lines.get(j);

        match (orig_line, fixed_line) {
            (Some(o), Some(f)) if o == f => {
                // Lines match
                if in_hunk {
                    hunk_lines.push(format!(" {}", o));
                }
                i += 1;
                j += 1;
            }
            (Some(o), Some(f)) => {
                // Lines differ
                if !in_hunk {
                    in_hunk = true;
                    hunk_start_orig = i + 1;
                    hunk_start_fixed = j + 1;
                }
                hunk_lines.push(format!("{}{}", "-".red(), o));
                hunk_lines.push(format!("{}{}", "+".green(), f));
                i += 1;
                j += 1;
            }
            (Some(o), None) => {
                // Original has extra line
                if !in_hunk {
                    in_hunk = true;
                    hunk_start_orig = i + 1;
                    hunk_start_fixed = j + 1;
                }
                hunk_lines.push(format!("{}{}", "-".red(), o));
                i += 1;
            }
            (None, Some(f)) => {
                // Fixed has extra line
                if !in_hunk {
                    in_hunk = true;
                    hunk_start_orig = i + 1;
                    hunk_start_fixed = j + 1;
                }
                hunk_lines.push(format!("{}{}", "+".green(), f));
                j += 1;
            }
            (None, None) => break,
        }

        // Flush hunk if we have context after changes
        if in_hunk && hunk_lines.len() > 6 {
            let context_count = hunk_lines
                .iter()
                .rev()
                .take_while(|l| l.starts_with(' '))
                .count();
            if context_count >= 3 {
                // Flush the hunk
                writeln!(
                    output,
                    "@@ -{},{} +{},{} @@",
                    hunk_start_orig,
                    hunk_lines.iter().filter(|l| !l.starts_with('+')).count(),
                    hunk_start_fixed,
                    hunk_lines.iter().filter(|l| !l.starts_with('-')).count()
                )
                .unwrap();
                for line in &hunk_lines {
                    writeln!(output, "{}", line).unwrap();
                }
                hunk_lines.clear();
                in_hunk = false;
            }
        }
    }

    // Flush remaining hunk
    if !hunk_lines.is_empty() {
        writeln!(
            output,
            "@@ -{},{} +{},{} @@",
            hunk_start_orig,
            hunk_lines.iter().filter(|l| !l.starts_with('+')).count(),
            hunk_start_fixed,
            hunk_lines.iter().filter(|l| !l.starts_with('-')).count()
        )
        .unwrap();
        for line in &hunk_lines {
            writeln!(output, "{}", line).unwrap();
        }
    }

    print!("{}", output);
}

/// Load rules from configuration or use defaults.
#[allow(clippy::type_complexity)]
fn load_rules(
    config_path: Option<&Path>,
) -> Result<(
    Vec<Box<dyn Rule>>,
    Option<MergedConfig>,
    Vec<PlainTextCommentFilterConfig>,
)> {
    let registry = RuleRegistry::builtin();

    // Try to load configuration
    let (merged_config, suppression_filters) = load_config(config_path)?;

    let rules: Vec<Box<dyn Rule>> = match &merged_config {
        Some(config) => {
            // Create rules from configuration
            config
                .enabled_rules()
                .filter_map(|configured_rule| create_rule_from_config(&registry, configured_rule))
                .collect()
        }
        None => {
            // No config found, use default WhitespaceAround
            eprintln!(
                "{}",
                "No checkstyle.xml found, using default WhitespaceAround rule".yellow()
            );
            vec![Box::new(lintal_linter::rules::WhitespaceAround::default())]
        }
    };

    Ok((rules, merged_config, suppression_filters))
}

/// Load merged configuration from files.
fn load_config(
    config_path: Option<&Path>,
) -> Result<(Option<MergedConfig>, Vec<PlainTextCommentFilterConfig>)> {
    // Load lintal.toml if it exists
    let lintal = find_lintal_config();

    // Determine checkstyle.xml path
    let checkstyle_path = config_path
        .map(PathBuf::from)
        .or_else(|| {
            lintal
                .as_ref()
                .and_then(|l| l.checkstyle.config.clone().map(PathBuf::from))
        })
        .or_else(find_checkstyle_config);

    let Some(checkstyle_path) = checkstyle_path else {
        return Ok((None, vec![]));
    };

    if !checkstyle_path.exists() {
        anyhow::bail!("Checkstyle config not found: {}", checkstyle_path.display());
    }

    let checkstyle = CheckstyleConfig::from_file(&checkstyle_path)
        .with_context(|| format!("Failed to parse {}", checkstyle_path.display()))?;

    eprintln!("Loaded config from: {}", checkstyle_path.display());

    // Extract suppression filters from config
    let suppression_filters = extract_suppression_filters(&checkstyle);

    Ok((
        Some(MergedConfig::new(&checkstyle, lintal.as_ref())),
        suppression_filters,
    ))
}

/// Extract suppression filters from checkstyle config.
fn extract_suppression_filters(config: &CheckstyleConfig) -> Vec<PlainTextCommentFilterConfig> {
    let mut filters = vec![];

    // Always add the default checkstyle suppression filter
    filters.push(PlainTextCommentFilterConfig::checkstyle_default());

    // Look for SuppressWithPlainTextCommentFilter modules
    for module in &config.modules {
        if module.name == "SuppressWithPlainTextCommentFilter"
            && let Some(filter) = create_filter_from_module(module)
        {
            filters.push(filter);
        }
    }

    filters
}

/// Create a filter config from a checkstyle module.
fn create_filter_from_module(
    module: &lintal_checkstyle::Module,
) -> Option<PlainTextCommentFilterConfig> {
    let off_format = module.property("offCommentFormat")?;
    let on_format = module.property("onCommentFormat")?;
    let check_format = module.property("checkFormat");

    PlainTextCommentFilterConfig::new(off_format, on_format, check_format)
}

/// Find lintal.toml in common locations.
fn find_lintal_config() -> Option<LintalConfig> {
    let candidates = ["lintal.toml", ".lintal.toml", "config/lintal.toml"];
    for candidate in candidates {
        let path = Path::new(candidate);
        if path.exists()
            && let Ok(config) = LintalConfig::from_file(path)
        {
            eprintln!("Loaded lintal.toml from: {}", candidate);
            return Some(config);
        }
    }
    None
}

/// Find checkstyle.xml in common locations.
fn find_checkstyle_config() -> Option<PathBuf> {
    let candidates = [
        "checkstyle.xml",
        "config/checkstyle/checkstyle.xml",
        "config/checkstyle.xml",
        ".checkstyle.xml",
    ];
    for candidate in candidates {
        let path = PathBuf::from(candidate);
        if path.exists() {
            return Some(path);
        }
    }
    None
}

/// Create a rule from configuration using the registry.
fn create_rule_from_config(
    registry: &RuleRegistry,
    configured_rule: &ConfiguredRule,
) -> Option<Box<dyn Rule>> {
    let props = configured_rule.properties_ref();

    if let Some(rule) = registry.create_rule(&configured_rule.name, &props) {
        Some(rule)
    } else {
        eprintln!(
            "{}: Unknown rule '{}', skipping",
            "Warning".yellow(),
            configured_rule.name
        );
        None
    }
}

fn collect_java_files(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for path in paths {
        if path.is_file() && path.extension().is_some_and(|e| e == "java") {
            files.push(path.clone());
        } else if path.is_dir() {
            for entry in WalkDir::new(path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().is_some_and(|ext| ext == "java"))
            {
                files.push(entry.path().to_path_buf());
            }
        }
    }
    files
}

fn check_file(
    path: &PathBuf,
    rules: &[Box<dyn Rule>],
    suppression_filters: &[PlainTextCommentFilterConfig],
) -> Result<(usize, usize)> {
    let source = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;

    let mut parser = JavaParser::new();
    let Some(result) = parser.parse(&source) else {
        eprintln!("{}: Failed to parse", path.display());
        return Ok((0, 0));
    };

    let ctx = CheckContext::new(&source);
    let mut suppression_ctx = SuppressionContext::from_source(&source, suppression_filters);

    // Parse @SuppressWarnings annotations for additional suppressions
    let root = CstNode::new(result.tree.root_node(), &source);
    suppression_ctx.parse_suppress_warnings(&source, &root);

    let mut violations = 0;
    let mut fixable = 0;

    for node in TreeWalker::new(root.inner(), &source) {
        for rule in rules {
            let diagnostics = rule.check(&ctx, &node);
            for diagnostic in diagnostics {
                // Skip suppressed diagnostics
                if suppression_ctx.is_suppressed(rule.name(), diagnostic.range.start()) {
                    continue;
                }

                violations += 1;
                if diagnostic.fix.is_some() {
                    fixable += 1;
                }

                let source_code = ctx.source_code();
                let loc = source_code.line_column(diagnostic.range.start());
                println!(
                    "{}:{}:{}: {} {}",
                    path.display(),
                    loc.line.get(),
                    loc.column.get(),
                    format!("[{}]", rule.name()).blue(),
                    diagnostic.kind.body
                );
            }
        }
    }

    Ok((violations, fixable))
}
