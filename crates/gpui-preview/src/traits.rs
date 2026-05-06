use crate::types::{FieldMeta, FieldValue};

/// Describes how a component can be previewed.
/// Derive this on structs to auto-register and generate prop controls.
pub trait Previewable: 'static + Sized + Clone {
    /// Human-readable name shown in the sidebar.
    fn name() -> &'static str;

    /// Optional grouping (e.g., "Inputs", "Layout", "Feedback").
    fn category() -> &'static str {
        "Uncategorized"
    }

    /// Optional description shown below the component name.
    fn description() -> &'static str {
        ""
    }

    /// Returns a default instance used as the starting state.
    fn default_preview() -> Self;

    /// Returns metadata about each tweakable field.
    fn fields() -> Vec<FieldMeta>;

    /// Get the current value of a field by name.
    fn get_field(&self, name: &str) -> Option<FieldValue>;

    /// Set a field by name from a value. Returns a new instance (clone-with-modification).
    fn set_field(&self, name: &str, value: FieldValue) -> Self;
}

/// Trait for enums used as field types in previewable structs.
/// Derive `Previewable` on a unit-variant enum to auto-implement this.
pub trait PreviewEnum: 'static + Sized + Clone {
    /// List of variant names.
    fn variants() -> &'static [&'static str];

    /// Convert this value to its variant name.
    fn to_variant_name(&self) -> &'static str;

    /// Construct a value from a variant name.
    fn from_variant_name(name: &str) -> Option<Self>;
}
