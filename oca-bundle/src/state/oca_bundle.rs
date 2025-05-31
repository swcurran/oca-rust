use indexmap::IndexMap;
use log::info;
use overlay::{Overlay, OverlayModel};
use overlay_file::overlay_registry::OverlayLocalRegistry;
use said::derivation::HashFunctionCode;
use said::{make_me_sad, ProtocolVersion, SelfAddressingIdentifier};
use serde::ser::Error;
use serde::{Deserialize, Serialize, Serializer};
use serde_json::Value;
use thiserror::Error;
use std::collections::HashMap;
pub mod capture_base;
pub mod overlay;
use crate::state::{
    attribute::Attribute,
    oca_bundle::capture_base::CaptureBase,
};
use oca_ast::ast::{CaptureContent, Command, CommandType, OCAAst, ObjectKind};

#[derive(Debug, Deserialize, Clone)]
// #[version(protocol = "OCAS", major = 2, minor = 0)]
pub struct OCABundle {
    pub model: OCABundleModel,
}

#[derive(Serialize, Debug, Deserialize, Clone)]
pub struct OCABundleModel {
    pub digest: Option<said::SelfAddressingIdentifier>,
    pub capture_base: CaptureBase,
    pub overlays: Vec<OverlayModel>,
    // Storing attributes in different model for easy read access
    pub attributes: HashMap<String, Attribute>,
}

impl Default for OCABundleModel {
    fn default() -> Self {
        OCABundleModel {
            digest: None,
            capture_base: CaptureBase::default(),
            overlays: Vec::new(),
            attributes: HashMap::new(),
        }
    }
}

impl From<OCABundleModel> for OCABundle {
    fn from(model: OCABundleModel) -> Self {
        OCABundle { model }
    }
}

impl OCABundle {
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        let code = HashFunctionCode::Blake3_256;
        let serialized_bundle = serde_json::to_string(&self).map_err(|_| {
            serde_json::Error::custom("Failed to serialize OCABundleModel")
        })?;
        let said_field = Some("digest");
        let version = ProtocolVersion::new("OCAS", 2, 0).unwrap();
        let input = serialized_bundle.as_str();
        match make_me_sad(input, code, version, said_field) {
            Ok(sad) => {
                let json: Value = serde_json::from_str(&serialized_bundle)
                    .map_err(|_| serde_json::Error::custom("Failed to parse OCABundle JSON"))?;
                info!("OCABundle serialized successfully with digest: {}", json.get("digest").unwrap()); Ok(sad)
            },
            Err(_) => Err(serde_json::Error::custom("Failed to compute digest for oca bundle")),
        }
    }

}

impl Serialize for OCABundle {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("digest", &self.model.digest)?;
        let capture_base_json = self.model.capture_base.to_json().map_err(serde::ser::Error::custom)?;
        let capture_base: Value = serde_json::from_str(&capture_base_json)
            .map_err(serde::ser::Error::custom)?;
        map.serialize_entry("capture_base", &capture_base)?;
        let overlays: Vec<Overlay> = self.model.overlays.iter().map(Overlay::from).collect();
        let overlays_json: Vec<Value> = overlays
            .iter()
            .map(|overlay| {
                let overlay_json = overlay.to_json().map_err(serde::ser::Error::custom)?;
                serde_json::from_str(&overlay_json).map_err(serde::ser::Error::custom)
            })
            .collect::<Result<_, _>>()?;
        map.serialize_entry( "overlays", &overlays_json)?;
        map.end()
    }
}

impl OCABundleModel {
    pub fn new(
        capture_base: CaptureBase,
        overlays: Vec<OverlayModel>,
    ) -> Self {
        OCABundleModel {
            digest: None,
            capture_base,
            overlays,
            attributes: HashMap::new(),
        }
    }
    // pub fn fill_said(&mut self) {
    //     let code = HashFunctionCode::Blake3_256;
    //     let format = SerializationFormats::JSON;
    //     self.compute_digest(&code, &format);
    // }

