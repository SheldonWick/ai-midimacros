//! Placeholder MIDI manager hooking into compiled cache.

use crate::config::CompiledCache;
use crate::executor::MidiEvent;
use tokio::sync::broadcast;

#[derive(Debug)]
pub struct MidiManager {
    pub last_loaded_macros: Vec<String>,
    pub sender: broadcast::Sender<MidiEvent>,
}

impl MidiManager {
    pub fn new(sender: broadcast::Sender<MidiEvent>) -> Self {
        Self {
            last_loaded_macros: Vec::new(),
            sender,
        }
    }

    pub fn apply_cache(&mut self, cache: &CompiledCache) {
        self.last_loaded_macros = cache.bundle.macros.iter().map(|m| m.id.clone()).collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cache_format::{CacheBundle, CacheHeader, MacroEntry};

    fn sample_cache() -> CompiledCache {
        let bundle = CacheBundle {
            header: CacheHeader {
                version: cache_format::CACHE_VERSION,
                source_hash: 0,
                generated_at: 0,
            },
            devices: vec![],
            macros: vec![MacroEntry {
                id: "m1".into(),
                description: None,
                tags: vec![],
                trigger: None,
                steps: vec![],
            }],
        };
        CompiledCache {
            bundle,
            diagnostics: vec![],
            bytes: vec![],
        }
    }

    #[test]
    fn apply_cache_records_macro_ids() {
        let (tx, _rx) = broadcast::channel(16);
        let mut manager = MidiManager::new(tx);
        manager.apply_cache(&sample_cache());
        assert_eq!(manager.last_loaded_macros, vec!["m1".to_string()]);
    }
}

pub mod input;
