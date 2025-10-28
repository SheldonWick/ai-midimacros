use std::collections::{HashMap, HashSet};

use crate::schema::{Action, Config, MacroStatus, MacroStep, MidiTriggerType, Script};

#[derive(Debug, Clone, Copy)]
pub struct Location {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone)]
pub struct ValidationIssue {
    pub path: String,
    pub message: String,
    pub location: Option<Location>,
    pub severity: Severity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl ValidationIssue {
    pub fn new(path: String, message: String, severity: Severity) -> Self {
        Self {
            path,
            message,
            location: None,
            severity,
        }
    }
}

fn adjust_severity_for_macro(status: MacroStatus, severity: Severity) -> Severity {
    if status == MacroStatus::Draft && severity == Severity::Error {
        Severity::Warning
    } else {
        severity
    }
}

pub fn validate_config(config: &Config, source: &str) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    if config.version != 1 {
        issues.push(ValidationIssue::new(
            "version".into(),
            format!("Unsupported schema version {} (expected 1)", config.version),
            Severity::Error,
        ));
    }

    let mut hardware_ids: HashMap<String, String> = HashMap::new();
    for (device_name, device) in &config.devices {
        let path = format!("devices.{device_name}");

        match device.hardware_id.as_deref() {
            Some(id) if !id.trim().is_empty() => {
                let entry = hardware_ids.insert(id.trim().to_string(), device_name.clone());
                if let Some(previous) = entry {
                    issues.push(ValidationIssue::new(
                        format!("{path}.hardware_id"),
                        format!(
                            "Duplicate hardware_id `{}` also used by `{}`",
                            id.trim(),
                            previous
                        ),
                        Severity::Error,
                    ));
                }
            }
            Some(_) => {
                issues.push(ValidationIssue::new(
                    format!("{path}.hardware_id"),
                    "hardware_id must not be empty".into(),
                    Severity::Error,
                ));
            }
            None => {
                issues.push(ValidationIssue::new(
                    format!("{path}.hardware_id"),
                    "hardware_id is required".into(),
                    Severity::Error,
                ));
            }
        }

        for (page_index, page) in device.pages.iter().enumerate() {
            let mut widget_ids = HashSet::new();
            for widget in &page.widgets {
                let widget_path = format!("{path}.pages[{page_index}].widgets.{}", widget.id);

                if !widget_ids.insert(widget.id.clone()) {
                    issues.push(ValidationIssue::new(
                        widget_path.clone(),
                        "Duplicate widget id within page".into(),
                        Severity::Error,
                    ));
                }

                if let Some(action) = &widget.action {
                    match action {
                        Action::Macro { ref_ } => {
                            if !config.macros.contains_key(ref_) {
                                issues.push(ValidationIssue::new(
                                    widget_path.clone(),
                                    format!("References undefined macro `{}`", ref_),
                                    Severity::Error,
                                ));
                            } else if let Some(mac) = config.macros.get(ref_) {
                                if mac.status != MacroStatus::Ready {
                                    issues.push(ValidationIssue::new(
                                        widget_path.clone(),
                                        format!(
                                            "References macro `{}` that is not marked ready and will not be compiled",
                                            ref_
                                        ),
                                        Severity::Warning,
                                    ));
                                }
                            }
                        }
                        Action::Script { ref_ } => {
                            if !config.scripts.contains_key(ref_) {
                                issues.push(ValidationIssue::new(
                                    widget_path.clone(),
                                    format!("References undefined script `{}`", ref_),
                                    Severity::Error,
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    let mut note_map: HashMap<u8, String> = HashMap::new();

    for (macro_name, macro_def) in &config.macros {
        let macro_path = format!("macros.{macro_name}");

        if let Some(trigger) = &macro_def.trigger {
            match trigger.r#type {
                MidiTriggerType::Note => {
                    if trigger.number > 127 {
                        issues.push(ValidationIssue::new(
                            format!("{macro_path}.trigger"),
                            "Note trigger number must be between 0 and 127".into(),
                            adjust_severity_for_macro(macro_def.status, Severity::Error),
                        ));
                    } else if let Some(existing) =
                        note_map.insert(trigger.number, macro_name.clone())
                    {
                        issues.push(ValidationIssue::new(
                            format!("{macro_path}.trigger"),
                            format!(
                                "Note {} already assigned to macro `{}`",
                                trigger.number, existing
                            ),
                            Severity::Warning,
                        ));
                    }
                }
            }
        } else if macro_def.status == MacroStatus::Ready {
            issues.push(ValidationIssue::new(
                format!("{macro_path}.trigger"),
                "Ready macro missing trigger".into(),
                Severity::Warning,
            ));
        }

        for (idx, step) in macro_def.steps.iter().enumerate() {
            match step {
                MacroStep::Keystroke { keys } => {
                    if keys.is_empty() || keys.iter().any(|k| k.trim().is_empty()) {
                        issues.push(ValidationIssue::new(
                            format!("macros.{macro_name}.steps[{idx}]"),
                            "Keystroke step must define at least one non-empty key".into(),
                            adjust_severity_for_macro(macro_def.status, Severity::Error),
                        ));
                    }
                }
                MacroStep::Pause { ms } => {
                    if *ms == 0 {
                        issues.push(ValidationIssue::new(
                            format!("macros.{macro_name}.steps[{idx}]"),
                            "Pause duration must be greater than zero".into(),
                            adjust_severity_for_macro(macro_def.status, Severity::Error),
                        ));
                    }
                }
            }
        }
    }

    for (script_name, script) in &config.scripts {
        let empty = match script {
            Script::Body { body } => body.trim().is_empty(),
            Script::Inline(body) => body.trim().is_empty(),
        };
        if empty {
            issues.push(ValidationIssue::new(
                format!("scripts.{script_name}"),
                "Script body must not be empty".into(),
                Severity::Error,
            ));
        }
    }

    attach_locations(source, issues)
}

fn attach_locations(source: &str, mut issues: Vec<ValidationIssue>) -> Vec<ValidationIssue> {
    for issue in &mut issues {
        issue.location = find_location(source, &issue.path);
    }
    issues
}

fn find_location(source: &str, path: &str) -> Option<Location> {
    let needle = path.split('.').last()?;
    for (idx, line) in source.lines().enumerate() {
        if line.contains(needle) {
            let column = line.find(needle).map(|c| c + 1).unwrap_or(1);
            return Some(Location {
                line: idx + 1,
                column,
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_config_str;

    #[test]
    fn valid_config_with_trigger_passes() {
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
        keys: ["Ctrl", "C"]
scripts: {}
"#;
        let cfg = parse_config_str(yaml).expect("parse");
        let issues = validate_config(&cfg, yaml);
        assert!(issues.iter().all(|i| i.severity != Severity::Error));
    }

    #[test]
    fn missing_trigger_warns() {
        let yaml = r#"version: 1
devices: {}
macros:
  ready:
    status: ready
    steps:
      - type: keystroke
        keys: ["Ctrl", "C"]
scripts: {}
"#;
        let cfg = parse_config_str(yaml).expect("parse");
        let issues = validate_config(&cfg, yaml);
        assert!(
            issues
                .iter()
                .any(|i| matches!(i.severity, Severity::Warning))
        );
    }

    #[test]
    fn duplicate_note_triggers_warn() {
        let yaml = r#"version: 1
devices: {}
macros:
  a:
    status: ready
    trigger:
      type: note
      number: 64
    steps:
      - type: keystroke
        keys: ["A"]
  b:
    status: ready
    trigger:
      type: note
      number: 64
    steps:
      - type: keystroke
        keys: ["B"]
scripts: {}
"#;
        let cfg = parse_config_str(yaml).expect("parse");
        let issues = validate_config(&cfg, yaml);
        assert!(
            issues
                .iter()
                .any(|i| matches!(i.severity, Severity::Warning))
        );
    }

    #[test]
    fn draft_macro_invalid_step_downgrades_to_warning() {
        let yaml = r#"version: 1
devices: {}
macros:
  draft:
    status: draft
    steps:
      - type: keystroke
        keys: []
scripts: {}
"#;
        let cfg = parse_config_str(yaml).expect("parse");
        let issues = validate_config(&cfg, yaml);
        assert!(issues.iter().any(|i| i.path == "macros.draft.steps[0]"));
        assert!(!issues.iter().any(|i| matches!(i.severity, Severity::Error)));
    }

    #[test]
    fn widget_referencing_draft_macro_warns() {
        let yaml = r#"version: 1
devices:
  controller:
    hardware_id: "usb:test"
    pages:
      - name: "Main"
        widgets:
          - id: pad_1
            action:
              type: macro
              ref: draft_macro
macros:
  draft_macro:
    status: draft
    steps:
      - type: keystroke
        keys: ["A"]
scripts: {}
"#;
        let cfg = parse_config_str(yaml).expect("parse");
        let issues = validate_config(&cfg, yaml);
        assert!(issues.iter().any(|i| {
            i.path.ends_with("widgets.pad_1")
                && matches!(i.severity, Severity::Warning)
                && i.message.contains("not marked ready")
        }));
    }
}
