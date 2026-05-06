# gpui-preview

A Storybook-like component preview and capture tool for [GPUI](https://gpui.rs), Zed's GPU-accelerated UI framework.

## Features

- **Interactive preview app** with sidebar navigation, search, and live prop editing
- **Derive macro** (`#[derive(Previewable)]`) for zero-config component registration
- **Stories** — named presets saved as JSON, with full create/update/delete from the UI
- **Frame capture** — render components to PNG for visual regression testing and pixel diffing
- **Theme toggle** — switch between light and dark themes in the preview app

## Quick Start

### Preview App

Define your components with `#[derive(Previewable)]` and launch the preview app:

```rust
use gpui::prelude::*;
use gpui::*;
use gpui_component::ActiveTheme as _;
use gpui_preview::Previewable;

#[derive(Clone, Default, Previewable)]
#[preview(category = "Inputs")]
pub struct Button {
    /// The text displayed on the button.
    pub label: String,
    /// Whether the button is interactive.
    pub disabled: bool,
    /// Corner rounding in pixels.
    #[preview(slider(min = 0.0, max = 24.0))]
    pub border_radius: f32,
}

impl RenderOnce for Button {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .px_4().py_2()
            .rounded(px(self.border_radius))
            .bg(cx.theme().primary)
            .text_color(cx.theme().primary_foreground)
            .child(if self.label.is_empty() { "Button".into() } else { self.label })
    }
}

fn main() {
    gpui_preview::run_with_assets(gpui_component_assets::Assets);
}
```

### Frame Capture

Render components to PNG without a persistent UI:

```rust
use gpui::*;

fn main() {
    gpui_preview::capture(gpui_component_assets::Assets, |s| {
        s.png(Button::default(), size(px(200.), px(40.)), "button.png");
        s.png(Card::default(), size(px(300.), px(120.)), "card.png");
    });
}
```

## Using in Your Project

### Basic usage (preview app only)

Add `gpui-preview` as a dependency. No special setup required — the preview app uses upstream gpui.

```toml
[dependencies]
gpui-preview = { path = "..." }  # or version = "..." once published
```

### With frame capture

The `capture` feature requires the vendored gpui fork because upstream gpui does not expose framebuffer readback. Enable the feature and add a `[patch.crates-io]` entry pointing to the fork.

**Option A — reference the fork from this repo directly:**

```toml
[dependencies]
gpui-preview = { path = "...", features = ["capture"] }

[patch.crates-io]
gpui = { git = "https://github.com/Wally869/gpui", branch = "capture-frame" }
```

**Option B — vendor the fork yourself:**

```toml
[dependencies]
gpui-preview = { path = "...", features = ["capture"] }

[patch.crates-io]
gpui = { path = "vendor/gpui" }
```

The patch is transparent to all crates in your dependency tree — `gpui-component` and any other gpui consumers pick it up automatically.

### Why the fork exists

Upstream gpui does not expose a way to read back the rendered framebuffer from the GPU. The fork adds `capture_frame() -> Option<(u32, u32, Vec<u8>)>` to the `PlatformWindow` trait, with platform-specific implementations (D3D11 staging texture on Windows, Metal blit on macOS, Blade transfer pass on Linux). If upstream accepts an equivalent API, the fork and the patch entry can be dropped entirely.

## Project Structure

```
gpui-preview/
  crates/
    gpui-preview/          # Core library — app, capture, stories, registry
    gpui-preview-derive/   # Proc macro for #[derive(Previewable)]
  vendor/
    gpui/                  # Forked gpui 0.2.2 with capture_frame() API
  examples/
    my-app-preview/        # Interactive preview demo
    capture-test/          # Frame capture demo
  docs/
    frame-capture.md       # Capture system architecture
```

## Derive Macro

`#[derive(Previewable)]` generates everything needed to register a component:

| Attribute | Applies to | Description |
|-----------|-----------|-------------|
| `#[preview(category = "...")]` | struct | Sidebar grouping |
| `#[preview(skip)]` | field | Exclude from prop editor |
| `#[preview(slider(min = 0.0, max = 100.0))]` | field | Render as slider control |

Supported field types: `String`, `bool`, `f32`, `f64`, integer types, and enums deriving `Previewable`.

## Stories

Stories are named presets of prop values, persisted in `.preview/stories.json`. Create, select, update, and delete stories from the preview UI. Format:

```json
{
  "my_app::Button": {
    "Primary Large": { "label": "Click me", "border_radius": 8.0 },
    "Disabled": { "label": "Nope", "disabled": true }
  }
}
```

## Frame Capture

The capture system uses a [forked gpui](docs/frame-capture.md) with GPU framebuffer readback on all platforms:

| Platform | Backend | Readback method |
|----------|---------|----------------|
| Windows | DirectX 11 | Staging texture + CopyResource |
| macOS | Metal | Blit to managed texture + getBytes |
| Linux | Blade (Vulkan/GL) | Transfer pass + shared buffer |

The fork is wired in via `[patch.crates-io]` so all dependencies (including `gpui-component`) use it transparently.

## Dependencies

- [gpui](https://gpui.rs) 0.2.2 (forked, vendored)
- [gpui-component](https://crates.io/crates/gpui-component) 0.5.1
- [inventory](https://crates.io/crates/inventory) for zero-config component registration
