pub mod app;
pub mod config;
pub mod console;
pub mod executor;
pub mod midi;
pub mod runtime;
pub mod watch;

pub use app::{AppState, AppStateError};
pub use config::{
    compile_cache_from_path, compile_cache_from_str, load_from_path, load_from_str, CompileError,
    CompiledCache, Diagnostic, DiagnosticSeverity, LoadError, LoadedConfig,
};
pub use console::ConsoleManager;
pub use executor::{ActionLog, DefaultKeySender, Executor, MidiEvent};
pub use midi::MidiManager;
pub use runtime::{RuntimeManager, RuntimeManagerError};
pub use watch::{watch_config, ReloadEvent, WatchHandle};

pub fn init() {
    println!("ai_midimacros_core initialized (stub)");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_runs() {
        init();
    }
}
