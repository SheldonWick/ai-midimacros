use std::fs;
use std::path::PathBuf;

use clap::Parser;
use config_validator::Severity;

#[derive(Parser, Debug)]
#[command(author, version, about = "Validate MIDI Macro Studio configs", long_about = None)]
struct Cli {
    /// Path to YAML configuration file
    path: PathBuf,
}

fn main() {
    let cli = Cli::parse();

    let content = match fs::read_to_string(&cli.path) {
        Ok(c) => c,
        Err(err) => {
            eprintln!("Failed to read {}: {err}", cli.path.display());
            std::process::exit(1);
        }
    };

    match config_validator::parse_config_str(&content) {
        Ok(config) => {
            let issues = config_validator::validate_config(&config, &content);
            if issues.is_empty() {
                println!("Validation OK: {}", cli.path.display());
            } else {
                let has_errors = issues.iter().any(|i| i.severity == Severity::Error);
                eprintln!("Validation diagnostics:");
                for issue in &issues {
                    let level = match issue.severity {
                        Severity::Error => "error",
                        Severity::Warning => "warning",
                        Severity::Info => "info",
                    };
                    if let Some(loc) = issue.location {
                        eprintln!(
                            "- [{}] {}: {} (line {}, column {})",
                            level, issue.path, issue.message, loc.line, loc.column
                        );
                    } else {
                        eprintln!("- [{}] {}: {}", level, issue.path, issue.message);
                    }
                }
                if has_errors {
                    std::process::exit(2);
                }
            }
        }
        Err(err) => {
            eprintln!("Validation failed: {err}");
            std::process::exit(1);
        }
    }
}
