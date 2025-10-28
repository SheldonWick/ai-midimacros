# Cache Builder (planned)

- A Rust-based tool will transform validated YAML configs into binary cache artifacts.
- Shared cache format definitions will live in a library crate so both the builder and runtime can deserialize safely.
- Initial milestone: serialize ready macros into cache sections; future milestones add device layouts and script bytecode.
