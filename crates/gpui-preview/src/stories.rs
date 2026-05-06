use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::types::{ControlKind, FieldMeta, FieldValue};

const CURRENT_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
struct StoriesFile {
    version: u32,
    components: AllStories,
}

/// Per-story field values: field name → JSON value.
pub type StoryFields = HashMap<String, serde_json::Value>;

/// All stories for one component: story name → field values.
pub type StoryMap = HashMap<String, StoryFields>;

/// All stories for all components: component name → stories.
pub type AllStories = HashMap<String, StoryMap>;

pub fn stories_path() -> PathBuf {
    PathBuf::from(".preview/stories.json")
}

pub fn load_stories() -> AllStories {
    let path = stories_path();
    let Ok(data) = std::fs::read_to_string(&path) else {
        return AllStories::new();
    };
    if let Ok(file) = serde_json::from_str::<StoriesFile>(&data) {
        return file.components;
    }
    // Fall back to old flat format (version 0).
    if let Ok(stories) = serde_json::from_str::<AllStories>(&data) {
        eprintln!(
            "gpui-preview: migrating stories.json from version 0 to version {}",
            CURRENT_VERSION
        );
        return stories;
    }
    AllStories::new()
}

pub fn save_stories(stories: &AllStories) {
    let path = stories_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let file = StoriesFile {
        version: CURRENT_VERSION,
        components: stories.clone(),
    };
    if let Ok(json) = serde_json::to_string_pretty(&file) {
        let _ = std::fs::write(&path, json);
    }
}

pub fn field_value_to_json(value: &FieldValue) -> serde_json::Value {
    match value {
        FieldValue::String(s) => serde_json::Value::String(s.clone()),
        FieldValue::Bool(b) => serde_json::Value::Bool(*b),
        FieldValue::Float(f) => serde_json::json!(*f),
        FieldValue::Int(i) => serde_json::json!(*i),
        FieldValue::Enum(s) => serde_json::Value::String(s.clone()),
        FieldValue::Color(c) => serde_json::json!(c),
        FieldValue::None => serde_json::Value::Null,
    }
}

