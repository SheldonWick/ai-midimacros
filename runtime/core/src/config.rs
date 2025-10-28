use std::path::Path;
use std::{fs, path::PathBuf};

use bincode;
use cache_builder::{
    build_from_path as builder_build_from_path, build_from_str as builder_build_from_str,
    BuildError,
};
use cache_format::CacheBundle;
use config_validator::schema::{Config, Macro, MacroStatus};
use config_validator::{
    parse_config_str, validate_config, ConfigError, Location, Severity, ValidationIssue,
};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
}

impl From<Severity> for DiagnosticSeverity {
    fn from(value: Severity) -> Self {
        match value {
            Severity::Error => DiagnosticSeverity::Error,
            Severity::Warning => DiagnosticSeverity::Warning,
            Severity::Info => DiagnosticSeverity::Info,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub path: String,
    pub message: String,
    pub location: Option<Location>,
    pub severity: DiagnosticSeverity,
}

#[derive(Debug)]
pub struct LoadedConfig {
    pub path: Option<PathBuf>,
    pub config: Config,
    pub diagnostics: Vec<Diagnostic>,
}

impl LoadedConfig {
    pub fn ready_macros(&self) -> impl Iterator<Item = (&String, &Macro)> {
        self.config
            .macros
            .iter()
            .filter(|(_, macro_def)| macro_def.status == MacroStatus::Ready)
    }
}

#[derive(Debug, Error)]
pub enum LoadError {
    #[error("I/O error while reading config: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to parse config: {0}")]
    Parse(#[from] ConfigError),
    #[error("Validation errors prevented loading")]
    Validation(Vec<Diagnostic>),
}

pub fn load_from_path(path: impl AsRef<Path>) -> Result<LoadedConfig, LoadError> {
    let path_ref = path.as_ref();
    let content = fs::read_to_string(path_ref)?;
    let mut loaded = load_from_str(&content)?;
    loaded.path = Some(path_ref.to_path_buf());
    Ok(loaded)
}

pub fn load_from_str(content: &str) -> Result<LoadedConfig, LoadError> {
    let config = parse_config_str(content)?;
    let diagnostics = convert_issues(validate_config(&config, content));

    if diagnostics
        .iter()
        .any(|diag| diag.severity == DiagnosticSeverity::Error)
    {
        return Err(LoadError::Validation(diagnostics));
    }

    Ok(LoadedConfig {
        path: None,
        config,
        diagnostics,
    })
}

#[derive(Debug, Clone)]
pub struct CompiledCache {
    pub bundle: CacheBundle,
    pub diagnostics: Vec<Diagnostic>,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Error)]
pub enum CompileError {
    #[error("Validation errors prevented cache build")]
    Validation(Vec<Diagnostic>),
    #[error("Cache serialization failed: {0}")]
    Serialize(bincode::Error),
    #[error("Cache build failed: {0}")]
    Build(BuildError),
}

pub fn compile_cache_from_path(path: impl AsRef<Path>) -> Result<CompiledCache, CompileError> {
    match builder_build_from_path(path) {
        Ok((output, bytes)) => {
            let diagnostics = convert_issues(output.diagnostics);
            Ok(CompiledCache {
                bundle: output.bundle,
                diagnostics,
                bytes,
            })
        }
        Err(BuildError::Validation(diags)) => Err(CompileError::Validation(convert_issues(diags))),
        Err(err) => Err(CompileError::Build(err)),
    }
}

pub fn compile_cache_from_str(content: &str) -> Result<CompiledCache, CompileError> {
    match builder_build_from_str(content) {
        Ok(output) => {
            let diagnostics = convert_issues(output.diagnostics);
            let bytes = bincode::serialize(&output.bundle).map_err(CompileError::Serialize)?;
            Ok(CompiledCache {
                bundle: output.bundle,
                diagnostics,
                bytes,
            })
        }
        Err(BuildError::Validation(diags)) => Err(CompileError::Validation(convert_issues(diags))),
        Err(err) => Err(CompileError::Build(err)),
    }
}

fn convert_issues(issues: Vec<ValidationIssue>) -> Vec<Diagnostic> {
    issues.into_iter().map(convert_issue).collect()
}

fn convert_issue(issue: ValidationIssue) -> Diagnostic {
    Diagnostic {
        path: issue.path,
        message: issue.message,
        location: issue.location,
        severity: DiagnosticSeverity::from(issue.severity),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cache_format::MacroStep;

    #[test]
    fn loads_with_ready_macro_only() {
        let yaml = r#"version: 1
devices: {}
macros:
  ready_macro:
    status: ready
    trigger:
      type: note
      number: 60
    steps:
      - type: keystroke
        keys: ["Ctrl", "C"]
  draft_macro:
    status: draft
    steps:
      - type: keystroke
        keys: ["X"]
scripts: {}
"#;
        let loaded = load_from_str(yaml).expect("should load");
        let ready: Vec<_> = loaded.ready_macros().map(|(id, _)| id.clone()).collect();
        assert_eq!(ready, vec!["ready_macro".to_string()]);
        assert!(loaded.diagnostics.is_empty());
    }

    #[test]
    fn invalid_ready_macro_errors() {
        let yaml = r#"version: 1
devices: {}
macros:
  bad:
    status: ready
    trigger:
      type: note
      number: 60
    steps:
      - type: pause
        ms: 0
scripts: {}
"#;
        let err = load_from_str(yaml).unwrap_err();
        match err {
            LoadError::Validation(diags) => {
                assert_eq!(diags.len(), 1);
                assert_eq!(diags[0].severity, DiagnosticSeverity::Error);
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn compile_cache_from_str_ready_only() {
        let yaml = r#"version: 1
devices: {}
macros:
  ready:
    status: ready
    trigger:
      type: note
      number: 60
    steps:
      - type: keystroke
        keys: ["A"]
  draft:
    status: draft
    steps:
      - type: keystroke
        keys: ["B"]
scripts: {}
"#;
        let compiled = compile_cache_from_str(yaml).expect("compile");
        assert_eq!(compiled.bundle.macros.len(), 1);
        assert_eq!(compiled.bundle.macros[0].id, "ready");
        assert!(compiled.diagnostics.is_empty());
        match &compiled.bundle.macros[0].steps[0] {
            MacroStep::Keystroke { keys } => assert_eq!(keys, &vec!["A".to_string()]),
            _ => panic!("unexpected step"),
        }
    }

    #[test]
    fn compile_cache_from_str_fails_on_invalid_ready_macro() {
        let yaml = r#"version: 1
devices: {}
macros:
  bad:
    status: ready
    steps:
      - type: pause
        ms: 0
scripts: {}
"#;

        let err = compile_cache_from_str(yaml).unwrap_err();
        match err {
            CompileError::Validation(diags) => {
                assert!(diags
                    .iter()
                    .any(|d| d.severity == DiagnosticSeverity::Error));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
