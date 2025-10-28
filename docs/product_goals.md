## Product Vision

Create a next-generation macro and automation workstation that treats MIDI and future control surfaces as first-class programmable interfaces. The application must feel instantaneous in live-use scenarios, support sophisticated per-device layouts, and expose an extensible foundation for assistant-driven configuration and future hardware integrations.

## Target Users
- Power users who rely on MIDI controllers for creative, productivity, or accessibility workflows.
- Live performers needing reliable, low-latency automation tied to tactile hardware.
- Developers and tinkerers who want an open, scriptable automation hub they can extend.

## Core Use Cases
- **Per-device layouts:** Define multiple independent page sets for each attached controller without duplicating unrelated mappings.
- **Macro authoring:** Compose multi-step actions (keyboard, mouse, system calls, app automation) using reusable building blocks or scripts.
- **Rapid iteration:** Edit configurations in a human-friendly text format, reload instantly, and deploy to binary caches for live performance reliability.
- **Visual console:** Build screen-side dashboards (buttons, sliders, status monitors) that mirror controller state and provide manual overrides.
- **Assistant guidance:** Chat or voice interface that can describe mappings, suggest changes, and scaffold new workflows.

## Performance & Reliability Constraints
- **Latency ceiling:** Target end-to-end trigger-to-action latency under 5 ms on modern desktops; never exceed 10 ms even under load.
- **Deterministic execution:** Macro engine must avoid GC pauses or long critical sections; prefer lock-free or double-buffered queues for MIDI events.
- **Resilient configuration:** Treat text configs as authoritative, compiling to binary caches at load-time with integrity hashes to guarantee the runtime only uses validated data.
- **Hot reload safety:** Reject invalid configs atomically, roll back to last-known-good cache, and surface errors without interrupting live operation.
- **Resource footprint:** Keep background CPU usage negligible (<1% on idle systems) and memory overhead slim so multiple controllers can run concurrently.

## Guiding Principles
- **Responsiveness first:** UI interactions, macro execution, and assistant responses must feel immediate; all heavy work should be asynchronous or precomputed.
- **Open & inspectable:** Prefer widely understood languages (e.g., Python for scripting) and transparent schemas to encourage community adoption.
- **Modular core:** Separate hardware I/O, macro evaluation, UI, and assistant into explicit modules with versioned interfaces to support future plugin ecosystems.
- **Accessibility & Feedback:** On-screen prompts, hold/cancel behaviors, and state indicators should prevent accidental triggers and keep users informed.
- **Future hardware readiness:** Architecture should anticipate non-MIDI controllers (HID, OSC, custom USB) without redesign.
## Macro Library & UI Considerations

- Provide a collapsible vertical pane housing the macro library (both assigned and unassigned macros).
- Allow macros to be authored, edited, and saved without requiring immediate device/widget assignment.
- Support filtering/grouping (e.g., by tags, usage, device assignment) so unused macros can be organized for later use.
- Enable drag-and-drop from the library onto device widgets or the virtual console.
- Ensure schema supports macros existing independently, with metadata indicating usage or tags.
## Macro Lifecycle
- Macros can exist in two stages: `draft` (in-progress, not yet in runtime cache) and `ready` (validated, compiled, and available for assignment).
- The macro library pane displays separate sections for drafts and ready macros, with controls to promote/demote entries.
- Only ready macros participate in runtime cache generation; drafts remain editable without triggering runtime warnings.
- Validator reports errors for ready macros that fail validation; drafts receive informational diagnostics instead of blocking errors.
## Cache Architecture Notes
- Implement cache compiler in Rust under `tools/cache_builder` for reuse by runtime.
- Define shared structs in a dedicated crate (e.g., `cache_format`) so runtime and compiler agree on layout.
- Choose serialization format:
  - Start with `bincode` for simplicity; switch to FlatBuffers/Cap’n Proto later if random access is required.
- Runtime will watch cache output directory; on successful compile, atomically swap active cache file.
## Runtime Loader Loop
- Introduce a runtime `AppState` manager responsible for:
  - Loading YAML config using `load_from_path`.
  - Compiling cache via `compile_cache_from_path`.
  - Storing compiled bundle/bytes for other subsystems.
- Manager exposes channels or getters for UI/assistant to inspect diagnostics.
- Initial version runs synchronously at startup; later we add file watching for hot reload.
## Config Reload & Watcher
- Use filesystem watcher (notify crate) running on dedicated Tokio task.
- Debounce rapid events (e.g., 250ms) before triggering `AppState::reload` to avoid thrashing.
- On reload failure, preserve previous cache and surface diagnostics for UI/assistant.
- Provide channel/broadcast of reload events for other subsystems.
## MIDI & Virtual Console Integration
- Runtime should expose a `RuntimeBus` where modules (MIDI manager, VC renderer, assistant) subscribe to cache reload events.
- MIDI Manager responsibilities:
  - Consume `CompiledCache` to map MIDI triggers to macro execution descriptors.
  - Listen for debounced reload notifications; rebuild internal mappings when cache updates.
- Virtual Console manager responsibilities:
  - Render state from cache (macros, pages) and update UI components upon reload.
- Both managers initial implementation can use placeholders to log cache contents until full execution engine is built.
## Macro Execution Engine
- Executor builds trigger lookup from compiled cache (for now trigger == macro id).
- On MIDI event, executor resolves macro, iterates steps, and logs simulated actions (keystroke, pause).
- Later iterations will integrate real OS automation and script runner.
- Executor state is shared across runtime modules and refreshed on every cache reload.
## Action Execution Requirements
- Support keystroke actions via cross-platform injection layer (placeholder: logging + structure).
- Implement pause action using async timers to ensure sequential execution.
- Provide hook for future script execution by queuing tasks into Python host.
- Executor should run actions on a dedicated task to avoid blocking MIDI event loop.
## Keystroke Execution Strategy
- Abstract keystroke injection behind a trait (`KeySender`) so OS-specific implementations can live under feature flags.
- Initial placeholder logs actions; future work to implement real send operations per platform (Win32 SendInput, macOS Quartz, X11/Wayland).
- Ensure executor remains async-friendly: keystrokes dispatched sequentially, with optional rate limiting.
## MIDI Input Strategy
- Use `midir` for cross-platform MIDI input; map note-on events to macro trigger IDs via config-defined mapping (initially assume trigger_id == macro id).
- Normalize velocity/channel; allow future extensions for per-device mappings.
- Runtime manager spawns MIDI listener task feeding events into executor without blocking main loop.