    pub fn to_ast(&self, _registry: OverlayLocalRegistry) -> OCAAst {
        let mut ast = OCAAst::new();

        let properties = None;

        let mut attributes = IndexMap::new();
        self.capture_base
            .attributes
            .iter()
            .for_each(|(attr_name, attr_type)| {
                attributes.insert(attr_name.clone(), attr_type.clone());
            });

        let command = Command {
            kind: CommandType::Add,
            object_kind: ObjectKind::CaptureBase(CaptureContent {
                // TODO find out if we can use indexmap in capture base to simplify stuff
                attributes: Some(self.capture_base.attributes.clone().into_iter().collect()),
                properties,
            }),
            overlay_def: None,
        };

        // TODO ADD Overlays
        ast.commands.push(command);
        ast
    }

    // Prepare list of attributes to fill with overlays properties
    fn fill_attributes(&mut self) {
        for (attr_name, attr_type) in self.capture_base.attributes.clone() {
            let attr = Attribute {
                name: attr_name.clone(),
                attribute_type: Some(attr_type),
                ..Default::default()
            };
            self.attributes.insert(attr_name.clone(), attr);
        }
    }

    /// Remove attribute from the OCA Bundle
    /// if attribute does not exist, nothing will happen
    pub fn remove_attribute(&mut self, attr_name: &String) {
        self.attributes.remove(attr_name);
    }
    /// Add an attribute to the OCA Bundle
    /// If the attribute already exists, it will be merged with the new attribute
    /// for simple types: the new value will overwrite the old value
    /// for complex types: the new value will be added to the old value
    pub fn add_attribute(&mut self, attribute: Attribute) {
        if let Some(attr) = self.get_attribute_mut(&attribute.name) {
            attr.merge(&attribute);
        } else {
            self.attributes.insert(attribute.name.clone(), attribute);
        }
    }
    pub fn get_attribute_by_name(&self, name: &str) -> Option<&Attribute> {
        self.attributes.get(name)
    }

    fn get_attribute_mut(&mut self, name: &str) -> Option<&mut Attribute> {
        self.attributes.get_mut(name)
    }

    pub fn fill_digest(&mut self) {
        let said = self.compute_digest();
        match said {
            Ok(said) => {
                let digest = said.clone();
                self.digest = Some(digest);
                info!("OCABundle SAID computed: {:?}", self.digest);
            }
            Err(e) => {
                info!("Failed to compute OCABundle SAID: {}", e);
            }
        }


    }

    fn compute_digest(&mut self) -> Result<said::SelfAddressingIdentifier, OCABundleSerializationError> {
        self.capture_base.fill_digest();
        let cb_said = self.capture_base.digest.clone();
        self.overlays.iter_mut().for_each(|overlay| {
            overlay.capture_base_said = cb_said.clone();
            overlay.fill_digest();
        });

        let oca_bundle = OCABundle::from(self.clone());
        let json = oca_bundle.to_json().unwrap();
        let said: SelfAddressingIdentifier = serde_json::from_str(&json)
            .map_err(|_| OCABundleSerializationError::SerializationError("Failed to parse OCABundle JSON".to_string()))
            .and_then(|v: Value| {
                v.get("digest")
                    .and_then(|d| d.as_str())
                    .ok_or_else(|| OCABundleSerializationError::SerializationError("Missing digest in OCABundle JSON".to_string()))
                    .and_then(|s| s.parse().map_err(|_| OCABundleSerializationError::SerializationError("Failed to parse SAID".to_string())))
            }).unwrap();
        Ok(said)
    }
}
#[derive(Error, Debug)]
pub enum OCABundleSerializationError {
    #[error("Failed to serialize OCA bundle")]
    SerializationError(String),

}

#[cfg(test)]
mod tests {
    use oca_file::ocafile::parse_from_string;
    use serde_value::Value;
    use crate::build::from_ast;
    use super::*;

