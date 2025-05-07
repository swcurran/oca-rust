use std::any::Any;
use std::collections::HashMap;

use crate::state::attribute::Attribute;
use oca_ast::ast::OverlayType;
use said::derivation::HashFunctionCode;
use said::{sad::SerializationFormats, sad::SAD};
use serde::{Deserialize, Serializer};
use serde::{ser::SerializeMap, Serialize};

use super::Overlay;

pub trait Sensitive {
    fn set_sensitive(&mut self);
}

impl Sensitive for Attribute {
    fn set_sensitive(&mut self) {
        todo!()
    }
}

pub fn serialize_labels<S>(attributes: &HashMap<String, String>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    use std::collections::BTreeMap;

    let mut ser = s.serialize_map(Some(attributes.len()))?;
    let sorted_attributes: BTreeMap<_, _> = attributes.iter().collect();
    for (k, v) in sorted_attributes {
        ser.serialize_entry(k, v)?;
    }
    ser.end()
}

#[derive(SAD, Serialize, Deserialize, Debug, Clone)]
pub struct SensitiveOverlay {
    #[said]
    #[serde(rename = "digest")]
    said: Option<said::SelfAddressingIdentifier>,
    capture_base: Option<said::SelfAddressingIdentifier>,
    #[serde(rename = "type")]
    overlay_type: OverlayType,
    pub attributes: Vec<String>,
}

impl Overlay for SensitiveOverlay {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn capture_base(&self) -> &Option<said::SelfAddressingIdentifier> {
        &self.capture_base
    }
    fn set_capture_base(&mut self, said: &said::SelfAddressingIdentifier) {
        self.capture_base = Some(said.clone());
    }
    fn overlay_type(&self) -> &OverlayType {
        &self.overlay_type
    }
    fn said(&self) -> &Option<said::SelfAddressingIdentifier> {
        &self.said
    }
    fn attributes(&self) -> Vec<&String> {
        self.attributes.iter().collect::<Vec<&String>>()
    }
    /// Add an attribute to the Sensitive Overlay
    fn add(&mut self, attribute: &Attribute) {
        if attribute.sensitive.is_none() {
            return;
        }
        if let Some(value) = &attribute.sensitive {
            if value == &false {
                return;
            }
        }
        // Check if the attribute is already in the overlay
        if self.attributes.contains(&attribute.name) {
            return;
        }
        // Add the attribute to the overlay
       self.attributes.push(attribute.name.clone());
    }
}

impl SensitiveOverlay {
    pub fn new() -> Self {
        Self {
            capture_base: None,
            said: None,
            overlay_type: OverlayType::Sensitive,
            attributes: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_sensitive_overlay() {
        let mut overlay = SensitiveOverlay::new();
        let mut attr = Attribute::new("first_name".to_string());
        attr.set_sensitive();
        let attr2 = Attribute::new("last_name".to_string());
        overlay.add(&attr);
        overlay.add(&attr2);

        assert_eq!(overlay.overlay_type, OverlayType::Label);
        assert_eq!(overlay.attributes.len(), 1);
    }
}
