use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use cache_format::{
    CACHE_VERSION, CacheBundle, CacheHeader, DeviceLayout, LayoutPage, LayoutWidget, MacroEntry,
    MacroStep, MidiTrigger, MidiTriggerType, WidgetAction,
};
use config_validator::schema::{
    Action, Config, Device, MacroStatus, MacroStep as SchemaMacroStep,
    MidiTrigger as SchemaTrigger, MidiTriggerType as SchemaTriggerType, Page,
    Widget as SchemaWidget,
};
use config_validator::{ConfigError, ValidationIssue, parse_config_str, validate_config};
use thiserror::Error;
use xxhash_rust::xxh3::xxh3_64;

#[derive(Debug)]
pub struct BuildOutput {
    pub bundle: CacheBundle,
    pub diagnostics: Vec<ValidationIssue>,
}

#[derive(Debug, Error)]
pub enum BuildError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Parse error: {0}")]
    Parse(#[from] ConfigError),
    #[error("Validation errors encountered")]
    Validation(Vec<ValidationIssue>),
    #[error("Serialization error: {0}")]
    Serialize(#[from] bincode::Error),
}

pub fn build_from_path(path: impl AsRef<Path>) -> Result<(BuildOutput, Vec<u8>), BuildError> {
    let path_ref = path.as_ref();
    let content = fs::read_to_string(path_ref)?;
    let output = build_from_str(&content)?;
    let bytes = bincode::serialize(&output.bundle)?;
    Ok((output, bytes))
}

pub fn build_from_str(content: &str) -> Result<BuildOutput, BuildError> {
    let config = parse_config_str(content)?;
    build_from_config(&config, content)
}

fn build_from_config(config: &Config, source: &str) -> Result<BuildOutput, BuildError> {
    let diagnostics = validate_config(config, source);
    if diagnostics
        .iter()
        .any(|issue| matches!(issue.severity, config_validator::Severity::Error))
    {
        return Err(BuildError::Validation(diagnostics));
    }

    let bundle = assemble_bundle(config, source);
    Ok(BuildOutput {
        bundle,
        diagnostics,
    })
}

fn assemble_bundle(config: &Config, source: &str) -> CacheBundle {
    let source_hash = xxh3_64(source.as_bytes());
    let generated_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let devices = convert_devices(&config.devices);
    let macros = config
        .macros
        .iter()
        .filter(|(_, m)| m.status == MacroStatus::Ready)
        .map(|(id, m)| MacroEntry {
            id: id.clone(),
            description: m.description.clone(),
            tags: m.tags.clone(),
            trigger: m.trigger.as_ref().map(convert_trigger),
            steps: m.steps.iter().map(convert_macro_step).collect(),
        })
        .collect();

    CacheBundle {
        header: CacheHeader {
            version: CACHE_VERSION,
            source_hash,
            generated_at,
        },
        devices,
        macros,
    }
}

fn convert_macro_step(step: &SchemaMacroStep) -> MacroStep {
    match step {
        SchemaMacroStep::Keystroke { keys } => MacroStep::Keystroke { keys: keys.clone() },
        SchemaMacroStep::Pause { ms } => MacroStep::Pause { ms: *ms },
    }
}

fn convert_trigger(trigger: &SchemaTrigger) -> MidiTrigger {
    MidiTrigger {
        r#type: match trigger.r#type {
            SchemaTriggerType::Note => MidiTriggerType::Note,
        },
        number: trigger.number,
    }
}

fn convert_devices(devices: &std::collections::HashMap<String, Device>) -> Vec<DeviceLayout> {
    let mut list: Vec<_> = devices.iter().collect();
    list.sort_by(|(a, _), (b, _)| a.cmp(b));

    list.into_iter()
        .map(|(id, device)| DeviceLayout {
            id: id.clone(),
            hardware_id: device.hardware_id.clone(),
            pages: convert_pages(&device.pages),
        })
        .collect()
}

fn convert_pages(pages: &[Page]) -> Vec<LayoutPage> {
    pages
        .iter()
        .map(|page| LayoutPage {
            name: page.name.clone(),
            widgets: convert_widgets(&page.widgets),
        })
        .collect()
}

fn convert_widgets(widgets: &[SchemaWidget]) -> Vec<LayoutWidget> {
    widgets
        .iter()
        .map(|widget| LayoutWidget {
            id: widget.id.clone(),
            tap_behavior: widget.tap_behavior.clone(),
            action: widget.action.as_ref().map(convert_action),
        })
        .collect()
}

fn convert_action(action: &Action) -> WidgetAction {
    match action {
        Action::Macro { ref_ } => WidgetAction::Macro { id: ref_.clone() },
        Action::Script { ref_ } => WidgetAction::Script { id: ref_.clone() },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_ready_macros_only() {
        let yaml = r#"version: 1
devices: {}
macros:
  ready:
    status: ready
    description: "Ready macro"
    tags: ["live"]
    trigger:
      type: note
      number: 60
    steps:
      - type: keystroke
        keys: ["Ctrl", "S"]
  draft:
    status: draft
    steps:
      - type: keystroke
        keys: ["Z"]
scripts: {}
"#;
        let output = build_from_str(yaml).expect("build");
        assert_eq!(output.bundle.macros.len(), 1);
        assert!(output.bundle.devices.is_empty());
        let ready = &output.bundle.macros[0];
        assert_eq!(ready.id, "ready");
        assert_eq!(ready.description.as_deref(), Some("Ready macro"));
        assert_eq!(ready.tags, vec!["live"]);
        assert_eq!(ready.steps.len(), 1);
        assert_eq!(ready.trigger.as_ref().unwrap().number, 60);
        match &ready.steps[0] {
            MacroStep::Keystroke { keys } => {
                assert_eq!(keys, &vec!["Ctrl".to_string(), "S".to_string()])
            }
            _ => panic!("unexpected step"),
        }
        assert!(output.diagnostics.is_empty());
    }
}