    #[test]
    fn build_oca_bundle() {
        let _ = env_logger::builder().is_test(true).try_init();
        let unparsed_file = r#"
-- version=2.0.0
-- name=プラスウルトラ
ADD ATTRIBUTE remove=Text
ADD ATTRIBUTE name=Text age=Numeric car=[refs:EJeWVGxkqxWrdGi0efOzwg1YQK8FrA-ZmtegiVEtAVcu]
REMOVE ATTRIBUTE remove
ADD ATTRIBUTE incidentals_spare_parts=[[refs:EJeWVGxkqxWrdGi0efOzwg1YQK8FrA-ZmtegiVEtAVcu]]
ADD ATTRIBUTE d=Text i=Text passed=Boolean
ADD Overlay META
  language="en"
  description="Entrance credential"
  name="Entrance credential"
ADD Overlay CHARACTER_ENCODING
  attribute_character_encoding
    d="utf-8"
    i="utf-8"
    passed="utf-8"
ADD Overlay CONFORMANCE
  attribute_conformance
    d="M"
    i="M"
    passed="M"
ADD Overlay LABEL
  attr_labels
    language="en"
    d="Schema digest"
    i="Credential Issuee"
    passed="Passed"
ADD Overlay FORMAT
  attribute_formats
    d="image/jpeg"
ADD Overlay UNIT
  attribute_units
    i="m^2"
    d="°"
ADD ATTRIBUTE list=[Text] el=Text
ADD Overlay CARDINALITY
  attr_cardinality
    list="1-2"
ADD Overlay ENTRY_CODE
  attribute_entry_codes
    list="entry_code_said"
    el=["o1", "o2", "o3"]
ADD Overlay ENTRY
  attribute_entrires
    language="en"
    list="entry_said"
    el
     o1="o1_label"
     o2="o2_label"
     o3="o3_label"
"#;
        let registry = OverlayLocalRegistry::from_dir("../overlay-file/core_overlays/").unwrap();
        let oca_ast = parse_from_string(unparsed_file.to_string(), &registry).unwrap();

        let bundle_json = r#"
        {
  "v": "OCAS02JSON000940_",
  "digest": "EEObcwCDURXRbDQcPxXm-UFD6cauIlarxWBxP8cadmV0",
  "capture_base": {
    "digest": "ECUpSbGNlOKbqOqqW9640x-ev-flOKZ6Q-_h97DKehdY",
    "type": "capture_base/2.0.0",
    "attributes": {
      "age": "Numeric",
      "car": [
        "refs:EJeWVGxkqxWrdGi0efOzwg1YQK8FrA-ZmtegiVEtAVcu"
      ],
      "d": "Text",
      "el": "Text",
      "i": "Text",
      "incidentals_spare_parts": [
        [
          "refs:EJeWVGxkqxWrdGi0efOzwg1YQK8FrA-ZmtegiVEtAVcu"
        ]
      ],
      "list": [
        "Text"
      ],
      "name": "Text",
      "passed": "Boolean",
      "remove": "Text"
    }
  },
  "overlays": [
    {
      "digest": "EJJapYdgZ61TX5V7sbcXP_7ob5UCiRdWKeK5pmqUqByW",
      "capture_base": "ECUpSbGNlOKbqOqqW9640x-ev-flOKZ6Q-_h97DKehdY",
      "type": "Meta/2.0.0",
      "language": "en",
      "description": "Entrance credential",
      "language": "en",
      "name": "Entrance credential"
    },
    {
      "digest": "EFU0g81U1SDrAuJG2pbgmPq22KaSHwEhPNLu5_tRS7Rp",
      "capture_base": "ECUpSbGNlOKbqOqqW9640x-ev-flOKZ6Q-_h97DKehdY",
      "type": "Character_Encoding/2.0.0",
      "attribute_character_encoding": {
        "d": "utf-8",
        "i": "utf-8",
        "passed": "utf-8"
      },
      "attribute_character_encoding": {
        "d": "utf-8",
        "i": "utf-8",
        "passed": "utf-8"
      }
    },
    {
      "digest": "EC_yBQRqY4NNdgLi2yAmZ0KYruG4mB9Bdk-LcfZD1V6k",
      "capture_base": "ECUpSbGNlOKbqOqqW9640x-ev-flOKZ6Q-_h97DKehdY",
      "type": "conformance/2.0.0",
      "attribute_conformance": {
        "d": "M",
        "i": "M",
        "passed": "M"
      },
      "attribute_conformance": {
        "d": "M",
        "i": "M",
        "passed": "M"
      }
    },
    {
      "digest": "EMw3ldrzC9jlIzHVtBKME7wq2BSJXOQXA6avmWwqFBfu",
      "capture_base": "ECUpSbGNlOKbqOqqW9640x-ev-flOKZ6Q-_h97DKehdY",
      "type": "label/2.0.0",
      "attr_labels": {
        "language": "en",
        "d": "Schema digest",
        "i": "Credential Issuee",
        "passed": "Passed"
      },
      "attr_labels": {
        "language": "en",
        "d": "Schema digest",
        "i": "Credential Issuee",
        "passed": "Passed"
      }
    },
    {
      "digest": "EBvwOhLmNkRbrSDlFRFFUZMX__OuCTbJNLybliFEd3GQ",
      "capture_base": "ECUpSbGNlOKbqOqqW9640x-ev-flOKZ6Q-_h97DKehdY",
      "type": "format/2.0.0",
      "attribute_formats": {
        "d": "image/jpeg"
      },
      "attribute_formats": {
        "d": "image/jpeg"
      }
    },
    {
      "digest": "EGldBR6aO9x58qBzmmkMHL6Kfbaa2MKV_JzUP8LaTc-7",
      "capture_base": "ECUpSbGNlOKbqOqqW9640x-ev-flOKZ6Q-_h97DKehdY",
      "type": "unit/2.0.0",
      "attribute_units": {
        "i": "m^2",
        "d": "°"
      },
      "attribute_units": {
        "i": "m^2",
        "d": "°"
      }
    },
    {
      "digest": "EBOhsb4M15hQJRGLuUJJak4xHJ1AKwwaIs3uF09ePMAF",
      "capture_base": "ECUpSbGNlOKbqOqqW9640x-ev-flOKZ6Q-_h97DKehdY",
      "type": "cardinality/2.0.0",
      "attr_cardinality": {
        "list": "1-2"
      },
      "attr_cardinality": {
        "list": "1-2"
      }
    },
    {
      "digest": "ECmMD5tfJxOBxMzO8zIq9mvxpUjMxnI97sBcenCeu8zR",
      "capture_base": "ECUpSbGNlOKbqOqqW9640x-ev-flOKZ6Q-_h97DKehdY",
      "type": "ENTRY_CODE/2.0.0",
      "attribute_entry_codes": {
        "list": "entry_code_said",
        "el": [
          "o1",
          "o2",
          "o3"
        ]
      },
      "attribute_entry_codes": {
        "list": "entry_code_said",
        "el": [
          "o1",
          "o2",
          "o3"
        ]
      }
    },
    {
      "digest": "EBFvEURPLR3FrcXhsksdzynGKxm1wnenvGM208FtE6h0",
      "capture_base": "ECUpSbGNlOKbqOqqW9640x-ev-flOKZ6Q-_h97DKehdY",
      "type": "ENTRY/2.0.0",
      "attribute_entrires": {
        "language": "en",
        "list": "entry_said",
        "el": {
          "o1": "o1_label",
          "o2": "o2_label",
          "o3": "o3_label"
        }
      }
    }
  ]
}
"#;
        let reference_json: serde_json::Value = serde_json::from_str(bundle_json).unwrap();
        let mut oca_bundle = from_ast(None, &oca_ast).unwrap().oca_bundle;
        let mut oca_bundle2 = from_ast(None, &oca_ast).unwrap().oca_bundle;
        oca_bundle.compute_digest();
        oca_bundle2.compute_digest();
        let overlay_model = oca_bundle.overlays.first().unwrap();
        let overlay = Overlay::from(&overlay_model.clone());
        let meta_overlay: serde_json::Value = serde_json::from_str(&overlay.to_json().unwrap()).unwrap();
        let meta_said = "EJJapYdgZ61TX5V7sbcXP_7ob5UCiRdWKeK5pmqUqByW";
        assert_eq!(meta_overlay.get("digest").unwrap().as_str().unwrap(), meta_said.to_string());

        let said = oca_bundle.clone().digest;
        let bundle = OCABundle::from(oca_bundle.clone());
        let oca_bundle_json = bundle.to_json().unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&oca_bundle_json).unwrap(),
            reference_json
        );
        let said2 = oca_bundle2.digest;
        // Check if process is deterministic and gives always same SAID
        assert_eq!(said, said2);
    }
}
