use indexmap::IndexMap;
use oca_ast::ast::{NestedValue, OverlayContent};
use overlay_file::OverlayDef;
use said::{derivation::HashFunctionCode, SelfAddressingIdentifier};
use said::make_me_happy;
use serde::{Deserialize, Serialize, Serializer};
use thiserror::Error;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use log::{debug, info};

pub type OverlayName = String;

// #[derive(Deserialize, Debug, Clone)]
// // #[version(protocol = "OCAS", major = 2, minor = 0)]
// pub struct Overlay {
//     pub digest: Option<said::SelfAddressingIdentifier>,
//     pub capture_base: Option<said::SelfAddressingIdentifier>,
//     #[serde(rename = "type")]
//     pub name: OverlayName,
//     /// List of unique keys for this overlay to differentiate instances. Only one unique instance
//     /// allowed per bundle.
//     #[serde(skip)]
//     pub unique_keys: Option<Vec<String>>,
//     #[serde(flatten)]
//     pub properties: Option<IndexMap<String, NestedValue>>,
//     #[serde(skip)]
//     pub overlay_def: Option<OverlayDef>,
// }

/// Overlay struct is used only for serialization purposes.
/// Internally OverlayModel should be used.
#[derive(Deserialize, Debug, Clone)]
pub struct Overlay {
    model: OverlayModel,
}


#[derive(Deserialize, Debug, Clone)]
pub struct OverlayModel {
    pub digest: Option<said::SelfAddressingIdentifier>,
    #[serde(rename = "capture_base")]
    pub capture_base_said: Option<said::SelfAddressingIdentifier>,
    #[serde(rename = "type")]
    pub name: OverlayName,
    /// List of unique keys for this overlay to differentiate instances. Only one unique instance
    /// allowed per bundle.
    pub unique_keys: Option<Vec<String>>,
    pub properties: Option<IndexMap<String, NestedValue>>,
    pub overlay_def: Option<OverlayDef>,
}

impl Serialize for Overlay {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap;
        debug!("Serializing overlay for: {}", self.model.name);

        let mut map = serializer.serialize_map(None)?;

        // Serialize attributes in the specified order
        map.serialize_entry("digest", &self.model.digest)?;
        map.serialize_entry("capture_base", &self.model.capture_base_said)?;
        map.serialize_entry("type", &self.model.name)?;

            // If registry is set, use it to serialize the overlay elements
            for element in self.model.overlay_def.as_ref().unwrap().elements.iter() {
                if let Some(value) = self.model.properties.as_ref().and_then(|props| props.get(&element.name)) {
                    map.serialize_entry(&element.name, value)?;
                }
            }

        // Serialize remaining properties in lexicographical order
        if let Some(properties) = &self.model.properties {
            let mut sorted_properties: Vec<_> = properties.iter().collect();
            sorted_properties.sort_by(|(a, _), (b, _)| a.cmp(b));
            for (key, value) in sorted_properties {
                map.serialize_entry(key, value)?;
            }
        }

        map.end()
    }
}

impl Serialize for OverlayModel {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap;
        debug!("Serializing overlay model for: {}", self.name);
        let mut map = serializer.serialize_map(None)?;

        // Serialize attributes in the specified order
        map.serialize_entry("digest", &self.digest)?;
        map.serialize_entry("capture_base", &self.capture_base_said)?;
        map.serialize_entry("type", &self.name)?;
        map.serialize_entry("unique_keys", &self.unique_keys)?;
        map.serialize_entry("overlay_def", &self.overlay_def)?;

        let mut props = BTreeMap::new();
        // Use overlay definition to serialize elements in the correct order
        for element in self.overlay_def.as_ref().unwrap().elements.iter() {
            if let Some(value) = self.properties.as_ref().and_then(|props| props.get(&element.name)) {
                props.insert(element.name.clone(), value.clone());
            }
        }

        // Serialize remaining properties in lexicographical order
        if let Some(properties) = &self.properties {
            let mut sorted_properties: Vec<_> = properties.iter().collect();
            sorted_properties.sort_by(|(a, _), (b, _)| a.cmp(b));
            for (key, value) in sorted_properties {
                if !props.contains_key(key) {
                    props.insert(key.clone(), value.clone());
                }
            }
        }
        map.serialize_entry("properties", &props)?;

