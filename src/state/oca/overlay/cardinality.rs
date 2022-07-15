use crate::state::{attribute::Attribute, oca::Overlay};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CardinalityOverlay {
    capture_base: String,
    #[serde(rename = "type")]
    overlay_type: String,
    attr_cardinality: BTreeMap<String, String>,
}

impl Overlay for CardinalityOverlay {
    fn capture_base(&mut self) -> &mut String {
        &mut self.capture_base
    }
    fn overlay_type(&self) -> &String {
        &self.overlay_type
    }
    fn attributes(&self) -> Vec<&String> {
        self.attr_cardinality.keys().collect::<Vec<&String>>()
    }

    fn add(&mut self, attribute: &Attribute) {
        if attribute.cardinality.is_some() {
            self.attr_cardinality.insert(
                attribute.name.clone(),
                attribute.cardinality.as_ref().unwrap().clone(),
            );
        }
    }
}
impl CardinalityOverlay {
    pub fn new() -> Box<CardinalityOverlay> {
        Box::new(CardinalityOverlay {
            capture_base: String::new(),
            overlay_type: "spec/overlays/cardinality/1.0".to_string(),
            attr_cardinality: BTreeMap::new(),
        })
    }
}