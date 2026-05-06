/// What kind of control to render in the prop editor.
#[derive(Debug, Clone)]
pub enum ControlKind {
    /// Single-line text input (for `String`).
    TextInput,
    /// Boolean toggle (for `bool`).
    Toggle,
    /// Numeric slider with min/max bounds (for `f32`, `f64`, integers).
    NumberSlider { min: f64, max: f64 },
    /// Dropdown/radio select from named variants (for enums).
    Select(Vec<&'static str>),
    /// RGBA color picker.
    Color,
    /// Wraps another control, adding a None/Some toggle (for `Option<T>`).
    Optional(Box<ControlKind>),
    /// User-defined custom control.
    Custom(String),
    /// Fallback for unsupported types — displays type name, no interaction.
    Unsupported,
}

/// Runtime values passed between the prop editor and the component.
#[derive(Debug, Clone)]
pub enum FieldValue {
    /// UTF-8 string value.
    String(String),
    /// Boolean value.
    Bool(bool),
    /// 64-bit floating-point value.
    Float(f64),
    /// 64-bit signed integer value.
    Int(i64),
    /// Enum variant name.
    Enum(String),
    /// RGBA color as four bytes `[r, g, b, a]`.
    Color([u8; 4]),
    /// Represents the `None` state of an `Option<T>` field.
    None,
}

/// Metadata about a single tweakable field.
#[derive(Debug, Clone)]
pub struct FieldMeta {
    /// Field name as it appears in source (used as key for get/set).
    pub name: &'static str,
    /// Optional doc string shown as a tooltip in the prop editor.
    pub doc: &'static str,
    /// Control type to render for this field.
    pub control: ControlKind,
}
