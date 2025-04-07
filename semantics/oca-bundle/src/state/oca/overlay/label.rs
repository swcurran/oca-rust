use crate::state::{attribute::Attribute, oca::Overlay};
use isolang::Language;
use oca_ast_semantics::ast::OverlayType;
use said::derivation::HashFunctionCode;
use said::{sad::SerializationFormats, sad::SAD};
use serde::{ser::SerializeMap, ser::SerializeSeq, Deserialize, Serialize, Serializer};
use std::any::Any;
use std::collections::HashMap;

pub trait Labels {
    fn set_label(&mut self, l: Language, label: String);
}

impl Labels for Attribute {
    fn set_label(&mut self, l: Language, label: String) {
        match self.labels {
            Some(ref mut labels) => {
                labels.insert(l, label);
            }
            None => {
                let mut labels = HashMap::new();
                labels.insert(l, label);
                self.labels = Some(labels);
            }
        }
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

pub fn serialize_categories<S>(attributes: &[String], s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut ser = s.serialize_seq(Some(attributes.len()))?;

    let mut sorted_flagged_attributes = attributes.to_owned();
    sorted_flagged_attributes.sort();
    for attr in sorted_flagged_attributes {
        ser.serialize_element(&attr)?;
    }
    ser.end()
}

#[derive(SAD, Serialize, Deserialize, Debug, Clone)]
pub struct LabelOverlay {
    #[said]
    #[serde(rename = "d")]
    said: Option<said::SelfAddressingIdentifier>,
    capture_base: Option<said::SelfAddressingIdentifier>,
    #[serde(rename = "type")]
    overlay_type: OverlayType,
    language: Language,
    #[serde(serialize_with = "serialize_labels")]
    pub attribute_labels: HashMap<String, String>,
}

impl Overlay for LabelOverlay {
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
    fn language(&self) -> Option<&Language> {
        Some(&self.language)
    }
    fn attributes(&self) -> Vec<&String> {
        self.attribute_labels.keys().collect::<Vec<&String>>()
    }
    /// Add an attribute to the Label Overlay
    fn add(&mut self, attribute: &Attribute) {
        if let Some(labels) = &attribute.labels {
            if let Some(value) = labels.get(&self.language) {
                self.attribute_labels
                    .insert(attribute.name.clone(), value.to_string());
            }
        }
    }
}

impl LabelOverlay {
    pub fn new(lang: Language) -> Self {
        let overlay_version = "2.0.0".to_string();
        Self {
            capture_base: None,
            said: None,
            overlay_type: OverlayType::Label(overlay_version),
            language: lang,
            attribute_labels: HashMap::new(),
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_label_overlay() {
        let mut overlay = LabelOverlay::new(Language::Eng);
        let attr = cascade! {
            Attribute::new("attr1".to_string());
            ..set_label(Language::Pol, "Etykieta".to_string());
            ..set_label(Language::Eng, "Label".to_string());
        };
        // even that attribute has 2 lagnuage only one attribute should be added to the overlay according to it's language
        overlay.add(&attr);

        let overlay_version = "2.0.0".to_string();
        assert_eq!(overlay.overlay_type, OverlayType::Label(overlay_version));
        assert_eq!(overlay.language, Language::Eng);
        assert_eq!(overlay.attribute_labels.len(), 1);
    }
}
