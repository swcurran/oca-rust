use crate::state::attribute::Attribute;
use indexmap::IndexMap;
use oca_ast::ast::NestedAttrType;
use said::{SelfAddressingIdentifier, derivation::HashFunctionCode, make_me_happy};
use serde::{Deserialize, Serialize, Serializer, ser::SerializeMap};
use thiserror::Error;

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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CaptureBase {
    pub digest: Option<said::SelfAddressingIdentifier>,
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
            digest: None,
            attributes: IndexMap::new(),
        }
    }

    pub fn add(&mut self, attribute: &Attribute) {
        self.attributes.insert(
            attribute.name.clone(),
            attribute.attribute_type.clone().unwrap(),
        );
    }

    fn compute_digest(&self) -> Result<SelfAddressingIdentifier, CaptureBaseSerializationError> {
        let code = HashFunctionCode::Blake3_256;
        let serialized_overlay = serde_json::to_string(&self)
            .map_err(|_| CaptureBaseSerializationError::SerializationError())?;
        let said_field = Some("digest");
        let input = serialized_overlay.as_str();
        let serialized = make_me_happy(input, code, said_field)
            .map_err(|_| CaptureBaseSerializationError::DeriveSaidError());
        let json: serde_json::Value = serde_json::from_str(&serialized.unwrap())
            .map_err(|_| CaptureBaseSerializationError::SerializationError())?;
        let said: SelfAddressingIdentifier = json
            .get("digest")
            .unwrap()
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        Ok(said)
    }

    pub fn fill_digest(&mut self) -> Result<(), CaptureBaseSerializationError> {
        let said = self.compute_digest()?;
        self.digest = Some(said);
        Ok(())
    }
}

#[derive(Error, Debug)]
pub enum CaptureBaseSerializationError {
    #[error("Failed to serialize capture based")]
    SerializationError(),

    #[error("Failed to derive said for capture base")]
    DeriveSaidError(),
}
