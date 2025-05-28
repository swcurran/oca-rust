use indexmap::IndexMap;
use oca_ast::ast::NestedValue;
use overlay_file::overlay_registry::{OverlayLocalRegistry, OverlayRegistry};
use said::derivation::HashFunctionCode;
use serde::{Deserialize, Serialize, Serializer};
use serde::ser::Error as SerdeError;
use said::sad::{SerializationFormats, SAD};
use said::version::SerializationInfo;
use thiserror::Error;
use std::cmp::Ordering;
use std::io::Cursor;
use log::{debug, info};

pub type OverlayName = String;

use std::cell::RefCell;

thread_local! {
    static GLOBAL_REGISTRY: RefCell<Option<OverlayLocalRegistry>> = RefCell::new(None);
}

pub fn set_global_registry(registry: OverlayLocalRegistry) {
    GLOBAL_REGISTRY.with(|r| *r.borrow_mut() = Some(registry));
}

pub fn get_global_registry() -> Option<OverlayLocalRegistry> {
    GLOBAL_REGISTRY.with(|r| r.borrow().clone())
}


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

        // Use the global registry to serialize the content
        let registry = get_global_registry().ok_or_else(|| {
            serde::ser::Error::custom("Global registry not set")
        })?;
        // Fetch OverlayDef from registry
        match registry.get_by_name(&self.name).unwrap() {
            Some(overlay_def) => {
                if let Some(properties) = &self.properties {
                    for element in &overlay_def.elements {
                        if let Some(value) = properties.get(&element.name) {
                            map.serialize_entry(&element.name, value)?;
                        }
                    }
                }
            },
            None => {
                debug!("Overlay '{}' not found in registry. This may indicate a mismatch between the overlay definition and the current registry state.", self.name);
                return Err(S::Error::custom(format!(
                    "Overlay '{}' not found in registry. This may indicate a mismatch between the overlay definition and the current registry state.",
                    self.name
                )));
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
    pub fn new(name: OverlayName) -> Self {
        Self {
            digest: None,
            name,
            unique_keys: None,
            capture_base: None,
            properties: None,
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
