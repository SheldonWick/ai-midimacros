pub mod schema;
pub mod validation;

use schema::Config;
use serde_yaml::Error as YamlError;
use thiserror::Error;

pub use validation::{Location, Severity, ValidationIssue, validate_config};

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("YAML parse error: {0}")]
    Parse(#[from] YamlError),
}

pub fn parse_config_str(src: &str) -> Result<Config, ConfigError> {
    let config = serde_yaml::from_str::<Config>(src)?;
    Ok(config)
}
