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
use isolang::Language;
use oca_ast::ast::{CaptureContent, Command, CommandType, OCAAst, ObjectKind, OverlayContent};

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


    // pub fn generate_bundle(&mut self, context: Arc<OCAContext>) -> OCABundle {
    //     let mut capture_base = self.generate_capture_base();
    //
    //     capture_base.calculate_said();
    //
    //     let cb_said = capture_base.said.as_ref();
    //     self.overlays
    //         .iter_mut()
    //         .for_each(|x| x.calculate_said(cb_said.unwrap()));
    //
    //     let mut oca_bundle = OCABundle {
    //         said: None,
    //         capture_base,
    //         overlays: self.overlays.clone(),
    //         context,
    //     };
    //
    //     oca_bundle.fill_said();
    //     oca_bundle
    // }
    //
    // fn generate_capture_base(&mut self) -> CaptureBase {
    //     let mut capture_base = CaptureBase::new();
    //
    //     for attribute in self.attributes.values() {
    //         capture_base.add(attribute);
    //     }
    //     capture_base
    // }
}

#[cfg(test)]
mod tests {

    use oca_file::ocafile::parse_from_string;

    use crate::build::from_ast;

    use super::*;

    #[test]
    fn build_oca_bundle() {
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
        let mut b1 = OCABundle::default();
        b1.context = Arc::new(OCAContext::new(registry.clone()));

        let mut b2 = OCABundle::default();
        b2.context = Arc::new(OCAContext::new(registry));

        let oca_bundle = from_ast(Some(b1), &oca_ast).unwrap().oca_bundle;
        let oca_bundle2 = from_ast(Some(b2), &oca_ast).unwrap().oca_bundle;
        // println!("{}", oca_bundle_json); */
        let said = oca_bundle.clone().said;
        // let oca_bundle = oca.generate_bundle();
        let oca_bundle_json = serde_json::to_string_pretty(&oca_bundle).unwrap();
        let said2 = oca_bundle2.said;
        println!("{}", oca_bundle_json);
        assert_eq!(said, said2);
    }
}
