use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use cache_builder::{BuildError, build_from_path};
use clap::Parser;
use config_validator::Severity;

#[derive(Parser, Debug)]
#[command(author, version, about = "Compile configs into cache files", long_about = None)]
struct Cli {
    /// Path to YAML configuration bundle
    config: PathBuf,
    /// Output cache file path (defaults to config path with .cache)
    #[arg(long)]
    out: Option<PathBuf>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let out_path = cli
        .out
        .clone()
        .or_else(|| Some(default_output_path(&cli.config)));
    let out_path = out_path.expect("output path");

    match build_from_path(&cli.config) {
        Ok((output, bytes)) => {
            print_diagnostics(&output.diagnostics);
            fs::write(&out_path, bytes)
                .with_context(|| format!("writing cache to {}", out_path.display()))?;
            println!(
                "Cache generated at {} ({} macros)",
                out_path.display(),
                output.bundle.macros.len()
            );
            Ok(())
        }
        Err(BuildError::Validation(diags)) => {
            print_diagnostics(&diags);
            eprintln!("Cache build failed due to validation errors.");
            std::process::exit(2);
        }
        Err(err) => Err(err.into()),
    }
}

fn default_output_path(config_path: &PathBuf) -> PathBuf {
    let mut out = config_path.clone();
    out.set_extension("cache");
    out
}

fn print_diagnostics(diags: &[config_validator::ValidationIssue]) {
    if diags.is_empty() {
        return;
    }
    eprintln!("Diagnostics:");
    for diag in diags {
        let level = match diag.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        };
        if let Some(loc) = diag.location {
            eprintln!(
                "- [{}] {}: {} (line {}, column {})",
                level, diag.path, diag.message, loc.line, loc.column
            );
        } else {
            eprintln!("- [{}] {}: {}", level, diag.path, diag.message);
        }
    }
}
