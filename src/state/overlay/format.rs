use crate::state::Attribute;
use crate::state::Overlay;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FormatOverlay {
    capture_base: String,
    #[serde(rename = "type")]
    overlay_type: String,
    attr_formats: HashMap<String, String>,
}

impl Overlay for FormatOverlay {
    fn capture_base(&mut self) -> &mut String {
        &mut self.capture_base
    }
    fn overlay_type(&self) -> &String {
        &self.overlay_type
    }

    fn add(&mut self, attribute: &Attribute) {
        if attribute.format.is_some() {
            self.attr_formats.insert(
                attribute.name.clone(),
                attribute.format.as_ref().unwrap().clone(),
            );
        }
    }
}
impl FormatOverlay {
    pub fn new() -> Box<FormatOverlay> {
        Box::new(FormatOverlay {
            capture_base: String::new(),
            overlay_type: "spec/overalys/format/1.0".to_string(),
            attr_formats: HashMap::new(),
        })
    }
}
