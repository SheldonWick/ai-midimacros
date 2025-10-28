# Config Validator

CLI tool for validating YAML configuration bundles and (eventually) compiling them into binary caches.

## Roadmap
- Load YAML using `serde_yaml` into strongly typed structs.
- Enforce schema constraints (unique IDs, known action types, required references).
- Emit diagnostics with file/line context.
- Future: generate binary cache artifacts.
- Warnings for unused macros/scripts to support the macro library UX.
- Respect macro `status`: `ready` macros must pass validation; `draft` macros produce informational diagnostics.

- Integrate with cache builder once format is defined to ensure compatibility checks.
