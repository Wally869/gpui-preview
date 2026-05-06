use gpui::*;
use gpui_component::ActiveTheme as _;

#[derive(Clone)]
struct TestButton {
    label: String,
}

impl Default for TestButton {
    fn default() -> Self {
        Self {
            label: "Hello from GPUI!".into(),
        }
    }
}

impl RenderOnce for TestButton {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .px_6()
            .py_3()
            .rounded(px(8.))
            .bg(cx.theme().primary)
            .text_color(cx.theme().primary_foreground)
            .text_base()
            .font_weight(FontWeight::SEMIBOLD)
            .child(self.label)
    }
}

#[derive(Clone, Default)]
struct TestCard;

impl RenderOnce for TestCard {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .w(px(300.))
            .p(px(16.))
            .rounded(px(8.))
            .bg(cx.theme().secondary)
            .border_1()
            .border_color(cx.theme().border)
            .child(
                div()
                    .text_base()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Capture Test Card"),
            )
            .child(
                div()
                    .mt_2()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("If you see this as a PNG, frame capture works!"),
            )
    }
}

fn main() {
    gpui_preview::capture(gpui_component_assets::Assets, |s| {
        s.png(TestButton::default(), size(px(250.), px(50.)), "button.png");
        s.png(TestCard, size(px(320.), px(120.)), "card.png");
    });
}
