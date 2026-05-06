use gpui_preview::{ControlKind, FieldValue, PreviewEnum, Previewable};

// --- Enum for use as a field type ---
#[derive(Clone, Debug, Default, PartialEq, gpui_preview::Previewable)]
enum TestVariant {
    #[default]
    Alpha,
    Beta,
    Gamma,
}

// --- Struct with all supported field types ---
/// A test component.
#[derive(Clone, Default, gpui_preview::Previewable)]
#[preview(category = "Testing", no_register)]
struct TestComponent {
    /// A text field.
    label: String,
    enabled: bool,
    opacity: f32,
    big_float: f64,
    count: u32,
    signed: i64,
    variant: TestVariant,
    #[preview(skip)]
    internal: usize,
    #[preview(slider(min = 0.0, max = 24.0))]
    radius: f32,
}

// --- Struct with Option and Hsla ---
#[derive(Clone, Default, gpui_preview::Previewable)]
#[preview(no_register)]
struct OptionalFields {
    name: Option<String>,
    flag: Option<bool>,
}

// ── Metadata tests ────────────────────────────────────────────────────

#[test]
fn struct_name() {
    assert_eq!(TestComponent::name(), "TestComponent");
}

#[test]
fn struct_category() {
    assert_eq!(TestComponent::category(), "Testing");
}

#[test]
fn struct_description() {
    assert_eq!(TestComponent::description(), "A test component.");
}

#[test]
fn uncategorized_default() {
    assert_eq!(OptionalFields::category(), "Uncategorized");
}

// ── Fields metadata tests ─────────────────────────────────────────────

#[test]
fn field_list() {
    let fields = TestComponent::fields();
    let names: Vec<_> = fields.iter().map(|f| f.name).collect();
    assert!(names.contains(&"label"));
    assert!(names.contains(&"enabled"));
    assert!(names.contains(&"opacity"));
    assert!(names.contains(&"big_float"));
    assert!(names.contains(&"count"));
    assert!(names.contains(&"signed"));
    assert!(names.contains(&"variant"));
    assert!(names.contains(&"radius"));
    assert!(
        !names.contains(&"internal"),
        "skipped field should not appear"
    );
}

#[test]
fn field_doc_comment() {
    let fields = TestComponent::fields();
    let label = fields.iter().find(|f| f.name == "label").unwrap();
    assert_eq!(label.doc, "A text field.");
}

#[test]
fn field_control_kinds() {
    let fields = TestComponent::fields();
    let find = |name: &str| fields.iter().find(|f| f.name == name).unwrap();

    assert!(matches!(find("label").control, ControlKind::TextInput));
    assert!(matches!(find("enabled").control, ControlKind::Toggle));
    assert!(matches!(
        find("opacity").control,
        ControlKind::NumberSlider { .. }
    ));
    assert!(matches!(
        find("big_float").control,
        ControlKind::NumberSlider { .. }
    ));
    assert!(matches!(
        find("count").control,
        ControlKind::NumberSlider { .. }
    ));
    assert!(matches!(
        find("signed").control,
        ControlKind::NumberSlider { .. }
    ));
    assert!(matches!(find("variant").control, ControlKind::Select(_)));
}

#[test]
fn slider_attribute_bounds() {
    let fields = TestComponent::fields();
    let radius = fields.iter().find(|f| f.name == "radius").unwrap();
    match &radius.control {
        ControlKind::NumberSlider { min, max } => {
            assert_eq!(*min, 0.0);
            assert_eq!(*max, 24.0);
        }
        other => panic!("expected NumberSlider, got {:?}", other),
    }
}

#[test]
fn optional_control_kind() {
    let fields = OptionalFields::fields();
    let name = fields.iter().find(|f| f.name == "name").unwrap();
    assert!(matches!(name.control, ControlKind::Optional(_)));
    if let ControlKind::Optional(inner) = &name.control {
        assert!(matches!(**inner, ControlKind::TextInput));
    }
}