        map.end()
    }
}

// impl Serialize for Overlay {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         use serde::ser::SerializeMap;
//
//         let mut map = serializer.serialize_map(None)?;
//
//         // Serialize attributes in the specified order
//         map.serialize_entry("digest", &self.digest)?;
//         map.serialize_entry("capture_base", &self.capture_base)?;
//         map.serialize_entry("type", &self.name)?;
//
//             // If registry is set, use it to serialize the overlay elements
//             for element in self.overlay_def.as_ref().unwrap().elements.iter() {
//                 if let Some(value) = self.properties.as_ref().and_then(|props| props.get(&element.name)) {
//                     map.serialize_entry(&element.name, value)?;
//                 }
//             }
//
//         // Serialize remaining properties in lexicographical order
//         if let Some(properties) = &self.properties {
//             let mut sorted_properties: Vec<_> = properties.iter().collect();
//             sorted_properties.sort_by(|(a, _), (b, _)| a.cmp(b));
//             for (key, value) in sorted_properties {
//                 map.serialize_entry(key, value)?;
//             }
//         }
//
//         map.end()
//     }
// }

impl From<&OverlayModel> for Overlay {
    fn from(model: &OverlayModel) -> Self {
        Self { model: model.clone() }
    }
}

impl Overlay {
    pub fn new(overlay_model: OverlayModel) -> Self {
        Self {
            model: overlay_model
        }
    }

    pub fn to_json(&self) -> Result<String, OverlaySerializationError> {
        let code = HashFunctionCode::Blake3_256;
        if self.model.capture_base_said.is_none() {
            return Err(OverlaySerializationError::MissingCaptureBaseSaid(self.model.name.clone()));
        }
        let serialized_overlay = serde_json::to_string(&self).map_err(|_| OverlaySerializationError::MissingOverlayDef(self.model.name.clone()))?;
        let said_field = Some("digest");
        let input = serialized_overlay.as_str();
        match make_me_happy(input, code, said_field) {
            Ok(sad) => {
                let json: serde_json::Value = serde_json::from_str(&sad).map_err(|_| OverlaySerializationError::MissingOverlayDef(self.model.name.clone()))?;
                info!("Overlay {} serialized successfully with digest: {}", self.model.name, json.get("digest").unwrap());
                Ok(sad)
            },
            Err(_) => Err(OverlaySerializationError::MissingOverlayDef(self.model.name.clone())),
        }
    }

}

impl OverlayModel {
    pub fn new(content: OverlayContent) -> Self {
        Self {
            digest: None,
            name: content.overlay_name,
            unique_keys: None,
            capture_base_said: None,
            properties: content.properties,
            overlay_def: None,
        }
    }

    pub fn fill_digest(&mut self) -> Result<(), OverlaySerializationError> {
        if self.digest.is_none() {
            let digest = self.compute_digest()?;
            self.digest = Some(digest);
        }
        Ok(())
    }

    fn compute_digest(&self) -> Result<said::SelfAddressingIdentifier, OverlaySerializationError> {
        let code = HashFunctionCode::Blake3_256;
        if self.capture_base_said.is_none() {
            return Err(OverlaySerializationError::MissingCaptureBaseSaid(self.name.clone()));
        }
        let overlay = Overlay::from(self);
        let serialized_overlay = serde_json::to_string(&overlay).map_err(|_| OverlaySerializationError::MissingOverlayDef(self.name.clone()))?;
        let said_field = Some("digest");
        let input = serialized_overlay.as_str();
        let str = make_me_happy(input, code, said_field).map_err(|_| OverlaySerializationError::MissingOverlayDef(self.name.clone()));
        let json: serde_json::Value = serde_json::from_str(&str.unwrap()).map_err(|_| OverlaySerializationError::MissingOverlayDef(self.name.clone()))?;
        let said: SelfAddressingIdentifier = json.get("digest").unwrap().as_str().unwrap().parse().unwrap();
        Ok(said)
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

    #[error("Failed to serialize overlay capture based said missing: {0}")]
    MissingCaptureBaseSaid(String),
}
