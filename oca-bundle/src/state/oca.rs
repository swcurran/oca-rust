use indexmap::IndexMap;
use overlay_file::overlay_registry::OverlayLocalRegistry;
use said::derivation::HashFunctionCode;
use said::sad::{SerializationFormats, SAD};
use said::version::SerializationInfo;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
pub mod capture_base;
pub mod overlay;
use crate::state::{
    attribute::Attribute,
    oca::{capture_base::CaptureBase, overlay::Overlay},
};
use oca_ast::ast::{CaptureContent, Command, CommandType, OCAAst, ObjectKind};

/// OCA Context to provide access to overlay registry
#[derive(Debug, Clone)]
pub struct OCAContext {
    registry: Arc<OverlayLocalRegistry>,
}

impl OCAContext {
    pub fn new(registry: OverlayLocalRegistry) -> Self {
        OCAContext {
            registry: Arc::new(registry),
        }
    }
}

impl Default for OCAContext {
    fn default() -> Self {
        OCAContext {
            registry: Arc::new(OverlayLocalRegistry::new()),
        }
    }
}

#[derive(SAD, Serialize, Debug, Deserialize, Clone)]
#[version(protocol = "OCAS", major = 2, minor = 0)]
pub struct OCABundle {
    #[said]
    #[serde(rename = "digest")]
    pub said: Option<said::SelfAddressingIdentifier>,
    pub capture_base: CaptureBase,
    pub overlays: Vec<Overlay>,

    #[serde(skip)]
    pub context: Arc<OCAContext>,
    #[serde(skip)]
    // Storing attributes in different model for easy access
    pub attributes: HashMap<String, Attribute>,
}

impl Default for OCABundle {
    fn default() -> Self {
        OCABundle {
            said: None,
            capture_base: CaptureBase::new(),
            overlays: Vec::new(),
            context: Arc::new(OCAContext::default()),
            attributes: HashMap::new(),
        }
    }
}

impl OCABundle {
    pub fn new(
        capture_base: CaptureBase,
        overlays: Vec<Overlay>,
        context: Arc<OCAContext>,
    ) -> Self {
        OCABundle {
            said: None,
            capture_base,
            overlays,
            context,
            attributes: HashMap::new(),
        }
    }
    pub fn fill_said(&mut self) {
        let code = HashFunctionCode::Blake3_256;
        let format = SerializationFormats::JSON;
        self.compute_digest(&code, &format);
    }

    pub fn set_context(&mut self, context: Arc<OCAContext>) {
        self.context = context;
    }

    pub fn to_ast(&self, registry: OverlayLocalRegistry) -> OCAAst {
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

    pub fn compute(&mut self) {
        self.capture_base.calculate_said();
        let cb_said = self.capture_base.said.clone().unwrap();
        self.overlays.iter_mut().for_each(|overlay| {
            overlay.calculate_said(&cb_said);
        });
        self.fill_said();
    }
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

        println!("OCA AST: {:#?}", oca_ast);

        let bundle_json = r#"
        {
  "digest": "EHm2AW4F6kh3HVYDlq7X8h4zHMHy4UoGLfhFF0tA5BIH",
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
      "digest": "EL_0PrkT0C914YZ2wsrqultjUpUeSCwxjhM3kQdzCsHn",
      "capture_base": "ECUpSbGNlOKbqOqqW9640x-ev-flOKZ6Q-_h97DKehdY",
      "type": "Meta/2.0.0",
      "language": "en",
      "description": "Entrance credential",
      "language": "en",
      "name": "Entrance credential"
    },
    {
      "digest": "EDb0-3T9elDEOk5mrbI-Dd58dnQrWAyQxywlpYxRwrQo",
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
      "digest": "EHV4aoto43PHxIEu9xnyCmKN3shbiOAb970GQ61DWtP4",
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
      "digest": "EAc3v_t8LTZZfL5TAuC0DZNvtMCYCgFi0yFOfFtUvbXf",
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
      "digest": "EKX3BzX4RbqHqo38kjtFbM7oKnAtd6586JDpnozCHyfJ",
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
      "digest": "ENLaH1nmt6Kkq_q-qYY5FSoO3nKSgt351FtD_tWsR6VL",
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
      "digest": "EBkhQh3X5-b9DWArEDElzU9BdVZ4vqHxf7UpdkmFq-8o",
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
      "digest": "EN7SBx8PFcJsOr2TIkhnik_eObQ4I9US6h2apn2v9SDO",
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
      "digest": "EBaxpD_M1vs25VRgs8CokqfpLd_o_uLtilniBTrxkc_2",
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
        let reference_json: Value = serde_json::from_str(bundle_json).unwrap();
        let oca_bundle = from_ast(None, &oca_ast).unwrap().oca_bundle;
        let oca_bundle2 = from_ast(None, &oca_ast).unwrap().oca_bundle;
        let said = oca_bundle.clone().said;
        let oca_bundle_json = serde_json::to_string_pretty(&oca_bundle).unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&oca_bundle_json).unwrap(),
            reference_json
        );
        let said2 = oca_bundle2.said;
        // Check if process is deterministic and gives always same SAID
        assert_eq!(said, said2);
    }
}
