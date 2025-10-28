//! Macro execution engine placeholder.

use std::collections::HashMap;
use std::sync::Arc;

use crate::config::CompiledCache;
use cache_format::{MacroEntry, MacroStep};
use tokio::sync::Mutex;
use tokio::task;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionLog {
    Keystroke(Vec<String>),
    Pause(u64),
}

#[derive(Debug, Clone)]
pub struct MidiEvent {
    pub note: u8,
    pub velocity: u8,
}

#[async_trait::async_trait]
pub trait KeySender: Send + Sync {
    async fn send_keystroke(&self, keys: &[String]);
}

pub struct LoggingKeySender;

impl LoggingKeySender {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl KeySender for LoggingKeySender {
    async fn send_keystroke(&self, _keys: &[String]) {}
}

pub struct EnigoKeySender;

impl EnigoKeySender {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl KeySender for EnigoKeySender {
    async fn send_keystroke(&self, keys: &[String]) {
        let keys = keys.to_vec();
        let _ = task::spawn_blocking(move || send_keys_blocking(keys)).await;
    }
}

#[derive(Debug)]
pub struct Executor<T: KeySender + 'static> {
    macros: HashMap<String, MacroEntry>,
    triggers: HashMap<u8, String>,
    pub last_actions: Vec<ActionLog>,
    key_sender: Arc<T>,
}

impl<T: KeySender + 'static> Executor<T> {
    pub fn new(key_sender: Arc<T>) -> Self {
        Self {
            macros: HashMap::new(),
            triggers: HashMap::new(),
            last_actions: Vec::new(),
            key_sender,
        }
    }

    pub fn apply_cache(&mut self, cache: &CompiledCache) {
        self.macros = cache
            .bundle
            .macros
            .iter()
            .cloned()
            .map(|entry| (entry.id.clone(), entry))
            .collect();
        self.triggers.clear();
        for entry in self.macros.values() {
            if let Some(trigger) = &entry.trigger {
                self.triggers.insert(trigger.number, entry.id.clone());
            }
        }
    }

    pub async fn execute_midi_event(&mut self, event: MidiEvent) -> bool {
        if let Some(id) = self.triggers.get(&event.note).cloned() {
            self.execute_macro(&id).await
        } else {
            false
        }
    }

    pub async fn execute_macro(&mut self, id: &str) -> bool {
        let Some(entry) = self.macros.get(id) else {
            return false;
        };
        self.last_actions.clear();
        for step in &entry.steps {
            match step {
                MacroStep::Keystroke { keys } => {
                    self.key_sender.send_keystroke(keys).await;
                    self.last_actions.push(ActionLog::Keystroke(keys.clone()))
                }
                MacroStep::Pause { ms } => {
                    self.last_actions.push(ActionLog::Pause(*ms));
                    tokio::time::sleep(std::time::Duration::from_millis(*ms)).await;
                }
            }
        }
        true
    }
}

pub type SharedExecutor<T> = Arc<Mutex<Executor<T>>>;

#[cfg(not(test))]
pub type DefaultKeySender = EnigoKeySender;

#[cfg(test)]
pub type DefaultKeySender = LoggingKeySender;

fn send_keys_blocking(keys: Vec<String>) {
    use enigo::{Enigo, Key, KeyboardControllable};

    if keys.is_empty() {
        return;
    }

    let mut enigo = Enigo::new();
    let mut modifiers: Vec<Key> = Vec::new();

    for key_str in keys.iter().take(keys.len().saturating_sub(1)) {
        if let Some(key) = map_key(key_str) {
            enigo.key_down(key.clone());
            modifiers.push(key);
        }
    }

    if let Some(last_str) = keys.last() {
        if let Some(last_key) = map_key(last_str) {
            enigo.key_click(last_key);
        }
    }

    for key in modifiers.into_iter().rev() {
        enigo.key_up(key);
    }
}

fn map_key(input: &str) -> Option<enigo::Key> {
    use enigo::Key;
    match input.to_ascii_lowercase().as_str() {
        "ctrl" | "control" => Some(Key::Control),
        "alt" => Some(Key::Alt),
        "shift" => Some(Key::Shift),
        "meta" | "cmd" | "command" | "super" => Some(Key::Meta),
        "enter" | "return" => Some(Key::Return),
        "space" | "spacebar" => Some(Key::Space),
        "tab" => Some(Key::Tab),
        "esc" | "escape" => Some(Key::Escape),
        s if s.len() == 1 => {
            let ch = s.chars().next().unwrap();
            Some(Key::Layout(ch))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cache_format::{CacheBundle, CacheHeader, MacroEntry, MidiTrigger, MidiTriggerType};

    struct MockSender;

    #[async_trait::async_trait]
    impl KeySender for MockSender {
        async fn send_keystroke(&self, _keys: &[String]) {}
    }

    fn sample_cache() -> CompiledCache {
        let bundle = CacheBundle {
            header: CacheHeader {
                version: cache_format::CACHE_VERSION,
                source_hash: 1,
                generated_at: 1,
            },
            devices: vec![],
            macros: vec![MacroEntry {
                id: "macro_a".into(),
                description: None,
                tags: vec![],
                trigger: Some(MidiTrigger {
                    r#type: MidiTriggerType::Note,
                    number: 60,
                }),
                steps: vec![
                    MacroStep::Keystroke {
                        keys: vec!["Ctrl".into(), "S".into()],
                    },
                    MacroStep::Pause { ms: 10 },
                ],
            }],
        };
        CompiledCache {
            bundle,
            diagnostics: vec![],
            bytes: vec![],
        }
    }

    #[tokio::test]
    async fn executes_macro_actions() {
        let cache = sample_cache();
        let mut executor = Executor::new(Arc::new(MockSender));
        executor.apply_cache(&cache);
        let result = executor.execute_macro("macro_a").await;
        assert!(result);
        assert_eq!(
            executor.last_actions,
            vec![
                ActionLog::Keystroke(vec!["Ctrl".into(), "S".into()]),
                ActionLog::Pause(10)
            ]
        );
    }

    #[tokio::test]
    async fn midi_event_dispatches_macro() {
        let cache = sample_cache();
        let mut executor = Executor::new(Arc::new(MockSender));
        executor.apply_cache(&cache);
        let event = MidiEvent {
            note: 60,
            velocity: 127,
        };
        assert!(executor.execute_midi_event(event).await);
        assert_eq!(executor.last_actions.len(), 2);
    }
}
