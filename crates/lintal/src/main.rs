//! lintal - A fast Java linter with auto-fix support.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use lintal_checkstyle::{CheckstyleConfig, ConfiguredRule, LintalConfig, MergedConfig};
use lintal_diagnostics::{Applicability, Diagnostic, Edit};
use lintal_java_cst::{CstNode, TreeWalker};
use lintal_java_parser::{JavaParser, java_kind_id_map, java_language};
use lintal_linter::{
    CheckContext, FileSuppressionsConfig, PlainTextCommentFilterConfig, Rule, RuleRegistry,
    SuppressionContext,
};
use lintal_text_size::Ranged;
use rayon::prelude::*;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use walkdir::WalkDir;

// Thread-local parser to avoid repeated initialization overhead
thread_local! {
    static PARSER: RefCell<JavaParser> = RefCell::new(JavaParser::new());
}

struct DispatchTable {
    per_kind: Vec<Vec<usize>>,
    catch_all: Vec<usize>,
    /// Bitmap of which node kinds have any rules (including catch_all)
    has_rules: Vec<bool>,
}

impl DispatchTable {
    fn new(rules: &[Box<dyn Rule>]) -> Self {
        let language = java_language();
        let kind_count = language.node_kind_count();
        let mut per_kind: Vec<Vec<usize>> = vec![Vec::new(); kind_count];
        let mut catch_all = Vec::new();
        let kind_map = java_kind_id_map();
        let mut unknown_kinds: Vec<(&'static str, &'static str)> = Vec::new();

        for (idx, rule) in rules.iter().enumerate() {
            let kinds = rule.relevant_kinds();
            if kinds.is_empty() {
                catch_all.push(idx);
                continue;
            }

            for &kind in kinds {
                if let Some(ids) = kind_map.get(kind) {
                    for id in ids {
                        let slot = &mut per_kind[*id as usize];
                        if !slot.contains(&idx) {
                            slot.push(idx);
                        }
                    }
                } else {
                    unknown_kinds.push((rule.name(), kind));
                }
            }
        }

        #[cfg(debug_assertions)]
        if !unknown_kinds.is_empty() {
            let mut seen: std::collections::HashSet<(&'static str, &'static str)> =
                std::collections::HashSet::new();
            for (rule, kind) in unknown_kinds {
                if seen.insert((rule, kind)) {
                    eprintln!(
                        "Debug: rule '{}' references unknown node kind '{}'",
                        rule, kind
                    );
                }
            }
        }

        // Pre-compute which kinds have any rules
        let has_catch_all = !catch_all.is_empty();
        let has_rules: Vec<bool> = per_kind
            .iter()
            .map(|rules| has_catch_all || !rules.is_empty())
            .collect();

        Self {
            per_kind,
            catch_all,
            has_rules,
        }
    }

    /// Quick check if this node kind has any rules to run
    #[inline]
    fn has_rules_for_kind(&self, kind_id: u16) -> bool {
        self.has_rules[kind_id as usize]
    }

    fn rule_indices_for_kind(&self, kind_id: u16) -> impl Iterator<Item = usize> + '_ {
        self.per_kind[kind_id as usize]
            .iter()
            .copied()
            .chain(self.catch_all.iter().copied())
    }
}

/// Result of checking a single file.
struct FileCheckResult {
    violations: Vec<String>,
    violation_count: usize,
    fixable_count: usize,
}

/// Result of fixing a single file.
struct FileFixResult {
    fixed: usize,
    unfixable: usize,
    changed: bool,
    messages: Vec<String>,
}

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

        /// Directory for resolving ${config_loc} in suppressions.xml paths
        /// (defaults to the directory containing checkstyle.xml)
        #[arg(long)]
        config_loc: Option<PathBuf>,
    },
    /// Fix violations in files
    Fix {
        /// Paths to fix
        #[arg(required = true)]
        paths: Vec<PathBuf>,

        /// Path to checkstyle.xml config
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// Directory for resolving ${config_loc} in suppressions.xml paths
        /// (defaults to the directory containing checkstyle.xml)
        #[arg(long)]
        config_loc: Option<PathBuf>,

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
        Commands::Check {
            paths,
            config,
            config_loc,
        } => run_check(&paths, config.as_deref(), config_loc.as_deref()),
        Commands::Fix {
            paths,
            config,
            config_loc,
            diff,
            r#unsafe: allow_unsafe,
        } => run_fix(
            &paths,
            config.as_deref(),
            config_loc.as_deref(),
            diff,
            allow_unsafe,
        ),
    }
}

