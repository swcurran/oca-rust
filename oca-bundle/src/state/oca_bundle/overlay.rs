use indexmap::IndexMap;
use log::debug;
use oca_ast::ast::{NestedValue, OverlayContent};
use overlay_file::OverlayDef;
use said::make_me_happy;
use said::{SelfAddressingIdentifier, derivation::HashFunctionCode};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;
use thiserror::Error;

pub type OverlayName = String;

/// Overlay struct is used only for serialization purposes.
/// Internally OverlayModel should be used.
#[derive(Debug, Clone)]
pub struct Overlay {
    pub model: OverlayModel,
}

#[derive(Deserialize, Debug, Clone)]
pub struct OverlayModel {
    pub digest: Option<said::SelfAddressingIdentifier>,
    #[serde(rename = "capture_base")]
    pub capture_base_said: Option<said::SelfAddressingIdentifier>,
    #[serde(rename = "type")]
    pub name: OverlayName,
    pub properties: Option<IndexMap<String, NestedValue>>,
    // if we deserialize from OCA BUNDLE json we do not have overlay_def it would be loaded during validation of the obejct
    pub overlay_def: Option<OverlayDef>,
}

impl<'de> Deserialize<'de> for Overlay {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct OverlayWire {
            #[serde(default)]
            digest: Option<said::SelfAddressingIdentifier>,

            #[serde(rename = "capture_base", default)]
            capture_base_said: Option<said::SelfAddressingIdentifier>,

            // Serialized as a string "overlay/{full_name}"
            #[serde(rename = "type")]
            type_field: String,

            // Everything else goes into properties
            #[serde(flatten)]
            properties: IndexMap<String, NestedValue>,
        }

        let wire = OverlayWire::deserialize(deserializer)?;

        let name = wire.type_field;

        let model = OverlayModel {
            digest: wire.digest,
            capture_base_said: wire.capture_base_said,
            name,
            properties: if wire.properties.is_empty() {
                None
            } else {
                Some(wire.properties)
            },
            // Also not present in JSON; filled in later by your logic
            overlay_def: None,
        };

        Ok(Overlay { model })
    }
}

impl Serialize for Overlay {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap;
        debug!("Serializing overlay for: {}", self.model.name);

        let mut map = serializer.serialize_map(None)?;

        map.serialize_entry("digest", &self.model.digest)?;
        map.serialize_entry("capture_base", &self.model.capture_base_said)?;

        let overlay_def = self
            .model
            .overlay_def
            .as_ref()
            .ok_or_else(|| serde::ser::Error::custom("Missing overlay definition"))?;

        map.serialize_entry("type", &format!("overlay/{}", overlay_def.get_full_name()))?;

        // Create a set to keep track of serialized keys
        let mut serialized_keys = std::collections::HashSet::new();

        // Serialize attributes in the order defined in the overlay definition
        for element in overlay_def.elements.iter() {
            if let Some(value) = self
                .model
                .properties
                .as_ref()
                .and_then(|props| props.get(&element.name))
            {
                map.serialize_entry(&element.name, value)?;
                serialized_keys.insert(&element.name);
            }
        }

