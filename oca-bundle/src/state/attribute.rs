// use super::standard::Standard;
use isolang::Language;
pub use oca_ast::ast::AttributeType;
use oca_ast::ast::{NestedAttrType, NestedValue};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type OverlayName = String;
// use crate::state::{encoding::Encoding, entries::EntriesElement, entry_codes::EntryCodes};
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Attribute {
    pub name: String,
    #[serde(rename = "type")]
    pub attribute_type: Option<NestedAttrType>,
    pub properties: Option<HashMap<OverlayName, NestedValue>>,
}

impl Default for Attribute {
    fn default() -> Self {
        Self::new("".to_string())
    }
}

impl Attribute {
    pub fn new(name: String) -> Attribute {
        Attribute {
            name,
            attribute_type: None,
            properties: None,
        }
    }

    pub fn set_attribute_type(&mut self, attribute_type: NestedAttrType) {
        self.attribute_type = Some(attribute_type);
    }

    // Merge assumption is that if `other` is not None then it would overwrite `self` or would be concatenated with `self`
    pub fn merge(&mut self, other: &Attribute) {
        if self.name != other.name {
            panic!("Cannot merge attributes with different names");
        } else if other.attribute_type.is_some() {
            self.attribute_type.clone_from(&other.attribute_type);
        }
    }

    // fn merge_entries(&mut self, other: &Attribute) {
    //     if self.entries.is_none() {
    //         self.entries.clone_from(&other.entries);
    //     } else if let Some(entries) = &other.entries {
    //         for (lang, entry) in entries {
    //             self.entries.as_mut().unwrap().insert(*lang, entry.clone());
    //         }
    //     }
    // }
    //
    // fn merge_labels(&mut self, other: &Attribute) {
    //     if self.labels.is_none() {
    //         self.labels.clone_from(&other.labels)
    //     } else if let Some(labels) = &other.labels {
    //         for (lang, label) in labels {
    //             self.labels.as_mut().unwrap().insert(*lang, label.clone());
    //         }
    //     }
    // }

    // pub fn add_mapping(mut self, mapping: String) -> AttributeBuilder {
    //     self.attribute.mapping = Some(mapping);
    //     self
    // }

    // pub fn add_entry_codes_mapping(mut self, mapping: Vec<String>) -> AttributeBuilder {
    //     self.attribute.entry_codes_mapping = Some(mapping);
    //     self
    // }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Entry {
    pub id: String,
    pub translations: HashMap<Language, String>,
}

impl Entry {
    pub fn new(id: String, translations: HashMap<Language, String>) -> Entry {
        Entry { id, translations }
    }
}

/*
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Entries {
    Sai(HashMap<Language, String>),
    Object(Vec<Entry>),
}
*/
