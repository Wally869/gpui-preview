use gpui::prelude::*;
use gpui::*;
use gpui_component::ActiveTheme as _;
use gpui_preview::Previewable;

/// A standard button component.
#[derive(Clone, Default, Previewable)]
#[preview(category = "Inputs")]
pub struct Button {
    /// The text displayed on the button.
    pub label: String,

    /// Visual style variant.
    pub variant: ButtonVariant,

    /// Whether the button is interactive.
    pub disabled: bool,

    /// Corner rounding in pixels.
    #[preview(slider(min = 0.0, max = 24.0))]
    pub border_radius: f32,

    /// Internal state — excluded from the preview panel.
    #[preview(skip)]
    pub click_count: usize,
}

impl RenderOnce for Button {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let label = if self.label.is_empty() {
            "Button".to_string()
        } else {
            self.label
        };

        let (bg, fg) = match self.variant {
            ButtonVariant::Primary => (cx.theme().primary, cx.theme().primary_foreground),
            ButtonVariant::Secondary => (cx.theme().secondary, cx.theme().secondary_foreground),
            ButtonVariant::Ghost => (gpui::transparent_black(), cx.theme().foreground),
            ButtonVariant::Danger => (cx.theme().danger, cx.theme().danger_foreground),
        };

        div()
            .px_4()
            .py_2()
            .rounded(px(self.border_radius))
            .bg(bg)
            .text_color(fg)
            .text_sm()
            .font_weight(FontWeight::MEDIUM)
            .cursor_pointer()
            .hover(|el| el.opacity(0.8))
            .when(self.disabled, |el| el.opacity(0.5).cursor_default())
            .child(label)
    }
}

/// Visual style options for buttons.
#[derive(Clone, Default, Previewable)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Ghost,
    Danger,
}

/// A simple card container.
#[derive(Clone, Default, Previewable)]
#[preview(category = "Layout")]
pub struct Card {
    /// Card title text.
    pub title: String,

    /// Whether the card has a visible border.
    pub bordered: bool,

    /// Padding in pixels.
    #[preview(slider(min = 0.0, max = 48.0))]
    pub padding: f32,
}

impl RenderOnce for Card {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let title = if self.title.is_empty() {
            "Card Title".to_string()
        } else {
            self.title
        };

        div()
            .w(px(320.))
            .p(px(self.padding))
            .rounded(px(8.))
            .bg(cx.theme().secondary)
            .when(self.bordered, |el| {
                el.border_1().border_color(cx.theme().border)
            })
            .child(
                div()
                    .text_base()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(title),
            )
            .child(
                div()
                    .mt_2()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("Card content goes here. This is a preview of the card layout."),
            )
    }
}

/// A colored badge with optional subtitle.
#[derive(Clone, Previewable)]
#[preview(category = "Display")]
pub struct Badge {
    /// Badge text.
    pub label: String,

    /// Background color.
    pub color: Hsla,

    /// Optional subtitle below the badge.
    pub subtitle: Option<String>,
}

impl Default for Badge {
    fn default() -> Self {
        Self {
            label: String::new(),
            color: gpui::hsla(0.6, 0.7, 0.5, 1.0),
            subtitle: None,
        }
    }
}

impl RenderOnce for Badge {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let label = if self.label.is_empty() {
            "Badge".into()
        } else {
            self.label
        };
        let container = div()
            .px_3()
            .py_1()
            .rounded(px(12.))
            .bg(self.color)
            .text_color(cx.theme().primary_foreground)
            .text_xs()
            .font_weight(FontWeight::SEMIBOLD)
            .child(label);

        if let Some(sub) = self.subtitle {
            div()
                .flex()
                .flex_col()
                .gap_1()
                .items_start()
                .child(container)
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(sub),
                )
        } else {
            div().child(container)
        }
    }
}

/// A toggle switch component.
#[derive(Clone, Default, Previewable)]
#[preview(category = "Inputs")]
pub struct Toggle {
    /// Label displayed next to the toggle.
    pub label: String,

    /// Current on/off state.
    pub checked: bool,
}

impl RenderOnce for Toggle {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let label = if self.label.is_empty() {
            "Toggle".to_string()
        } else {
            self.label
        };

        let (track_bg, knob_offset) = if self.checked {
            (cx.theme().primary, px(18.))
        } else {
            (cx.theme().muted, px(2.))
        };

        div()
            .flex()
            .items_center()
            .gap_3()
            .child(
                div()
                    .w(px(36.))
                    .h(px(20.))
                    .rounded(px(10.))
                    .bg(track_bg)
                    .relative()
                    .child(
                        div()
                            .absolute()
                            .top(px(2.))
                            .left(knob_offset)
                            .w(px(16.))
                            .h(px(16.))
                            .rounded(px(8.))
                            .bg(gpui::white()),
                    ),
            )
            .child(div().text_sm().child(label))
    }
}

fn main() {
    gpui_preview::run_with_assets(gpui_component_assets::Assets);
}
