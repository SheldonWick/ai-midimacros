//! Shared cache format describing the binary cache produced by the builder.

use serde::{Deserialize, Serialize};

/// Current cache format version.
pub const CACHE_VERSION: u32 = 1;

/// Header stored at the beginning of every cache artifact.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct CacheHeader {
    /// Cache format version (`CACHE_VERSION`).
    pub version: u32,
    /// Hash of the source configuration (e.g., xxhash64).
    pub source_hash: u64,
    /// UNIX timestamp (seconds) when cache was generated.
    pub generated_at: u64,
}

/// Root structure serialized into cache file.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct CacheBundle {
    pub header: CacheHeader,
    /// Per-device layouts and resolved widget wiring.
    pub devices: Vec<DeviceLayout>,
    /// Compiled macros that are safe to execute at runtime.
    pub macros: Vec<MacroEntry>,
    // TODO: add device layouts, scripts, overlays, etc.
}

/// A compiled macro ready for runtime execution.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct MacroEntry {
    pub id: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub trigger: Option<MidiTrigger>,
    pub steps: Vec<MacroStep>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct MidiTrigger {
    pub r#type: MidiTriggerType,
    pub number: u8,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "snake_case")]
pub enum MidiTriggerType {
    Note,
}

/// Device/page/widget layout snapshot for runtime/VC modules.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct DeviceLayout {
    pub id: String,
    pub hardware_id: Option<String>,
    pub pages: Vec<LayoutPage>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct LayoutPage {
    pub name: String,
    pub widgets: Vec<LayoutWidget>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct LayoutWidget {
    pub id: String,
    pub tap_behavior: Option<String>,
    pub action: Option<WidgetAction>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "snake_case")]
pub enum WidgetAction {
    Macro { id: String },
    Script { id: String },
}

/// Macro steps recorded in the cache.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum MacroStep {
    Keystroke { keys: Vec<String> },
    Pause { ms: u64 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_round_trip() {
        let bundle = CacheBundle {
            header: CacheHeader {
                version: CACHE_VERSION,
                source_hash: 42,
                generated_at: 1_700_000_000,
            },
            devices: vec![DeviceLayout {
                id: "launchpad".into(),
                hardware_id: Some("usb:demo.launchpad".into()),
                pages: vec![LayoutPage {
                    name: "Main".into(),
                    widgets: vec![LayoutWidget {
                        id: "pad_1".into(),
                        tap_behavior: Some("tap".into()),
                        action: Some(WidgetAction::Macro { id: "copy".into() }),
                    }],
                }],
            }],
            macros: vec![MacroEntry {
                id: "copy".to_string(),
                description: Some("Demo macro".into()),
                tags: vec!["demo".into()],
                trigger: Some(MidiTrigger {
                    r#type: MidiTriggerType::Note,
                    number: 60,
                }),
                steps: vec![
                    MacroStep::Keystroke {
                        keys: vec!["Ctrl".into(), "C".into()],
                    },
                    MacroStep::Pause { ms: 50 },
                ],
            }],
        };

        let bytes = bincode::serialize(&bundle).expect("serialize");
        let decoded: CacheBundle = bincode::deserialize(&bytes).expect("deserialize");
        assert_eq!(bundle, decoded);
    }
}
