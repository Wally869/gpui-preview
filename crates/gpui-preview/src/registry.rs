use crate::traits::Previewable;
use crate::types::{FieldMeta, FieldValue};

/// Object-safe wrapper around `Previewable` for dynamic dispatch.
pub trait AnyPreviewable: std::any::Any {
    /// Get the current value of a field by name.
    fn get_field(&self, name: &str) -> Option<FieldValue>;
    /// Set a field by name, returning the updated component as a new boxed instance.
    fn set_field_boxed(&self, name: &str, value: FieldValue) -> Box<dyn AnyPreviewable>;
    /// Upcast to `&dyn Any` for concrete type recovery.
    fn as_any(&self) -> &dyn std::any::Any;
}

impl<T: Previewable> AnyPreviewable for T {
    fn get_field(&self, name: &str) -> Option<FieldValue> {
        Previewable::get_field(self, name)
    }

    fn set_field_boxed(&self, name: &str, value: FieldValue) -> Box<dyn AnyPreviewable> {
        Box::new(Previewable::set_field(self, name, value))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Render function: clones the current instance and converts it to an AnyElement.
pub type RenderFn = fn(&dyn std::any::Any) -> gpui::AnyElement;

/// A registered component entry, collected automatically via `inventory`.
pub struct PreviewEntry {
    /// Unique identifier — full type path (e.g. `my_app::profile::Button`).
    pub id: fn() -> &'static str,
    /// Human-readable display name (e.g. `Button`).
    pub name: &'static str,
    /// Sidebar grouping label (e.g. `"Inputs"`, `"Layout"`).
    pub category: &'static str,
    /// Short description shown below the component name.
    pub description: &'static str,
    /// Returns metadata for all tweakable fields.
    pub fields: fn() -> Vec<FieldMeta>,
    /// Constructs the default component instance as a type-erased box.
    pub create_default: fn() -> Box<dyn AnyPreviewable>,
    /// Renders the component from a type-erased reference.
    pub render: RenderFn,
}

impl PreviewEntry {
    /// Returns the unique type path for this entry.
    pub fn type_id(&self) -> &'static str {
        (self.id)()
    }
}

// Safety: PreviewEntry only contains function pointers and &'static str references.
unsafe impl Send for PreviewEntry {}
unsafe impl Sync for PreviewEntry {}

inventory::collect!(PreviewEntry);