        // Serialize remaining properties in lexicographical order
        if let Some(properties) = &self.model.properties {
            let mut sorted_properties: Vec<_> = properties.iter().collect();
            sorted_properties.sort_by(|(a, _), (b, _)| a.cmp(b));
            for (key, value) in sorted_properties {
                if !serialized_keys.contains(key) {
                    map.serialize_entry(key, value)?;
                }
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
        map.serialize_entry("overlay_def", &self.overlay_def)?;
        let mut props = BTreeMap::new();
        // Use overlay definition to serialize elements in the correct order
        if let Some(ref overlay_def) = self.overlay_def {
            for element in overlay_def.elements.iter() {
                if let Some(value) = self
                    .properties
                    .as_ref()
                    .and_then(|props| props.get(&element.name))
                {
                    props.insert(element.name.clone(), value.clone());
                }
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

impl From<&OverlayModel> for Overlay {
    fn from(model: &OverlayModel) -> Self {
        Self {
            model: model.clone(),
        }
    }
}

impl Overlay {
    pub fn new(overlay_model: OverlayModel) -> Self {
        Self {
            model: overlay_model,
        }
    }

    pub fn overlay_type(&self) -> String {
        self.model.name.clone()
    }

    pub fn model(&self) -> &OverlayModel {
        &self.model
    }

    pub fn model_mut(&mut self) -> &mut OverlayModel {
        &mut self.model
    }

    /// Set the overlay definition for this overlay.
    /// This should be called after deserialization when definitions become available.
    pub fn set_overlay_def(&mut self, overlay_def: OverlayDef) {
        self.model.overlay_def = Some(overlay_def);
    }

    /// Check if this overlay has an overlay definition.
    pub fn has_overlay_def(&self) -> bool {
        self.model.overlay_def.is_some()
    }
}

impl OverlayModel {
    pub fn new(content: OverlayContent) -> Self {
        Self {
            digest: None,
            name: content.overlay_def.get_name().to_string(),
            capture_base_said: None,
            properties: content.properties,
            overlay_def: Some(content.overlay_def),
        }
    }

    pub fn fill_digest(&mut self) -> Result<(), OverlaySerializationError> {
        let digest = self.compute_digest()?;
        self.digest = Some(digest);
        Ok(())
    }

    fn compute_digest(&self) -> Result<said::SelfAddressingIdentifier, OverlaySerializationError> {
        let code = HashFunctionCode::Blake3_256;
        if self.capture_base_said.is_none() {
            return Err(OverlaySerializationError::MissingCaptureBaseSaid(
                self.name.clone(),
            ));
        }
        let overlay = Overlay::from(self);
        let serialized_overlay = serde_json::to_string(&overlay)
            .map_err(|_| OverlaySerializationError::MissingOverlayDef(self.name.clone()))?;
        let said_field = Some("digest");
        let input = serialized_overlay.as_str();
        let str = make_me_happy(input, code, said_field)
            .map_err(|_| OverlaySerializationError::MissingOverlayDef(self.name.clone()));
        let json: serde_json::Value = serde_json::from_str(&str.unwrap())
            .map_err(|_| OverlaySerializationError::MissingOverlayDef(self.name.clone()))?;
        let said: SelfAddressingIdentifier = json
            .get("digest")
            .ok_or_else(|| OverlaySerializationError::SaidComputationFailed(self.name.clone()))?
            .as_str()
            .ok_or_else(|| OverlaySerializationError::SaidComputationFailed(self.name.clone()))?
            .parse()
            .map_err(|_| OverlaySerializationError::SaidComputationFailed(self.name.clone()))?;
        Ok(said)
    }
}

// TODO not used? it should be used in all nested values for serialization, check it.
// fn sort_nested_value(value: &mut NestedValue) {
//     match value {
//         NestedValue::Object(map) => {
//             for (_, v) in map.iter_mut() {
//                 sort_nested_value(v);
//             }
//             map.sort_keys();
//         }
//         NestedValue::Array(arr) => {
//             for v in arr.iter_mut() {
//                 sort_nested_value(v);
//             }
//             arr.sort_by(|a, b| match (a, b) {
//                 (NestedValue::Value(a), NestedValue::Value(b)) => a.cmp(b),
//                 _ => Ordering::Equal,
//             });
//         }
//         _ => {}
//     }
// }
//
#[derive(Error, Debug)]
pub enum OverlaySerializationError {
    #[error("No overlay definition found for overlay type: {0}")]
    MissingOverlayDef(String),

    #[error("Failed to serialize overlay capture based said missing: {0}")]
    MissingCaptureBaseSaid(String),

    #[error("Failed to serialize overlay: {0}")]
    SerializationFailed(String),

    #[error("Failed to compute SAID for overlay: {0}")]
    SaidComputationFailed(String),
}

// #[derive(Error, Debug)]
// pub enum OverlayValidationError {
//     #[error("No overlay definition found for overlay type: {0}")]
//     MissingOverlayDef(String),
//
//     #[error("Missing required field '{field_name}' in overlay type '{overlay_type}'")]
//     MissingRequiredField {
//         overlay_type: String,
//         field_name: String,
//     },
// }

#[cfg(test)]
mod tests {
    use super::*;
    use overlay_file::{ElementType, KeyType, OverlayElementDef};
    use serde_json;

    #[test]
    fn overlay_deserialize_collects_properties_and_sets_defaults() {
        let json = r#"{
    "digest": "EF-fG_9Wy3dVaBVq3wHe-VZnWtNVJeM3MMt7IOqrvuSt",
    "capture_base": "EK-iSsbRjw5CvsGDK9nnCZ2JNVsa8cdQ_VwUgmpsVo_6",
    "type": "overlay/label/2.0.0",
    "language": "en",
    "attribute_labels": {
        "dateOfBirth": "Date of birth",
        "documentNumber": "Passport Number",
        "documentType": "Document",
        "fullName": "Name",
        "height": "Height",
        "issuingState": "Issuing State or organization (in full)",
        "photoImage": "Portrait image",
        "sex": "Sex"
    }
} "#;

        let mut overlay: Overlay =
            serde_json::from_str(json).expect("failed to deserialize Overlay");

        let label_overlay_def = OverlayDef {
            name: "label".to_string(),
            elements: vec![
                OverlayElementDef {
                    name: "attr_labels".to_string(),
                    values: ElementType::Text,
                    keys: KeyType::AttrNames,
                },
                OverlayElementDef {
                    name: "language".to_string(),
                    values: ElementType::Lang,
                    keys: KeyType::None,
                },
            ],
            namespace: None,
            version: "2.0.0".to_string(),
            unique_keys: Vec::new(),
        };
        overlay.set_overlay_def(label_overlay_def);
        // type → name
        assert_eq!(overlay.model.name, "overlay/label/2.0.0");
        assert!(overlay.model.digest.is_some());

        // calculate digest once more to verify integrity
        match overlay.model.fill_digest() {
            Ok(_) => {
                assert_eq!(
                    overlay.model.capture_base_said,
                    Some(
                        "EK-iSsbRjw5CvsGDK9nnCZ2JNVsa8cdQ_VwUgmpsVo_6"
                            .parse()
                            .unwrap()
                    )
                );
            }
            Err(_) => {
                panic!("failed to fill digest");
            }
        }

        // Properties must contain the extra keys
        let props = overlay
            .model
            .properties
            .as_ref()
            .expect("properties should be Some");

        assert_eq!(props.len(), 2);
        assert!(props.contains_key("language"));
        assert!(props.contains_key("attribute_labels"));
    }
}
