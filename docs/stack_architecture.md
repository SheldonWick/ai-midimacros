## Language & Platform Selection

### Core Runtime
- **Language**: Rust (for deterministic low-latency MIDI/timer handling, modern tooling, strong safety without GC pauses).
- **Runtime Process**: Cross-platform desktop service running as the low-level engine.

### UI Shell
- **Framework**: Qt/QML via Rust bindings (or fallback to C++/Qt if bindings prove limiting). Provides high-performance desktop UI with fluid animations and flexible layout for virtual console.
- **Packaging**: Bundle UI with core runtime in a single desktop app; communication via shared async runtime inside process.

### Scripting Layer
- **Language**: Embedded Python 3 (via PyO3) sandboxed per macro execution. Widely adopted, fast enough for scripted automation when preloaded, with rich library ecosystem. Provide curated standard API for automation tasks.

### Assistant Services
- Local microservice (Rust or Python) exposing gRPC/WebSocket for chat/voice integration. Allows swapping in offline or remote LLMs without touching runtime core.

## High-Level Architecture

```
+-------------------------------------------------------------+
|                      Desktop Application                    |
|                                                             |
|  +-----------------+     +-------------------------------+  |
|  |  UI Shell (Qt)  |<--->|   Core Runtime (Rust Engine)  |  |
|  |                 |     |                               |  |
|  |  Virtual Console|     |  - MIDI I/O Manager           |  |
|  |  Config Editor  |     |  - Macro Scheduler            |  |
|  |  Assistant Pane |     |  - Script Host (Python)       |  |
|  +-----------------+     |  - Config Cache Loader        |  |
|                          |  - State Pub/Sub (event bus)  |  |
|                          +-------------------------------+  |
|                                      |                      |
|                                      v                      |
|                          +-------------------------------+  |
|                          | Assistant Service Layer       |  |
|                          | (gRPC/WebSocket, optional)    |  |
|                          +-------------------------------+  |
+-------------------------------------------------------------+
```

## Communication Patterns
- UI communicates with runtime through async channels (tokio) or QMetaObject bridging; runtime exposes state snapshots via pub/sub.
- Config changes triggered from UI cause rebuild of binary cache; runtime hot-swaps atomically.
- Assistant consumes runtime state through defined API and can request mutations under user approval.

## Cross-Platform Targets
- Windows 10+, macOS 12+, modern Linux distros (Wayland/X11). Ensure MIDI/HID libraries abstract platform differences (e.g., `midir` crate for MIDI, optional HID plugin later).
