use crate::state::attribute::Attribute;
use indexmap::IndexMap;
use log::info;
use oca_ast::ast::NestedAttrType;
use said::{
    derivation::HashFunctionCode, make_me_happy, SelfAddressingIdentifier
};
use thiserror::Error;
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
        let serialized_overlay = serde_json::to_string(&self).map_err(|_| CaptureBaseSerializationError::SerializationError())?;
        let said_field = Some("digest");
        let input = serialized_overlay.as_str();
        let serialized = make_me_happy(input, code, said_field).map_err(|_| CaptureBaseSerializationError::DeriveSaidError());
        let json: serde_json::Value = serde_json::from_str(&serialized.unwrap()).map_err(|_| CaptureBaseSerializationError::SerializationError())?;
        let said: SelfAddressingIdentifier = json.get("digest").unwrap().as_str().unwrap().parse().unwrap();
        Ok(said)
    }

    pub fn fill_digest(&mut self) -> Result<(), CaptureBaseSerializationError> {
        if self.digest.is_none() {
            let said = self.compute_digest()?;
            self.digest = Some(said);
        }
        Ok(())
    }

    /// Serializes the CaptureBase to JSON and computes its SAID.
    /// we do not relay on filled said since this process needs to be atomic
    pub fn to_json(&self) -> Result<String, CaptureBaseSerializationError> {
        let code = HashFunctionCode::Blake3_256;
        let serialized_overlay = serde_json::to_string(&self).map_err(|_| CaptureBaseSerializationError::SerializationError())?;
        let said_field = Some("digest");
        let input = serialized_overlay.as_str();
        match make_me_happy(input, code,said_field) {
            Ok(sad) => {
                let json: serde_json::Value = serde_json::from_str(&sad).unwrap();
                let str = json.get("digest").unwrap().as_str();
                let said: SelfAddressingIdentifier = str.unwrap().parse().unwrap();
                info!("Capture base serialized successfully with digest: {:?}", said);
                Ok(sad)
            },
            Err(_) => Err(CaptureBaseSerializationError::DeriveSaidError()),
        }
    }
}

#[derive(Error, Debug)]
pub enum CaptureBaseSerializationError {
    #[error("Failed to serialize capture based")]
    SerializationError(),

    #[error("Failed to derive said for capture base")]
    DeriveSaidError(),
}
