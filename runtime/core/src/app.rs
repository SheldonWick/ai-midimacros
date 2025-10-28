use std::path::PathBuf;

use crate::config::{
    compile_cache_from_path, load_from_path, CompileError, CompiledCache, Diagnostic, LoadError,
    LoadedConfig,
};
use thiserror::Error;

#[derive(Debug)]
pub struct AppState {
    config_path: PathBuf,
    pub loaded: LoadedConfig,
    pub compiled: CompiledCache,
}

#[derive(Debug, Error)]
pub enum AppStateError {
    #[error("Failed to load config: {0}")]
    Load(#[from] LoadError),
    #[error("Failed to compile cache: {0}")]
    Compile(#[from] CompileError),
}

impl AppState {
    pub fn initialize(config_path: impl Into<PathBuf>) -> Result<Self, AppStateError> {
        let path = config_path.into();
        let loaded = load_from_path(&path)?;
        let compiled = compile_cache_from_path(&path)?;
        Ok(Self {
            config_path: path,
            loaded,
            compiled,
        })
    }

    pub fn reload(&mut self) -> Result<(), AppStateError> {
        let loaded = load_from_path(&self.config_path)?;
        let compiled = compile_cache_from_path(&self.config_path)?;
        self.loaded = loaded;
        self.compiled = compiled;
        Ok(())
    }

    pub fn config_path(&self) -> &PathBuf {
        &self.config_path
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.compiled.diagnostics
    }

    pub fn compiled_cache(&self) -> &CompiledCache {
        &self.compiled
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn sample_config() -> String {
        r#"version: 1
devices: {}
macros:
  ready:
    status: ready
    trigger:
      type: note
      number: 60
    steps:
      - type: keystroke
        keys: ["B"]
  draft:
    status: draft
    steps:
      - type: keystroke
        keys: ["X"]
scripts: {}
"#
        .to_string()
    }

    #[test]
    fn initialize_loads_and_compiles() {
        let dir = tempfile::tempdir().expect("temp dir");
        let config_path = dir.path().join("config.yaml");
        fs::write(&config_path, sample_config()).expect("write config");

        let app = AppState::initialize(config_path.clone()).expect("initialize");
        assert_eq!(app.compiled.bundle.macros.len(), 1);
        assert_eq!(app.compiled.bundle.macros[0].id, "ready");
        assert!(app.diagnostics().is_empty());

        // modify draft to ready with valid macro data and reload
        let new_config = r#"version: 1
devices: {}
macros:
  ready:
    status: ready
    trigger:
      type: note
      number: 60
    steps:
      - type: keystroke
        keys: ["B"]
  draft:
    status: ready
    trigger:
      type: note
      number: 61
    steps:
      - type: keystroke
        keys: ["C"]
scripts: {}
"#;
        fs::write(&config_path, new_config).expect("rewrite config");
        let mut app = app;
        app.reload().expect("reload");
        assert_eq!(app.compiled.bundle.macros.len(), 2);
    }
}
