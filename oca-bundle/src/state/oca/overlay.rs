use indexmap::IndexMap;
use oca_ast::ast::{NestedValue, OverlayContent};
use overlay_file::OverlayDef;
use said::derivation::HashFunctionCode;
use serde::{Deserialize, Serialize, Serializer};
use said::sad::{SerializationFormats, SAD};
use said::version::SerializationInfo;
use thiserror::Error;
use std::cmp::Ordering;
use std::io::Cursor;
use log::{debug, info};

pub type OverlayName = String;

#[derive(SAD, Deserialize, Debug, Clone)]
#[version(protocol = "OCAS", major = 2, minor = 0)]
pub struct Overlay {
    #[said]
    pub digest: Option<said::SelfAddressingIdentifier>,
    pub capture_base: Option<said::SelfAddressingIdentifier>,
    #[serde(rename = "type")]
    pub name: OverlayName,
    /// List of unique keys for this overlay to differentiate instances. Only one unique instance
    /// allowed per bundle.
    #[serde(skip)]
    pub unique_keys: Option<Vec<String>>,
    #[serde(flatten)]
    pub properties: Option<IndexMap<String, NestedValue>>,
    #[serde(skip)]
    pub overlay_def: Option<OverlayDef>,
}

impl Serialize for Overlay {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap;

        let mut map = serializer.serialize_map(None)?;

        // Serialize attributes in the specified order
        map.serialize_entry("digest", &self.digest)?;
        map.serialize_entry("capture_base", &self.capture_base)?;
        map.serialize_entry("type", &self.name)?;

            // If registry is set, use it to serialize the overlay elements
            for element in self.overlay_def.as_ref().unwrap().elements.iter() {
                if let Some(value) = self.properties.as_ref().and_then(|props| props.get(&element.name)) {
                    map.serialize_entry(&element.name, value)?;
                }
            }

        // Serialize remaining properties in lexicographical order
        if let Some(properties) = &self.properties {
            let mut sorted_properties: Vec<_> = properties.iter().collect();
            sorted_properties.sort_by(|(a, _), (b, _)| a.cmp(b));
            for (key, value) in sorted_properties {
                map.serialize_entry(key, value)?;
            }
        }

        map.end()
    }
}

impl Overlay {
    pub fn new(content: OverlayContent) -> Self {
        Self {
            digest: None,
            name: content.overlay_name,
            unique_keys: None,
            capture_base: None,
            properties: content.properties,
            overlay_def: None,
        }
    }

    pub fn set_capture_base(&mut self, said: &said::SelfAddressingIdentifier) {
        self.capture_base = Some(said.clone());
    }

    // Serialization before we send it to SAD to catch errors as SAD macro does not handle it
    // TODO try to fix it in SAD to properly handle Result from compute digest
    fn check_serialization(&mut self) {
        // Serialize to a temporary buffer
        let mut buffer = Cursor::new(Vec::new());
        match serde_json::to_writer(&mut buffer, self) {
            Ok(_) => {

            },
            Err(e) => {
                println!("Error serializing overlay '{}': {}", self.name, e)
            }
        }
    }

    pub fn fill_said(&mut self) {
        let code = HashFunctionCode::Blake3_256;
        let format = SerializationFormats::JSON;
        self.check_serialization();
        self.compute_digest(&code, &format);
    }

    pub fn calculate_said(&mut self, capture_base_sai: &said::SelfAddressingIdentifier) {
        self.set_capture_base(capture_base_sai);
        self.fill_said();
    }
}

// TODO not used?
fn sort_nested_value(value: &mut NestedValue) {
    match value {
        NestedValue::Object(map) => {
            for (_, v) in map.iter_mut() {
                sort_nested_value(v);
            }
            map.sort_keys();
        }
        NestedValue::Array(arr) => {
            for v in arr.iter_mut() {
                sort_nested_value(v);
            }
            arr.sort_by(|a, b| match (a, b) {
                (NestedValue::Value(a), NestedValue::Value(b)) => a.cmp(b),
                _ => Ordering::Equal,
            });
        }
        _ => {}
    }
}

#[derive(Error, Debug)]
pub enum OverlaySerializationError {
    #[error("No overlay definition found for overlay type: {0}")]
    MissingOverlayDef(String),
}
