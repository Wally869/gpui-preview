# How to Use gpui-preview

## Setup

Add the dependencies to your `Cargo.toml`:

```toml
[dependencies]
gpui = "0.2.2"
gpui-component = "0.5.1"
gpui-component-assets = "0.5.1"
gpui-preview = { path = "path/to/gpui-preview/crates/gpui-preview" }
```

## Making a Component Previewable

### 1. Derive `Previewable` on your struct

Your component must implement `Clone`, `Default`, and `RenderOnce`.

```rust
use gpui::prelude::*;
use gpui::*;
use gpui_preview::Previewable;

#[derive(Clone, Default, Previewable)]
#[preview(category = "Inputs")]
pub struct MyButton {
    /// Tooltip shown in the prop editor.
    pub label: String,
    pub disabled: bool,
}

impl RenderOnce for MyButton {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div().child(self.label)
    }
}
```

That's it — the component auto-registers via `inventory` and appears in the sidebar.

### 2. Derive `Previewable` on enums

Enum field types need their own `#[derive(Previewable)]`. Only unit variants are supported.

```rust
#[derive(Clone, Default, Previewable)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Ghost,
}
```

Use it as a field type and it renders as a dropdown in the prop editor:

```rust
#[derive(Clone, Default, Previewable)]
pub struct MyButton {
    pub variant: ButtonVariant,
    // ...
}
```

### 3. Launch the preview app

```rust
fn main() {
    gpui_preview::run_with_assets(gpui_component_assets::Assets);
}
```

Or without icon assets:

```rust
fn main() {
    gpui_preview::run();
}
```

## Attribute Reference

All attributes go on the `#[preview(...)]` helper.

### On structs

| Attribute | Example | Effect |
|-----------|---------|--------|
| `category` | `#[preview(category = "Layout")]` | Groups the component in the sidebar |

### On fields

| Attribute | Example | Effect |
|-----------|---------|--------|
| `skip` | `#[preview(skip)]` | Hides the field from the prop editor |
| `slider(min, max)` | `#[preview(slider(min = 0.0, max = 24.0))]` | Renders a slider instead of a plain number input |

## Supported Field Types

| Rust Type | Control | FieldValue |
|-----------|---------|------------|
| `String` | Text input | `String(s)` |
| `bool` | Toggle switch | `Bool(b)` |
| `f32`, `f64` | Number slider (0–100) | `Float(f)` |
| `u8`–`u64`, `i8`–`i64`, `usize`, `isize` | Number slider (0–100) | `Int(i)` |
| `Hsla` | Color picker | `Color([r,g,b,a])` |
| `Option<T>` | None/Some toggle + inner control | `None` or inner value |
| Enum (unit variants) | Dropdown select | `Enum(variant_name)` |

## Stories

Stories are named presets of prop values. They persist across sessions in `.preview/stories.json`.

### Creating a story
1. Adjust the component's props in the editor
2. Click "Save Story" in the story panel
3. Enter a name (e.g. "Primary Large")

### Loading a story
Click any saved story name in the list to restore its props.

### Deleting a story
Click the delete button next to a story name.

### JSON format

```json
{
  "version": 1,
  "components": {
    "my_app::MyButton": {
      "Primary Large": {
        "label": "Click me",
        "disabled": false,
        "border_radius": 8.0
      }
    }
  }
}
```

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `Up` | Select previous component |
| `Down` | Select next component |
| `Escape` | Close dialogs |

## Viewport Presets

The toolbar above the canvas has size presets:

| Preset | Width |
|--------|-------|
| S | 320px |
| M | 640px |
| L | 960px |
| Full | Fill available space |

## Frame Capture (optional)

Render components to PNG files for visual regression testing. Requires the `capture` feature and a [forked gpui](frame-capture.md) with framebuffer readback.

```toml
[dependencies]
gpui-preview = { path = "...", features = ["capture"] }

[patch.crates-io]
gpui = { path = "path/to/vendor/gpui" }
```

```rust
fn main() {
    gpui_preview::capture(gpui_component_assets::Assets, |s| {
        s.png(MyButton::default(), size(px(200.), px(40.)), "button.png");
    });
}
```

## Tips

- Doc comments on structs become the description shown in the sidebar
- Doc comments on fields become tooltips in the prop editor
- Use `#[preview(skip)]` for internal state fields (click counters, animation state, etc.)
- The `Default` impl determines the initial prop values shown in the editor
- Components register globally via `inventory` — no manual registry needed
