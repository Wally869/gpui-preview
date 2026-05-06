//! Storybook-like component preview and capture tool for GPUI.

mod app;
#[cfg(feature = "capture")]
pub mod capture;
mod error;
mod registry;
mod stories;
mod traits;
mod types;

pub use app::PreviewApp;
#[cfg(feature = "capture")]
pub use capture::capture;
pub use error::PreviewError;
pub use registry::{AnyPreviewable, PreviewEntry};
pub use traits::{PreviewEnum, Previewable};
pub use types::{ControlKind, FieldMeta, FieldValue};

// Re-export the derive macro
pub use gpui_preview_derive::Previewable;

// Re-export inventory so generated code can reference it
pub use inventory;

use gpui::*;

pub use app::{CloseDialog, SelectNext, SelectPrev};

/// Launch the preview app without icon assets.
pub fn run() {
    let app = Application::new();
    launch(app);
}

/// Launch the preview app with icon assets (e.g. `gpui_component_assets::Assets`).
pub fn run_with_assets(assets: impl AssetSource + 'static) {
    let app = Application::new().with_assets(assets);
    launch(app);
}

fn launch(app: Application) {
    app.run(move |cx| {
        gpui_component::init(cx);

        cx.bind_keys([
            KeyBinding::new("up", SelectPrev, None),
            KeyBinding::new("down", SelectNext, None),
            KeyBinding::new("escape", CloseDialog, None),
        ]);

        cx.activate(true);

        let window_size = size(px(1400.), px(900.));
        let bounds = Bounds::centered(None, window_size, cx);

        cx.spawn(async move |cx| {
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(800.), px(500.))),
                titlebar: Some(TitlebarOptions {
                    title: Some("gpui-preview".into()),
                    ..Default::default()
                }),
                ..Default::default()
            };

            cx.open_window(options, |window, cx| {
                let view = cx.new(|cx| PreviewApp::new(window, cx));
                cx.new(|cx| gpui_component::Root::new(view, window, cx))
            })?;

            Ok::<_, anyhow::Error>(())
        })
        .detach();
    });
}