/// Run the check command.
fn run_check(
    paths: &[PathBuf],
    config_path: Option<&Path>,
    config_loc: Option<&Path>,
) -> Result<()> {
    // Load configuration
    let (rules, merged_config, suppression_filters, file_suppressions) =
        load_rules(config_path, config_loc, paths)?;
    let dispatch = DispatchTable::new(&rules);

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

    let files = collect_java_files(paths);
    let file_count = files.len();
    let files_processed = AtomicUsize::new(0);

    // Process files in parallel
    let results: Vec<FileCheckResult> = files
        .par_iter()
        .filter_map(|path| {
            // Skip files that are fully suppressed by file-based suppressions
            let path_str = path.to_string_lossy();
            if file_suppressions.is_file_fully_suppressed(&path_str) {
                files_processed.fetch_add(1, Ordering::Relaxed);
                return None;
            }

            let result = check_file(
                path,
                &rules,
                &dispatch,
                &suppression_filters,
                &file_suppressions,
            );
            files_processed.fetch_add(1, Ordering::Relaxed);
            result.ok()
        })
        .collect();

    // Aggregate and output results
    let mut total_violations = 0;
    let mut total_fixable = 0;

    for result in results {
        for violation in &result.violations {
            println!("{violation}");
        }
        total_violations += result.violation_count;
        total_fixable += result.fixable_count;
    }

    eprintln!("Checked {} files", file_count);

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
    config_loc: Option<&Path>,
    diff_only: bool,
    allow_unsafe: bool,
) -> Result<()> {
    let (rules, merged_config, suppression_filters, file_suppressions) =
        load_rules(config_path, config_loc, paths)?;
    let dispatch = DispatchTable::new(&rules);

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

    let files = collect_java_files(paths);

    // Process files in parallel
    let results: Vec<FileFixResult> = files
        .par_iter()
        .filter_map(|path| {
            // Skip files that are fully suppressed
            let path_str = path.to_string_lossy();
            if file_suppressions.is_file_fully_suppressed(&path_str) {
                return None;
            }

            fix_file(
                path,
                &rules,
                &dispatch,
                &suppression_filters,
                &file_suppressions,
                applicability,
                diff_only,
            )
            .ok()
        })
        .collect();

    // Aggregate and output results
    let mut total_fixed = 0;
    let mut total_unfixable = 0;
    let mut files_changed = 0;

    for result in results {
        for msg in &result.messages {
            print!("{msg}");
        }
        total_fixed += result.fixed;
        total_unfixable += result.unfixable;
        if result.changed {
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
    dispatch: &DispatchTable,
    suppression_filters: &[PlainTextCommentFilterConfig],
    file_suppressions: &FileSuppressionsConfig,
    applicability: Applicability,
    diff_only: bool,
) -> Result<FileFixResult> {
    let source = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;

    // Use thread-local parser to avoid repeated initialization
    let parse_result = PARSER.with(|parser| parser.borrow_mut().parse(&source));
    let Some(result) = parse_result else {
        return Ok(FileFixResult {
            fixed: 0,
            unfixable: 0,
            changed: false,
            messages: vec![format!("{}: Failed to parse\n", path.display())],
        });
    };

    let ctx = CheckContext::new(&source);
    let mut suppression_ctx = SuppressionContext::from_source(&source, suppression_filters);

    // Parse @SuppressWarnings annotations for additional suppressions
    let root = CstNode::new(result.tree.root_node(), &source);
    suppression_ctx.parse_suppress_warnings(&source, &root);

    let path_str = path.to_string_lossy();

    // Cache which rules are suppressed for this file (check once, not per-node)
    let suppressed_rules: Option<Vec<bool>> = if file_suppressions.is_empty() {
        None
    } else {
        Some(
            rules
                .iter()
                .map(|rule| file_suppressions.is_suppressed(&path_str, rule.name()))
                .collect(),
        )
    };

    // Collect all diagnostics, filtering out suppressed ones
    let mut diagnostics: Vec<Diagnostic> = Vec::new();
    let has_suppressions = suppression_ctx.has_suppressions();
    for node in TreeWalker::new(root.inner(), &source) {
        // Quick skip for nodes with no rules
        let kind_id = node.kind_id();
        if !dispatch.has_rules_for_kind(kind_id) {
            continue;
        }
        for rule_idx in dispatch.rule_indices_for_kind(kind_id) {
            if suppressed_rules.as_ref().is_some_and(|mask| mask[rule_idx]) {
                continue;
            }
            let rule = &rules[rule_idx];
            for diagnostic in rule.check(&ctx, &node) {
                if has_suppressions
                    && suppression_ctx.is_suppressed(rule.name(), diagnostic.range.start())
                {
                    continue;
                }
                diagnostics.push(diagnostic);
            }
        }
    }

    if diagnostics.is_empty() {
        return Ok(FileFixResult {
            fixed: 0,
            unfixable: 0,
            changed: false,
            messages: vec![],
        });
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
        return Ok(FileFixResult {
            fixed: 0,
            unfixable,
            changed: false,
            messages: vec![],
        });
    }

    // Sort edits by position (descending) to apply from end to start
    edits.sort_by_key(|e| std::cmp::Reverse(e.start()));

    // Remove overlapping edits (keep first one, which is the one with highest start)
    let edits = remove_overlapping_edits(edits);

    // Apply edits to source
    let fixed_source = apply_edits(&source, &edits);

    let mut messages = Vec::new();

    if diff_only {
        // Buffer diff output
        messages.push(format_diff(path, &source, &fixed_source));
    } else {
        // Write fixed source
        std::fs::write(path, &fixed_source)
            .with_context(|| format!("Failed to write {}", path.display()))?;
        messages.push(format!("{}: {} fix(es) applied\n", path.display(), fixed));
    }

    Ok(FileFixResult {
        fixed,
        unfixable,
        changed: true,
        messages,
    })
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
fn format_diff(path: &Path, original: &str, fixed: &str) -> String {
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

    output
}

/// Load rules from configuration or use defaults.
#[allow(clippy::type_complexity)]
fn load_rules(
    config_path: Option<&Path>,
    config_loc: Option<&Path>,
    base_paths: &[PathBuf],
) -> Result<(
    Vec<Box<dyn Rule>>,
    Option<MergedConfig>,
    Vec<PlainTextCommentFilterConfig>,
    FileSuppressionsConfig,
)> {
    let registry = RuleRegistry::builtin();

    // Try to load configuration
    let (merged_config, suppression_filters, file_suppressions) =
        load_config(config_path, config_loc, base_paths)?;

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

    Ok((rules, merged_config, suppression_filters, file_suppressions))
}

/// Load merged configuration from files.
fn load_config(
    config_path: Option<&Path>,
    config_loc: Option<&Path>,
    base_paths: &[PathBuf],
) -> Result<(
    Option<MergedConfig>,
    Vec<PlainTextCommentFilterConfig>,
    FileSuppressionsConfig,
)> {
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
        .or_else(|| find_checkstyle_config(base_paths));

    let Some(checkstyle_path) = checkstyle_path else {
        return Ok((None, vec![], FileSuppressionsConfig::new()));
    };

    if !checkstyle_path.exists() {
        anyhow::bail!("Checkstyle config not found: {}", checkstyle_path.display());
    }

    let checkstyle = CheckstyleConfig::from_file(&checkstyle_path)
        .with_context(|| format!("Failed to parse {}", checkstyle_path.display()))?;

    eprintln!("Loaded config from: {}", checkstyle_path.display());

    // Extract suppression filters from config
    let suppression_filters = extract_suppression_filters(&checkstyle);

    // Extract file-based suppressions
    // Use config_loc if provided, otherwise use the directory containing checkstyle.xml
    let file_suppressions = extract_file_suppressions(&checkstyle, &checkstyle_path, config_loc);

    Ok((
        Some(MergedConfig::new(&checkstyle, lintal.as_ref())),
        suppression_filters,
        file_suppressions,
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

/// Extract file-based suppressions from checkstyle config.
/// Looks for SuppressionFilter module and loads the referenced suppressions.xml file.
fn extract_file_suppressions(
    config: &CheckstyleConfig,
    checkstyle_path: &Path,
    config_loc: Option<&Path>,
) -> FileSuppressionsConfig {
    // Look for SuppressionFilter module
    for module in &config.modules {
        if module.name == "SuppressionFilter"
            && let Some(file_prop) = module.property("file")
        {
            // Resolve ${config_loc}:
            // - If --config-loc was provided, use that directory
            // - Otherwise, use the directory containing checkstyle.xml
            let config_dir = config_loc
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| {
                    checkstyle_path
                        .parent()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|| ".".to_string())
                });

            let resolved_path = file_prop.replace("${config_loc}", &config_dir);
            let suppressions_path = Path::new(&resolved_path);

            if suppressions_path.exists()
                && let Ok(xml) = std::fs::read_to_string(suppressions_path)
            {
                let config = FileSuppressionsConfig::from_xml(&xml);
                if !config.is_empty() {
                    eprintln!(
                        "Loaded {} file suppression(s) from: {}",
                        config.len(),
                        suppressions_path.display()
                    );
                }
                return config;
            }
        }
    }

    FileSuppressionsConfig::new()
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
/// Searches in both the current directory and the given base paths.
fn find_checkstyle_config(base_paths: &[PathBuf]) -> Option<PathBuf> {
    let candidates = [
        "checkstyle.xml",
        "config/checkstyle/checkstyle.xml",
        "config/checkstyle.xml",
        ".checkstyle.xml",
    ];

    // First try relative to each base path (the directories being checked)
    for base in base_paths {
        // If base is a file, use its parent directory
        let base_dir = if base.is_file() {
            base.parent().map(|p| p.to_path_buf())
        } else {
            Some(base.clone())
        };

        if let Some(base_dir) = base_dir {
            for candidate in &candidates {
                let path = base_dir.join(candidate);
                if path.exists() {
                    return Some(path);
                }
            }
        }
    }

    // Then try relative to the current directory
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
    dispatch: &DispatchTable,
    suppression_filters: &[PlainTextCommentFilterConfig],
    file_suppressions: &FileSuppressionsConfig,
) -> Result<FileCheckResult> {
    let source = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;

    // Use thread-local parser to avoid repeated initialization
    let parse_result = PARSER.with(|parser| parser.borrow_mut().parse(&source));
    let Some(result) = parse_result else {
        return Ok(FileCheckResult {
            violations: vec![format!("{}: Failed to parse", path.display())],
            violation_count: 0,
            fixable_count: 0,
        });
    };

    let ctx = CheckContext::new(&source);
    let mut suppression_ctx = SuppressionContext::from_source(&source, suppression_filters);

    // Parse @SuppressWarnings annotations for additional suppressions
    let root = CstNode::new(result.tree.root_node(), &source);
    suppression_ctx.parse_suppress_warnings(&source, &root);

    let mut violation_messages = Vec::new();
    let mut violation_count = 0;
    let mut fixable_count = 0;

    let path_str = path.to_string_lossy();

    // Cache which rules are suppressed for this file (check once, not per-node)
    let suppressed_rules: Option<Vec<bool>> = if file_suppressions.is_empty() {
        None
    } else {
        Some(
            rules
                .iter()
                .map(|rule| file_suppressions.is_suppressed(&path_str, rule.name()))
                .collect(),
        )
    };

    let has_suppressions = suppression_ctx.has_suppressions();
    for node in TreeWalker::new(root.inner(), &source) {
        // Quick skip for nodes with no rules
        let kind_id = node.kind_id();
        if !dispatch.has_rules_for_kind(kind_id) {
            continue;
        }
        for rule_idx in dispatch.rule_indices_for_kind(kind_id) {
            if suppressed_rules.as_ref().is_some_and(|mask| mask[rule_idx]) {
                continue;
            }
            let rule = &rules[rule_idx];
            for diagnostic in rule.check(&ctx, &node) {
                // Skip suppressed diagnostics (comment-based and @SuppressWarnings)
                if has_suppressions
                    && suppression_ctx.is_suppressed(rule.name(), diagnostic.range.start())
                {
                    continue;
                }

                violation_count += 1;
                if diagnostic.fix.is_some() {
                    fixable_count += 1;
                }

                let source_code = ctx.source_code();
                let loc = source_code.line_column(diagnostic.range.start());
                violation_messages.push(format!(
                    "{}:{}:{}: {} {}",
                    path.display(),
                    loc.line.get(),
                    loc.column.get(),
                    format!("[{}]", rule.name()).blue(),
                    diagnostic.kind.body
                ));
            }
        }
    }

    Ok(FileCheckResult {
        violations: violation_messages,
        violation_count,
        fixable_count,
    })
}
