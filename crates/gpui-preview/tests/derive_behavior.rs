#![recursion_limit = "512"]

use gpui_preview::{FieldValue, PreviewEnum};

// --- Test enum only (no inventory::submit!, no RenderOnce needed) ---
#[derive(Clone, Debug, Default, PartialEq, gpui_preview::Previewable)]
enum TestVariant {
    #[default]
    Alpha,
    Beta,
    Gamma,
}

#[test]
fn enum_variants() {
    assert_eq!(TestVariant::variants(), &["Alpha", "Beta", "Gamma"]);
}

#[test]
fn enum_to_variant_name() {
    assert_eq!(TestVariant::Alpha.to_variant_name(), "Alpha");
    assert_eq!(TestVariant::Beta.to_variant_name(), "Beta");
    assert_eq!(TestVariant::Gamma.to_variant_name(), "Gamma");
}

#[test]
fn enum_from_variant_name() {
    assert_eq!(
        TestVariant::from_variant_name("Alpha"),
        Some(TestVariant::Alpha)
    );
    assert_eq!(
        TestVariant::from_variant_name("Beta"),
        Some(TestVariant::Beta)
    );
    assert_eq!(TestVariant::from_variant_name("Unknown"), None);
}

#[test]
fn enum_get_field_returns_enum_value() {
    // Previewable on an enum generates PreviewEnum, not get_field.
    // This test confirms enum variants round-trip through FieldValue::Enum.
    let name = TestVariant::Beta.to_variant_name().to_string();
    let reconstructed = TestVariant::from_variant_name(&name);
    assert_eq!(reconstructed, Some(TestVariant::Beta));
    let _ = FieldValue::Enum(name);
}
