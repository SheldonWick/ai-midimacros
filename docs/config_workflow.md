## Configuration & Cache Strategy

### Authoring Format (YAML)
Human-editable files use YAML with a consistent schema. Top-level structure:
```yaml
version: 1
global:
  defaults:
    tap_hold_timeout_ms: 400
    display:
      theme: "dark"
devices:
  launchpad_pro_mk3:
    hardware_id: "usb:focusrite.launchpadpro:rev3"
    pages:
      - name: "Photoshop"
        surface: "matrix"
        widgets:
          - id: pad_1_1
            action: {type: "macro", ref: "macros.copy"}
            tap_behavior: tap_hold
  nano_kontrol2:
    hardware_id: "usb:korg.nano_kontrol2"
    pages:
      - name: "Mix"
        surface: "sliders"
        widgets:
          - id: slider1
            action: {type: "script", ref: "scripts.level_fade"}
macros:
  copy:
    status: ready
    trigger:
      type: note
      number: 60
    steps:
      - {type: "keystroke", keys: ["Ctrl", "C"]}
scripts:
  level_fade: |
    def run(context):
        # Python script snippet
        pass
virtual_console:
  frames:
    - name: "Main"
      pages:
        - name: "Photoshop"
          widgets: ...
```

### Validation & Tooling
- Schema defined in JSON Schema for editor assistance.
- CLI tools provide `validate`, `format`, and `diff` commands.
- Config changes monitored; on save, validator produces diagnostics before cache rebuild.

### Binary Cache
- Compiled representation stored as `*.cache` (e.g., `config/main.v1.cache`).
- Contains:
  - Canonicalized device/page layout tables
  - Pre-parsed macro sequences
  - Bytecode for Python scripts (compiled via CPython, cached `.pyc` equivalent)
  - Hash of source YAML bundle to detect staleness
- Format: bincode-encoded `CacheBundle` with a versioned header.

### Build Pipeline
1. **Source bundle**: runtime watches configured directories (default `config/`).
2. **Merge**: YAML files merged in deterministic order (supporting `import` keys later).
3. **Validate**: Schema + semantic checks (unique IDs, hardware IDs present, macro references resolved).
4. **Compile**:
   - Translate macros to internal instruction lists.
   - Compile Python snippets to bytecode, embed required metadata.
   - Serialize to cache file with header `{version, source_hash, generated_at}`.
5. **Hot Swap**:
   - Runtime receives new cache path, verifies hash, loads into staging memory.
   - Atomically swap active configuration; on failure, revert to previous cache.

### Runtime Loading
- On startup: if cache exists and source hash matches, load directly.
- If cache missing/outdated: trigger compile before completing bootstrap.
- Keep last two cache versions for rollback (`config/.cache_history`).

### Error Handling
- Validation errors reported to UI with precise file/line via diagnostics channel.
- Runtime never applies partially valid configs; continues using last-known-good state.
- Assistant can query diagnostics to guide fixes.

### Semantic Validation Rules (Initial)
- Require `version` to match supported schema versions (starting at `1`).
- Ensure each device declares a non-empty `hardware_id`; IDs must be unique across the config.
- Enforce unique widget `id`s within a device page; warn if duplicates appear globally.
- Verify actions reference existing macros/scripts and those definitions are present.
- Validate macro steps contain required fields (e.g., keystroke has keys, pause has duration > 0).
- Macros marked `ready` are expected to declare a valid MIDI trigger; missing triggers generate warnings, while out-of-range values still surface as errors.
- Assigning the same note to multiple ready macros emits warnings so conflicts can be resolved intentionally.
- Macros marked `draft` surface semantic issues as warnings so authors can iterate without blocking the rest of the config.
- Widgets referencing macros that remain in `draft` state trigger warnings, signaling that the runtime cache will not include those actions until promoted to `ready`.

### Future Extensions
- Support `include:` directives for splitting configs per device or workflow.
- Add binary delta updates for large setups.
- Optional encryption for sensitive macros (keystores), layered atop cache.
## Macro Library Pane
- Collapsible vertical sidebar in the UI for browsing, searching, and organizing macros.
- Macros can be created and edited here independent of controller assignments.
- Provide metadata (tags, usage count) to support filtering and future reminders.
- Drag-and-drop onto device layouts or virtual console widgets.
- Reflect assignment status within the library (e.g., badges showing where a macro is mapped).

## Config Schema Notes
- Macros remain top-level entries with optional metadata (tags, description, usage locations).
- Widgets reference macros by ID; unreferenced macros remain valid and surfaced in the library.
- Validator should warn (not error) about unused macros once UI exposes them.
## Macro Metadata & Status
- Each macro definition gains optional metadata:
  - `status`: `draft` (default) or `ready`.
  - `description`, `tags`, `last_compiled_at`, `last_diagnostics`.
- Validator behavior:
  - `ready` macros must pass semantic validation; failures are reported as errors.
  - `draft` macros may have incomplete steps; issues are logged but do not fail validation.
- Cache compiler only serializes macros marked `ready`, ensuring runtime stability while drafts remain available in the UI.
  - Draft issues are downgraded to warnings in diagnostics to keep iteration fast; the library sidebar can highlight these items for follow-up.
## Cache Compiler Roadmap
- Source of truth: validated `LoadedConfig` from runtime.
- Cache artifact should contain:
  - Version header (schema, build timestamp, source hash).
  - Structured macro/action data (ready macros only).
  - Device/page layout tables with resolved references.
  - Script bytecode or preprocessed form (future work).
- Serialization currently uses bincode (shared with the runtime); future revisions might explore messagepack or flatbuffers if cross-language portability becomes a priority.
- Compiler workflow:
  1. Receive validated config + diagnostics.
  2. Transform to internal IR (sorted tables, IDs).
  3. Serialize IR to cache file (`.cache`).
  4. Emit manifest for debugging/inspection.
- Runtime loading: memory-map or read once at startup, validate checksum, and hydrate in-memory structs.
## MIDI Trigger Mapping
- Each macro may declare a MIDI trigger (`trigger: { type: note, number: 60 }`).
- Validator enforces 0-127 note range; cache carries trigger metadata for runtime lookup.
- Assigning the same note to multiple ready macros produces a warning so authors can deliberately resolve conflicts.
- Executor maintains note->macro map; listener emits note-on events and executor resolves to macro ID.
