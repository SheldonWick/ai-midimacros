use std::path::PathBuf;
use std::sync::Arc;

use crate::app::{AppState, AppStateError};
use crate::config::CompiledCache;
use crate::console::ConsoleManager;
use crate::executor::{DefaultKeySender, Executor, MidiEvent, SharedExecutor};
use crate::midi::input::{spawn_midi_listener, MidiHandle};
use crate::midi::MidiManager;
use crate::watch::{watch_config, ReloadEvent, WatchHandle};
use notify::Error as NotifyError;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

#[derive(thiserror::Error, Debug)]
pub enum RuntimeManagerError {
    #[error("app state error: {0}")]
    App(#[from] AppStateError),
    #[error("watch error: {0}")]
    Watch(#[from] NotifyError),
    #[error("midi error: {0}")]
    Midi(anyhow::Error),
}

pub struct RuntimeManager {
    pub state: Arc<Mutex<AppState>>,
    pub midi: Arc<Mutex<MidiManager>>,
    pub console: Arc<Mutex<ConsoleManager>>,
    pub executor: SharedExecutor<DefaultKeySender>,
    watch: WatchHandle,
    midi_handle: MidiHandle,
    listener: JoinHandle<()>,
}

impl RuntimeManager {
    pub async fn initialize(config_path: PathBuf) -> Result<Self, RuntimeManagerError> {
        let app_state = AppState::initialize(config_path.clone())?;
        let (midi_tx, _) = tokio::sync::broadcast::channel(32);
        let midi = Arc::new(Mutex::new(MidiManager::new(midi_tx.clone())));
        let console = Arc::new(Mutex::new(ConsoleManager::new()));
        let executor = Arc::new(Mutex::new(Executor::new(Arc::new(DefaultKeySender::new()))));
        let midi_handle = spawn_midi_listener("ai-midimacros", midi_tx.clone())
            .map_err(RuntimeManagerError::Midi)?;
        let state = Arc::new(Mutex::new(app_state));

        {
            let state_guard = state.lock().await;
            apply_cache_to_modules(
                state_guard.compiled_cache().clone(),
                &midi,
                &console,
                &executor,
            )
            .await;
        }

        let watch = watch_config(config_path, state.clone())?;
        let mut rx = watch.subscribe();
        let state_clone = state.clone();
        let midi_clone = midi.clone();
        let console_clone = console.clone();
        let executor_clone = executor.clone();
        let mut midi_rx_exec = midi_tx.subscribe();
        let listener = tokio::spawn(async move {
            loop {
                tokio::select! {
                    Ok(event) = midi_rx_exec.recv() => {
                        let mut exec = executor_clone.lock().await;
                        let _ = exec.execute_midi_event(event).await;
                    }
                    Ok(event) = rx.recv() => {
                        if let ReloadEvent::Reloaded = event {
                            let cache = {
                                let guard = state_clone.lock().await;
                                guard.compiled_cache().clone()
                            };
                            apply_cache_to_modules(cache, &midi_clone, &console_clone, &executor_clone)
                                .await;
                        }
                    }
                    else => break,
                }
            }
        });

        Ok(Self {
            state,
            midi,
            console,
            executor,
            watch,
            midi_handle,
            listener,
        })
    }

    pub async fn trigger_midi(&self, event: MidiEvent) -> bool {
        let mut exec_guard = self.executor.lock().await;
        exec_guard.execute_midi_event(event).await
    }

    pub fn shutdown(self) {
        self.watch.join_handle.abort();
        self.listener.abort();
        self.midi_handle.join_handle.abort();
    }
}

async fn apply_cache_to_modules(
    cache: CompiledCache,
    midi: &Arc<Mutex<MidiManager>>,
    console: &Arc<Mutex<ConsoleManager>>,
    executor: &SharedExecutor<DefaultKeySender>,
) {
    {
        let mut midi_guard = midi.lock().await;
        midi_guard.apply_cache(&cache);
    }
    {
        let mut console_guard = console.lock().await;
        console_guard.apply_cache(&cache);
    }
    {
        let mut exec_guard = executor.lock().await;
        exec_guard.apply_cache(&cache);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::Duration;

    fn sample_config(macros: &[(&str, &str)]) -> String {
        let mut yaml = String::from("version: 1\ndevices: {}\nmacros:\n");
        for (id, key) in macros {
            yaml.push_str(&format!(
                "  {}:\n    status: ready\n    trigger:\n      type: note\n      number: {}\n    steps:\n      - type: keystroke\n        keys: [\"{}\"]\n",
                id,
                60 + id.len() as u8,
                key
            ));
        }
        yaml.push_str("scripts: {}\n");
        yaml
    }

    #[tokio::test]
    async fn runtime_manager_tracks_reload() {
        let dir = tempfile::tempdir().expect("temp dir");
        let config_path = dir.path().join("config.yaml");
        fs::write(&config_path, sample_config(&[("macro1", "K")])).expect("write config");

        let manager = RuntimeManager::initialize(config_path.clone())
            .await
            .expect("init");
        {
            let midi = manager.midi.lock().await;
            assert_eq!(midi.last_loaded_macros, vec!["macro1".to_string()]);
        }

        fs::write(
            &config_path,
            sample_config(&[("macro1", "K"), ("macro2", "L")]),
        )
        .expect("rewrite config");

        tokio::time::sleep(Duration::from_secs(2)).await;

        {
            let midi = manager.midi.lock().await;
            assert!(midi.last_loaded_macros.iter().any(|id| id == "macro2"));
        }

        {
            let executed = manager
                .trigger_midi(MidiEvent {
                    note: 66,
                    velocity: 127,
                })
                .await;
            assert!(executed);
        }

        manager.shutdown();
    }
}
