use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::{broadcast, mpsc, Mutex};
use tokio::task::JoinHandle;

use crate::app::{AppState, AppStateError};

#[derive(Debug, Clone)]
pub enum ReloadEvent {
    Reloaded,
    Failed(Arc<AppStateError>),
}

pub struct WatchHandle {
    pub join_handle: JoinHandle<()>,
    event_tx: broadcast::Sender<ReloadEvent>,
    /// Keep watcher alive for lifetime of handle.
    _watcher: RecommendedWatcher,
}

impl WatchHandle {
    pub fn subscribe(&self) -> broadcast::Receiver<ReloadEvent> {
        self.event_tx.subscribe()
    }
}

pub fn watch_config(path: PathBuf, state: Arc<Mutex<AppState>>) -> notify::Result<WatchHandle> {
    let (event_tx, _event_rx) = broadcast::channel(16);
    let (notify_tx, mut notify_rx) = mpsc::channel(16);

    let mut watcher = notify::recommended_watcher({
        let notify_tx = notify_tx.clone();
        move |res| {
            let _ = notify_tx.blocking_send(res);
        }
    })?;

    watcher.watch(&path, RecursiveMode::NonRecursive)?;

    let event_tx_clone = event_tx.clone();
    let join_handle = tokio::spawn(async move {
        let event_tx = event_tx_clone;
        let debounce = Duration::from_millis(250);
        let mut deadline: Option<tokio::time::Instant> = None;

        loop {
            if let Some(next_deadline) = deadline {
                tokio::select! {
                    Some(event) = notify_rx.recv() => {
                        if let Ok(ev) = event {
                            if is_relevant(&ev.kind) {
                                deadline = Some(tokio::time::Instant::now() + debounce);
                            }
                        } else {
                            break;
                        }
                    }
                    _ = tokio::time::sleep_until(next_deadline) => {
                        deadline = None;
                        reload_state(&state, &event_tx).await;
                    }
                }
            } else {
                match notify_rx.recv().await {
                    Some(Ok(event)) => {
                        if is_relevant(&event.kind) {
                            deadline = Some(tokio::time::Instant::now() + debounce);
                        }
                    }
                    Some(Err(_)) => {
                        // Ignore errors but continue listening.
                        deadline = Some(tokio::time::Instant::now() + debounce);
                    }
                    None => break,
                }
            }
        }
    });

    Ok(WatchHandle {
        join_handle,
        event_tx,
        _watcher: watcher,
    })
}

fn is_relevant(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_) | EventKind::Other
    )
}

async fn reload_state(state: &Arc<Mutex<AppState>>, event_tx: &broadcast::Sender<ReloadEvent>) {
    let mut guard = state.lock().await;
    match guard.reload() {
        Ok(_) => {
            let _ = event_tx.send(ReloadEvent::Reloaded);
        }
        Err(err) => {
            let _ = event_tx.send(ReloadEvent::Failed(Arc::new(err)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppState;
    use std::fs;

    fn sample_config() -> String {
        r#"
version: 1
devices: {}
macros:
  ready:
    status: ready
    steps:
      - type: keystroke
        keys: ["Z"]
scripts: {}
"#
        .trim_start()
        .to_string()
    }

    #[tokio::test]
    async fn watcher_detects_changes() {
        let dir = tempfile::tempdir().expect("temp dir");
        let config_path = dir.path().join("config.yaml");
        fs::write(&config_path, sample_config()).expect("write config");

        let state = Arc::new(Mutex::new(
            AppState::initialize(config_path.clone()).expect("init"),
        ));

        let handle = watch_config(config_path.clone(), state.clone()).expect("watch");
        let mut rx = handle.subscribe();

        // Modify config to trigger reload.
        let updated = r#"
version: 1
devices: {}
macros:
  ready:
    status: ready
    steps:
      - type: keystroke
        keys: ["Z"]
  new_macro:
    status: ready
    steps:
      - type: keystroke
        keys: ["Y"]
scripts: {}
"#
        .trim_start();
        fs::write(&config_path, updated).expect("rewrite config");

        let event = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("timeout waiting for reload")
            .expect("channel closed");
        assert!(matches!(event, ReloadEvent::Reloaded));
        handle.join_handle.abort();
    }
}
