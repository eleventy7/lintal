//! jruff - A fast Java linter with auto-fix support.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use jruff_java_cst::TreeWalker;
use jruff_java_parser::JavaParser;
use jruff_linter::rules::WhitespaceAround;
use jruff_linter::{CheckContext, Rule};
use std::path::PathBuf;
use walkdir::WalkDir;

#[derive(Parser)]
#[command(name = "jruff")]
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
        Commands::Check { paths, config: _ } => {
            let mut total_violations = 0;
            let mut total_fixable = 0;

            for path in collect_java_files(&paths) {
                let (violations, fixable) = check_file(&path)?;
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
        }
        Commands::Fix {
            paths,
            config: _,
            diff,
            r#unsafe: _,
        } => {
            if diff {
                println!("Diff mode not yet implemented");
            } else {
                println!("Fix mode not yet implemented");
            }
            for _path in collect_java_files(&paths) {
                // TODO: Implement fixing
            }
        }
    }

    Ok(())
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

fn check_file(path: &PathBuf) -> Result<(usize, usize)> {
    let source = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;

    let mut parser = JavaParser::new();
    let Some(result) = parser.parse(&source) else {
        eprintln!("{}: Failed to parse", path.display());
        return Ok((0, 0));
    };

    let ctx = CheckContext::new(&source);
    let rules: Vec<Box<dyn Rule>> = vec![Box::new(WhitespaceAround::default())];

    let mut violations = 0;
    let mut fixable = 0;

    for node in TreeWalker::new(result.tree.root_node(), &source) {
        for rule in &rules {
            let diagnostics = rule.check(&ctx, &node);
            for diagnostic in diagnostics {
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
