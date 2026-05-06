use gpui_preview::Previewable;

#[derive(Clone, Default, Previewable)]
enum Foo {
    #[default]
    A,
    B(String),
}

fn main() {}