// ── get_field / set_field round-trips ─────────────────────────────────

#[test]
fn get_set_string() {
    let c = TestComponent::default_preview();
    assert!(matches!(c.get_field("label"), Some(FieldValue::String(ref s)) if s.is_empty()));
    let c2 = c.set_field("label", FieldValue::String("Hello".into()));
    assert!(matches!(c2.get_field("label"), Some(FieldValue::String(ref s)) if s == "Hello"));
}

#[test]
fn get_set_bool() {
    let c = TestComponent::default_preview();
    assert!(matches!(
        c.get_field("enabled"),
        Some(FieldValue::Bool(false))
    ));
    let c2 = c.set_field("enabled", FieldValue::Bool(true));
    assert!(matches!(
        c2.get_field("enabled"),
        Some(FieldValue::Bool(true))
    ));
}

#[test]
fn get_set_f32() {
    let c = TestComponent::default_preview();
    let c2 = c.set_field("opacity", FieldValue::Float(0.75));
    match c2.get_field("opacity") {
        Some(FieldValue::Float(v)) => assert!((v - 0.75).abs() < 0.01),
        other => panic!("expected Float, got {:?}", other),
    }
}

#[test]
fn get_set_f64() {
    let c = TestComponent::default_preview();
    let c2 = c.set_field("big_float", FieldValue::Float(3.14));
    match c2.get_field("big_float") {
        Some(FieldValue::Float(v)) => assert!((v - 3.14).abs() < 0.001),
        other => panic!("expected Float, got {:?}", other),
    }
}

#[test]
fn get_set_u32() {
    let c = TestComponent::default_preview();
    let c2 = c.set_field("count", FieldValue::Int(42));
    assert!(matches!(c2.get_field("count"), Some(FieldValue::Int(42))));
}

#[test]
fn get_set_i64() {
    let c = TestComponent::default_preview();
    let c2 = c.set_field("signed", FieldValue::Int(-7));
    assert!(matches!(c2.get_field("signed"), Some(FieldValue::Int(-7))));
}

#[test]
fn get_set_enum() {
    let c = TestComponent::default_preview();
    assert!(matches!(c.get_field("variant"), Some(FieldValue::Enum(ref s)) if s == "Alpha"));
    let c2 = c.set_field("variant", FieldValue::Enum("Beta".into()));
    assert!(matches!(c2.get_field("variant"), Some(FieldValue::Enum(ref s)) if s == "Beta"));
}

#[test]
fn skipped_field_not_accessible() {
    let c = TestComponent::default_preview();
    assert!(c.get_field("internal").is_none());
}

#[test]
fn unknown_field_returns_none() {
    let c = TestComponent::default_preview();
    assert!(c.get_field("nonexistent").is_none());
}

// ── Optional field tests ──────────────────────────────────────────────

#[test]
fn optional_none_default() {
    let c = OptionalFields::default_preview();
    assert!(matches!(c.get_field("name"), Some(FieldValue::None)));
    assert!(matches!(c.get_field("flag"), Some(FieldValue::None)));
}

#[test]
fn optional_set_some() {
    let c = OptionalFields::default_preview();
    let c2 = c.set_field("name", FieldValue::String("test".into()));
    assert!(matches!(c2.get_field("name"), Some(FieldValue::String(ref s)) if s == "test"));
}

#[test]
fn optional_set_back_to_none() {
    let c = OptionalFields::default_preview();
    let c2 = c.set_field("name", FieldValue::String("test".into()));
    let c3 = c2.set_field("name", FieldValue::None);
    assert!(matches!(c3.get_field("name"), Some(FieldValue::None)));
}

#[test]
fn optional_bool() {
    let c = OptionalFields::default_preview();
    let c2 = c.set_field("flag", FieldValue::Bool(true));
    assert!(matches!(c2.get_field("flag"), Some(FieldValue::Bool(true))));
}
