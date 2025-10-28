## Core Runtime Modules

### MIDI I/O Manager
- **Responsibilities**
  - Discover connected MIDI devices, maintain per-device state (ports, channels, velocity curves).
  - Normalize events into internal trigger messages with timestamps.
  - Manage per-device page state (active page index) and emit page-change events.
- **Interfaces**
  - Publishes `MidiEvent` messages onto Event Bus.
  - Exposes `DeviceRegistry` API (list devices, set active page, remap hardware IDs).
  - Depends on: platform MIDI library (`midir`), Config Cache for device layouts.

### Action & Macro Engine
- **Responsibilities**
  - Translate triggers into macro/action executions according to compiled cache.
  - Enforce hold/tap timing logic; schedule delayed or repeated actions.
  - Coordinate with Script Host for script actions; handle fallback if script fails.
- **Interfaces**
  - Consumes events from Event Bus (`TriggerEvent`).
  - Uses `ActionExecutor` trait implementations (keystroke, mouse, system command, display overlay).
  - Emits `ActionState` updates to UI/Assistant channels.
  - Depends on: Config Cache, Script Host, OS automation APIs.

### Script Host (Python Sandbox)
- **Responsibilities**
  - Maintain embedded CPython interpreter pool.
  - Preload compiled bytecode from cache; manage per-execution context objects (state, MIDIvalue, etc.).
  - Enforce timeouts/resource limits; provide escape hatches (terminate runaway scripts).
- **Interfaces**
  - Entry: `execute_script(script_id, context)` returning success/failure + outputs.
  - Publishes logs/diagnostics via Event Bus.
  - Depends on: compiled script bundle, Action APIs (callable from Python via bindings).

### Event Bus & State Store
- **Responsibilities**
  - Lightweight pub/sub for runtime modules (tokio broadcast channels).
  - Maintain snapshotable state (device statuses, active macros, diagnostics) for UI queries.
- **Interfaces**
  - Subscription API for UI and Assistant clients.
  - State query API returning struct snapshots.
  - Depends on: none (core utility).

### Config Loader & Watcher
- **Responsibilities**
  - Monitor config directories, trigger validation/compile pipeline.
  - Manage cache lifecycle (active, staged, history).
- **Interfaces**
  - `request_reload()` invoked by UI/CLI.
  - Emits `ConfigUpdate` events (success/failure) onto Event Bus.
  - Depends on: config workflow tooling.

### UI Shell Integration Layer
- **Responsibilities**
  - Bridge Qt/QML UI with runtime (through FFI bindings or IPC).
  - Expose commands (trigger macro, change page, open diagnostics) and state updates.
- **Interfaces**
  - QML-facing model classes (device list, virtual console widgets).
  - Subscribes to Event Bus snapshots; pushes user intents to runtime commands.

### Assistant Gateway (optional service)
- **Responsibilities**
  - Provide API endpoints for chat/voice; route intents to runtime with user confirmation.
  - Access sanitized state snapshots + schema docs for RAG.
- **Interfaces**
  - gRPC/WebSocket endpoints.
  - Consumes runtime state via Event Bus; issues requests over command channel.

### System Integration Helpers
- **Keystroke/Mouse Injector**: Abstract OS-level automation (Win32 SendInput, macOS Quartz, X11/Wayland backends).
- **Overlay Manager**: Render on-screen messages/labels (use Qt Quick overlays or platform toast equivalents).
- **Timer Wheel**: High-resolution scheduler for tap-hold timers, repeats.

## Module Interactions Summary
1. MIDI device sends event -> MIDI I/O normalizes -> Event Bus broadcast.
2. Macro Engine matches trigger -> resolves action list from cache -> executes via Action Executors.
3. Script actions call into Python host -> results forwarded to Action Executors.
4. UI listens for state updates (active page, running macro) -> updates virtual console display.
5. Config changes trigger loader -> compile to cache -> runtime hot swaps -> notify UI.
