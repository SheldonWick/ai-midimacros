use cache_builder::build_from_str;
use cache_format::{MacroStep, WidgetAction};

#[test]
fn cache_bundle_matches_ready_macros() {
    let yaml = r#"version: 1
devices:
  launchpad:
    hardware_id: "usb:demo.launchpad"
    pages:
      - name: "Main"
        widgets:
          - id: "pad_1"
            tap_behavior: "tap"
            action:
              type: macro
              ref: ready_one
macros:
  ready_one:
    status: ready
    trigger:
      type: note
      number: 60
    steps:
      - type: keystroke
        keys: ["A"]
  ready_two:
    status: ready
    trigger:
      type: note
      number: 61
    steps:
      - type: pause
        ms: 10
  draft:
    status: draft
    steps:
      - type: keystroke
        keys: []
scripts: {}
"#;

    let output = build_from_str(yaml).expect("build");
    assert_eq!(output.bundle.macros.len(), 2);
    let ids: Vec<_> = output.bundle.macros.iter().map(|m| m.id.as_str()).collect();
    assert!(ids.contains(&"ready_one"));
    assert!(ids.contains(&"ready_two"));
    assert_eq!(output.bundle.devices.len(), 1);
    let device = &output.bundle.devices[0];
    assert_eq!(device.id, "launchpad");
    assert_eq!(device.hardware_id.as_deref(), Some("usb:demo.launchpad"));
    assert_eq!(device.pages.len(), 1);
    let page = &device.pages[0];
    assert_eq!(page.name, "Main");
    assert_eq!(page.widgets.len(), 1);
    let widget = &page.widgets[0];
    assert_eq!(widget.id, "pad_1");
    assert_eq!(widget.tap_behavior.as_deref(), Some("tap"));
    match &widget.action {
        Some(WidgetAction::Macro { id }) => assert_eq!(id, "ready_one"),
        other => panic!("unexpected widget action: {:?}", other),
    }

    match &output.bundle.macros[0].steps[0] {
        MacroStep::Keystroke { .. } | MacroStep::Pause { .. } => {}
    }
}
