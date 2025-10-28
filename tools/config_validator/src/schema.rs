use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub version: u32,
    #[serde(default)]
    pub global: Option<Global>,
    #[serde(default)]
    pub devices: HashMap<String, Device>,
    #[serde(default)]
    pub macros: HashMap<String, Macro>,
    #[serde(default)]
    pub scripts: HashMap<String, Script>,
    #[serde(default)]
    pub virtual_console: Option<serde_yaml::Value>,
}

#[derive(Debug, Deserialize)]
pub struct Global {
    #[serde(default)]
    pub defaults: Option<Defaults>,
}

#[derive(Debug, Deserialize)]
pub struct Defaults {
    #[serde(default)]
    pub tap_hold_timeout_ms: Option<u64>,
    #[serde(default)]
    pub display: Option<DisplaySettings>,
}

#[derive(Debug, Deserialize)]
pub struct DisplaySettings {
    #[serde(default)]
    pub theme: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Device {
    pub hardware_id: Option<String>,
    #[serde(default)]
    pub pages: Vec<Page>,
}

#[derive(Debug, Deserialize)]
pub struct Page {
    pub name: String,
    #[serde(default)]
    pub widgets: Vec<Widget>,
}

#[derive(Debug, Deserialize)]
pub struct Widget {
    pub id: String,
    #[serde(default)]
    pub action: Option<Action>,
    #[serde(default)]
    pub tap_behavior: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    #[serde(rename_all = "snake_case")]
    Macro {
        #[serde(rename = "ref")]
        ref_: String,
    },
    Script {
        #[serde(rename = "ref")]
        ref_: String,
    },
}

#[derive(Debug, Deserialize)]
pub struct Macro {
    #[serde(default = "default_status")]
    pub status: MacroStatus,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub trigger: Option<MidiTrigger>,
    #[serde(default)]
    pub steps: Vec<MacroStep>,
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum MacroStatus {
    Draft,
    Ready,
}

fn default_status() -> MacroStatus {
    MacroStatus::Draft
}

#[derive(Debug, Deserialize)]
pub struct MidiTrigger {
    pub r#type: MidiTriggerType,
    pub number: u8,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MidiTriggerType {
    Note,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MacroStep {
    Keystroke { keys: Vec<String> },
    Pause { ms: u64 },
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Script {
    Body { body: String },
    Inline(String),
}