pub fn json_to_field_value(json: &serde_json::Value, control: &ControlKind) -> Option<FieldValue> {
    match (json, control) {
        (serde_json::Value::String(s), ControlKind::TextInput) => {
            Some(FieldValue::String(s.clone()))
        }
        (serde_json::Value::String(s), ControlKind::Select(_)) => Some(FieldValue::Enum(s.clone())),
        (serde_json::Value::Bool(b), _) => Some(FieldValue::Bool(*b)),
        (serde_json::Value::Number(n), ControlKind::NumberSlider { .. }) => {
            Some(FieldValue::Float(n.as_f64()?))
        }
        (serde_json::Value::Number(n), _) => {
            if let Some(i) = n.as_i64() {
                Some(FieldValue::Int(i))
            } else {
                Some(FieldValue::Float(n.as_f64()?))
            }
        }
        (serde_json::Value::Array(arr), ControlKind::Color) if arr.len() == 4 => {
            let rgba: Vec<u8> = arr
                .iter()
                .filter_map(|v| v.as_u64().map(|n| n as u8))
                .collect();
            if rgba.len() == 4 {
                Some(FieldValue::Color([rgba[0], rgba[1], rgba[2], rgba[3]]))
            } else {
                None
            }
        }
        (serde_json::Value::Null, ControlKind::Optional(_)) => Some(FieldValue::None),
        (val, ControlKind::Optional(inner)) => json_to_field_value(val, inner),
        (serde_json::Value::Null, _) => Some(FieldValue::None),
        // Fallback: try string for unknown controls
        (serde_json::Value::String(s), _) => Some(FieldValue::String(s.clone())),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ControlKind;

    // --- field_value_to_json / json_to_field_value round-trips ---

    fn roundtrip(value: &FieldValue, control: &ControlKind) -> Option<FieldValue> {
        let json = field_value_to_json(value);
        json_to_field_value(&json, control)
    }

    #[test]
    fn roundtrip_string() {
        let v = FieldValue::String("hello".to_string());
        let out = roundtrip(&v, &ControlKind::TextInput).unwrap();
        assert!(matches!(out, FieldValue::String(s) if s == "hello"));
    }

    #[test]
    fn roundtrip_bool_true() {
        let v = FieldValue::Bool(true);
        let out = roundtrip(&v, &ControlKind::Toggle).unwrap();
        assert!(matches!(out, FieldValue::Bool(true)));
    }

    #[test]
    fn roundtrip_bool_false() {
        let v = FieldValue::Bool(false);
        let out = roundtrip(&v, &ControlKind::Toggle).unwrap();
        assert!(matches!(out, FieldValue::Bool(false)));
    }

    #[test]
    fn roundtrip_float() {
        let v = FieldValue::Float(3.14);
        let out = roundtrip(
            &v,
            &ControlKind::NumberSlider {
                min: 0.0,
                max: 10.0,
            },
        )
        .unwrap();
        assert!(matches!(out, FieldValue::Float(f) if (f - 3.14).abs() < 1e-9));
    }

    #[test]
    fn roundtrip_int() {
        let v = FieldValue::Int(42);
        // Generic number control (not NumberSlider) → Int path
        let out = roundtrip(&v, &ControlKind::Unsupported).unwrap();
        assert!(matches!(out, FieldValue::Int(42)));
    }

    #[test]
    fn roundtrip_enum() {
        let v = FieldValue::Enum("Large".to_string());
        let out = roundtrip(&v, &ControlKind::Select(vec!["Small", "Large"])).unwrap();
        assert!(matches!(out, FieldValue::Enum(s) if s == "Large"));
    }

    #[test]
    fn roundtrip_color() {
        let v = FieldValue::Color([255, 128, 0, 200]);
        let out = roundtrip(&v, &ControlKind::Color).unwrap();
        assert!(matches!(out, FieldValue::Color([255, 128, 0, 200])));
    }

    #[test]
    fn roundtrip_none() {
        let v = FieldValue::None;
        let out = roundtrip(&v, &ControlKind::Optional(Box::new(ControlKind::TextInput))).unwrap();
        assert!(matches!(out, FieldValue::None));
    }

    // --- versioned format: write JSON manually, parse with StoriesFile ---

    #[test]
    fn versioned_format_round_trip() {
        // Build a small AllStories map
        let mut story_fields = StoryFields::new();
        story_fields.insert("label".to_string(), serde_json::json!("Click me"));

        let mut story_map = StoryMap::new();
        story_map.insert("default".to_string(), story_fields);

        let mut all_stories = AllStories::new();
        all_stories.insert("Button".to_string(), story_map);

        // Serialize via StoriesFile (same as save_stories does internally)
        let file = StoriesFile {
            version: CURRENT_VERSION,
            components: all_stories.clone(),
        };
        let json = serde_json::to_string_pretty(&file).unwrap();

        // Deserialize back
        let parsed: StoriesFile = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.version, CURRENT_VERSION);
        let label = parsed.components["Button"]["default"]["label"]
            .as_str()
            .unwrap();
        assert_eq!(label, "Click me");
    }

    // --- old flat format migration ---

    #[test]
    fn old_format_migration() {
        // Old format: plain AllStories JSON (no version wrapper)
        let mut story_fields = StoryFields::new();
        story_fields.insert("size".to_string(), serde_json::json!("medium"));

        let mut story_map = StoryMap::new();
        story_map.insert("default".to_string(), story_fields);

        let mut all_stories = AllStories::new();
        all_stories.insert("Card".to_string(), story_map);

        let old_json = serde_json::to_string(&all_stories).unwrap();

        // Try parsing as StoriesFile first (should fail), then as AllStories (should succeed)
        assert!(serde_json::from_str::<StoriesFile>(&old_json).is_err());
        let migrated: AllStories = serde_json::from_str(&old_json).unwrap();
        let size = migrated["Card"]["default"]["size"].as_str().unwrap();
        assert_eq!(size, "medium");
    }
}

/// Snapshot current field values from an instance into a StoryFields map.
pub fn snapshot_fields(
    instance: &dyn crate::registry::AnyPreviewable,
    fields: &[FieldMeta],
) -> StoryFields {
    let mut map = StoryFields::new();
    for field in fields {
        if let Some(value) = instance.get_field(field.name) {
            map.insert(field.name.to_string(), field_value_to_json(&value));
        }
    }
    map
}
