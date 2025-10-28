//! Virtual Console manager applying cache layouts and exposing diagnostics for UI/runtime subsystems.

use crate::config::{CompiledCache, Diagnostic, DiagnosticSeverity};
use cache_format::{DeviceLayout, LayoutPage, LayoutWidget};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WidgetWarning {
    pub device_id: String,
    pub page_index: usize,
    pub page_name: Option<String>,
    pub widget_id: String,
    pub message: String,
}

#[derive(Debug, Default)]
pub struct ConsoleManager {
    pub macro_count: usize,
    pub devices: Vec<DeviceLayout>,
    pub diagnostics: Vec<Diagnostic>,
    widget_warning_cache: Vec<WidgetWarning>,
}

impl ConsoleManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply_cache(&mut self, cache: &CompiledCache) {
        self.macro_count = cache.bundle.macros.len();
        self.devices = cache.bundle.devices.clone();
        self.diagnostics = cache.diagnostics.clone();
        self.rebuild_warning_cache();
    }

    pub fn pages_for_device(&self, device_id: &str) -> Option<&[LayoutPage]> {
        self.devices
            .iter()
            .find(|device| device.id == device_id)
            .map(|device| device.pages.as_slice())
    }

    pub fn widgets_for_page<'a>(
        &'a self,
        device_id: &str,
        page_name: &str,
    ) -> Option<&'a [LayoutWidget]> {
        self.devices
            .iter()
            .find(|device| device.id == device_id)
            .and_then(|device| {
                device
                    .pages
                    .iter()
                    .find(|page| page.name == page_name)
                    .map(|page| page.widgets.as_slice())
            })
    }

    pub fn widget_warnings(
        &self,
        device_id: &str,
        widget_id: &str,
    ) -> impl Iterator<Item = &WidgetWarning> + '_ {
        let device = device_id.to_string();
        let widget = widget_id.to_string();
        self.widget_warning_cache
            .iter()
            .filter(move |warn| warn.device_id == device && warn.widget_id == widget)
    }

    pub fn widget_warning_details(&self) -> &[WidgetWarning] {
        &self.widget_warning_cache
    }

    fn rebuild_warning_cache(&mut self) {
        self.widget_warning_cache.clear();
        for diag in &self.diagnostics {
            if diag.severity != DiagnosticSeverity::Warning {
                continue;
            }
            if let Some(warning) = self.parse_widget_warning(diag) {
                self.widget_warning_cache.push(warning);
            }
        }
    }

    fn parse_widget_warning(&self, diag: &Diagnostic) -> Option<WidgetWarning> {
        let path = &diag.path;
        let devices_prefix = "devices.";
        let device_start = path.find(devices_prefix)? + devices_prefix.len();
        let device_end = path[device_start..].find('.')? + device_start;
        let device_id = &path[device_start..device_end];

        let pages_marker = "pages[";
        let pages_start = path[device_end..].find(pages_marker)? + device_end + pages_marker.len();
        let pages_end = path[pages_start..].find(']')? + pages_start;
        let page_index = path[pages_start..pages_end].parse::<usize>().ok()?;

        let widgets_marker = ".widgets.";
        let widget_start =
            path[pages_end..].find(widgets_marker)? + pages_end + widgets_marker.len();
        let widget_id = &path[widget_start..];

        let page_name = self
            .devices
            .iter()
            .find(|d| d.id == device_id)
            .and_then(|device| device.pages.get(page_index))
            .map(|page| page.name.clone());

        Some(WidgetWarning {
            device_id: device_id.to_string(),
            page_index,
            page_name,
            widget_id: widget_id.to_string(),
            message: diag.message.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cache_format::{
        CacheBundle, CacheHeader, DeviceLayout, LayoutPage, LayoutWidget, MacroEntry, WidgetAction,
    };

    fn sample_cache(count: usize) -> CompiledCache {
        let mut macros = Vec::new();
        for idx in 0..count {
            macros.push(MacroEntry {
                id: format!("m{}", idx),
                description: None,
                tags: vec![],
                trigger: None,
                steps: vec![],
            });
        }
        let bundle = CacheBundle {
            header: CacheHeader {
                version: cache_format::CACHE_VERSION,
                source_hash: 0,
                generated_at: 0,
            },
            devices: vec![DeviceLayout {
                id: "launchpad".into(),
                hardware_id: Some("usb:demo.launchpad".into()),
                pages: vec![LayoutPage {
                    name: "Main".into(),
                    widgets: vec![LayoutWidget {
                        id: "pad_1".into(),
                        tap_behavior: Some("tap".into()),
                        action: Some(WidgetAction::Macro { id: "m0".into() }),
                    }],
                }],
            }],
            macros,
        };
        CompiledCache {
            bundle,
            diagnostics: vec![],
            bytes: vec![],
        }
    }

    #[test]
    fn apply_cache_updates_macro_count_and_devices() {
        let mut manager = ConsoleManager::new();
        manager.apply_cache(&sample_cache(3));
        assert_eq!(manager.macro_count, 3);
        assert_eq!(manager.devices.len(), 1);
        assert!(manager.widget_warning_details().is_empty());
        let pages = manager.pages_for_device("launchpad").expect("pages");
        assert_eq!(pages.len(), 1);
        let widgets = manager
            .widgets_for_page("launchpad", "Main")
            .expect("widgets");
        assert_eq!(widgets.len(), 1);
        assert_eq!(widgets[0].id, "pad_1");
    }

    #[test]
    fn widget_warning_lookup_matches_diagnostics() {
        let mut cache = sample_cache(1);
        cache.diagnostics.push(Diagnostic {
            path: "devices.launchpad.pages[0].widgets.pad_1".into(),
            message:
                "References macro `draft_macro` that is not marked ready and will not be compiled"
                    .into(),
            location: None,
            severity: DiagnosticSeverity::Warning,
        });

        let mut manager = ConsoleManager::new();
        manager.apply_cache(&cache);

        let warnings = manager.widget_warning_details();
        assert_eq!(warnings.len(), 1);
        let warn = &warnings[0];
        assert_eq!(warn.device_id, "launchpad");
        assert_eq!(warn.page_index, 0);
        assert_eq!(warn.page_name.as_deref(), Some("Main"));
        assert_eq!(warn.widget_id, "pad_1");
        assert!(warn
            .message
            .contains("not marked ready and will not be compiled"));

        let widget_specific: Vec<_> = manager.widget_warnings("launchpad", "pad_1").collect();
        assert_eq!(widget_specific.len(), 1);
        assert_eq!(widget_specific[0], warn);
    }
}
