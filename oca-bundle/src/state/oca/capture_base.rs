use crate::state::attribute::Attribute;
use indexmap::IndexMap;
use oca_ast::ast::NestedAttrType;
use said::{
    derivation::HashFunctionCode,
    sad::{SerializationFormats, SAD},
};
use serde::{ser::SerializeMap, Deserialize, Serialize, Serializer};

pub fn serialize_attributes<S>(
    attributes: &IndexMap<String, NestedAttrType>,
    s: S,
) -> Result<S::Ok, S::Error>
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
pub struct CaptureBase {
    #[said]
    #[serde(rename = "digest")]
    pub said: Option<said::SelfAddressingIdentifier>,
    #[serde(rename = "type")]
    pub schema_type: String,
    #[serde(serialize_with = "serialize_attributes")]
    pub attributes: IndexMap<String, NestedAttrType>,
}

impl Default for CaptureBase {
    fn default() -> Self {
        Self::new()
    }
}

impl CaptureBase {
    pub fn new() -> CaptureBase {
        CaptureBase {
            schema_type: String::from("capture_base/2.0.0"),
            said: None,
            attributes: IndexMap::new(),
        }
    }

    pub fn add(&mut self, attribute: &Attribute) {
        self.attributes.insert(
            attribute.name.clone(),
            attribute.attribute_type.clone().unwrap(),
        );
    }

    pub fn fill_said(&mut self) {
        let code = HashFunctionCode::Blake3_256;
        let format = SerializationFormats::JSON;
        self.compute_digest(&code, &format);
    }

    pub fn calculate_said(&mut self) {
        self.fill_said();
    }
}
