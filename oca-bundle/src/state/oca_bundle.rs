use indexmap::IndexMap;
use log::info;
use overlay::{Overlay, OverlayModel};
use overlay_file::overlay_registry::OverlayLocalRegistry;
pub use said::derivation::{HashFunction, HashFunctionCode};
pub use said::error;
pub use said::{make_me_sad, ProtocolVersion, SelfAddressingIdentifier};
use serde::ser::Error;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashMap;
use thiserror::Error;
pub mod capture_base;
pub mod overlay;
use crate::state::{attribute::Attribute, oca_bundle::capture_base::CaptureBase};
use oca_ast::ast::{CaptureContent, Command, CommandType, OCAAst, ObjectKind, OverlayContent};

#[derive(Debug, Deserialize, Clone)]
// #[version(protocol = "OCAS", major = 2, minor = 0)]
pub struct OCABundle {
    pub model: OCABundleModel,
}

#[derive(Serialize, Debug, Deserialize, Clone, Default)]
pub struct OCABundleModel {
    /// CESR version of the OCA Bundle with OCAS prefix
    #[serde(rename = "v")]
    pub version: String,
    pub digest: Option<said::SelfAddressingIdentifier>,
    pub capture_base: CaptureBase,
    pub overlays: Vec<OverlayModel>,
    // Storing attributes in different model for easy read access
    pub attributes: Option<HashMap<String, Attribute>>,
}

impl From<OCABundleModel> for OCABundle {
    fn from(model: OCABundleModel) -> Self {
        OCABundle { model }
    }
}

impl Serialize for OCABundle {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;
        // Version is not serialized as it colides with make_me_sad which adds it.
        // map.serialize_entry("v", &self.model.version)?;
        map.serialize_entry("digest", &self.model.digest)?;
        map.serialize_entry("capture_base", &self.model.capture_base)?;
        let overlays: Vec<Overlay> = self.model.overlays.iter().map(Overlay::from).collect();
        map.serialize_entry("overlays", &overlays)?;
        map.end()
    }
}

impl OCABundleModel {
    pub fn new(capture_base: CaptureBase, overlays: Vec<OverlayModel>) -> Self {
        OCABundleModel {
            version: "".to_string(),
            digest: None,
            capture_base,
            overlays,
            attributes: None,
        }
    }

    pub fn to_ast(&self, registry: OverlayLocalRegistry) -> OCAAst {
        let mut ast = OCAAst::new();

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
                attributes: Some(self.capture_base.attributes.clone()),
            }),
        };
        ast.commands.push(command);

        self.overlays.iter().for_each(|overlay| {
            let overlay_content = OverlayContent {
                overlay_def: overlay.overlay_def.clone(),
                properties: overlay.properties.clone(),
            };
            let overlay_command = Command {
                kind: CommandType::Add,
                object_kind: ObjectKind::Overlay(overlay_content),
            };
            ast.commands.push(overlay_command);
        });
        ast
    }

    // Prepare list of attributes to fill with overlays properties
    pub fn fill_attributes(&mut self) {
        // to avoid misalignment, we will allways populate fresh attribute from main model
        self.attributes = Some(HashMap::new());
        if let Some(ref mut attrs) = self.attributes {
            for (attr_name, attr_type) in self.capture_base.attributes.clone() {
                let attr = Attribute {
                    name: attr_name.clone(),
                    attribute_type: Some(attr_type),
                    ..Default::default()
                };
                attrs.insert(attr_name.clone(), attr);
            }
        }
    }

    /// Remove attribute from the OCA Bundle
    /// if attribute does not exist, nothing will happen
    pub fn remove_attribute(&mut self, attr_name: &String) {
        self.capture_base.attributes.remove(attr_name);
    }

    pub fn get_attribute_by_name(&self, name: &str) -> Option<&Attribute> {
        self.attributes.as_ref().and_then(|attrs| attrs.get(name))
    }

    /// This method will compute digest for OCABundle and all it's members (capture_base and each
    /// overlay) filling as well capture_base digest into overlays. It would use external
    /// structures instead of the internal Models i.e. OCABundle vs OCABundleModel
    /// Arguments:
    /// * `self` - OCABundleModel to compute digest for
    ///
    /// Returns:
    /// * `Result<said::SelfAddressingIdentifier, OCABundleSerializationError>` - Result with computed
    ///   SAID or an error if serialization or digest computation fails.
    pub fn compute_and_fill_digest(
        &mut self,
    ) -> Result<said::SelfAddressingIdentifier, OCABundleSerializationError> {
        // Compute digest for all objects
        info!("Computing digest for OCABundle");
        // TODO change to compute and fill
        match self.capture_base.fill_digest() {
            Ok(_) => info!("Capture base digest filled successfully"),
            Err(e) => {
                return Err(OCABundleSerializationError::SerializationError(
                    e.to_string(),
                ))
            }
        }
        let cb_said = self.capture_base.digest.clone();
        info!("Capture base SAID: {:?}", cb_said);
        for overlay in &mut self.overlays {
            overlay.capture_base_said = cb_said.clone();
            // TODO change to compute and fill
            match overlay.fill_digest() {
                Ok(_) => info!("Overlay {} digest filled successfully", overlay.name),
                Err(e) => {
                    return Err(OCABundleSerializationError::SerializationError(
                        e.to_string(),
                    ))
                }
            }
        }

        let oca_bundle = OCABundle::from(self.clone());
        let serialized_bundle = serde_json::to_string(&oca_bundle)
            .map_err(|_| serde_json::Error::custom("Failed to serialize OCABundleModel"))
            .unwrap();

        let code = HashFunctionCode::Blake3_256;
        let said_field = Some("digest");
        let version = ProtocolVersion::new("OCAS", 2, 0).unwrap();
        let input = serialized_bundle.as_str();
        match make_me_sad(input, code, version, said_field) {
            Ok(sad) => {
                #[derive(Deserialize)]
                struct OCABundlePartial {
                    digest: String,
                    #[serde(rename = "v")]
                    version: String,
                }

                let bundle: OCABundlePartial = serde_json::from_str(&sad)
                    .map_err(|e| OCABundleSerializationError::SerializationError(e.to_string()))?;
                let said: SelfAddressingIdentifier = bundle.digest.parse().map_err(|_| {
                    OCABundleSerializationError::SerializationError(
                        "Failed to parse SAID".to_string(),
                    )
                })?;
                self.digest = Some(said.clone());
                self.version = bundle.version;
                Ok(said)
            }
            Err(_) => Err(OCABundleSerializationError::SerializationError(
                "Failed to compute digest for OCABundle".to_string(),
            )),
        }
    }
}
#[derive(Error, Debug)]
pub enum OCABundleSerializationError {
    #[error("Failed to serialize OCA bundle: {0}")]
    SerializationError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::from_ast;
    use oca_file::ocafile::parse_from_string;

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
  language="en"
  attribute_labels
    d="Schema digest"
    i="Credential Issuee"
    passed="Passed"
ADD Overlay FORMAT
  attribute_formats
    d="image/jpeg"
ADD Overlay UNIT
  metric_system="SI"
  attribute_units
    i="m^2"
    d="°"
ADD ATTRIBUTE list=[Text] el=Text
ADD Overlay CARDINALITY
  attribute_cardinality
    list="1-2"
ADD Overlay ENTRY_CODE
  attribute_entry_codes
    list=refs:EJeWVGxkqxWrdGi0efOzwg1YQK8FrA-ZmtegiVEtAVcu
    el=["o1", "o2", "o3"]
ADD Overlay ENTRY
  language="en"
  attribute_entries
    list=refs:EJeWVGxkqxWrdGi0efOzwg1YQK8FrA-ZmtegiVEtAVcu
    el
     o1="o1_label"
     o2="o2_label"
     o3="o3_label"
"#;
        let registry = OverlayLocalRegistry::from_dir("../overlay-file/core_overlays/").unwrap();
        let oca_ast = parse_from_string(unparsed_file.to_string(), &registry).unwrap();

        let bundle_json = r#"
{"digest":"EC2U8zJYkTg7aS_-lMWKeis_FcS6E7D0sSf0y-BVMWUl","capture_base":{"digest":"EMDyoUr57UN7-Wy3kmF0WyG0xiQieckUdW18VGdEuve9","type":"capture_base/2.0.0","attributes":{"age":"Numeric","car":["refs:EJeWVGxkqxWrdGi0efOzwg1YQK8FrA-ZmtegiVEtAVcu"],"d":"Text","el":"Text","i":"Text","incidentals_spare_parts":[["refs:EJeWVGxkqxWrdGi0efOzwg1YQK8FrA-ZmtegiVEtAVcu"]],"list":["Text"],"name":"Text","passed":"Boolean"}},"overlays":[{"digest":"EFFHcSASwQWOku5wbTC9ad40N8478knwxTfe5zFuMlvb","capture_base":"EMDyoUr57UN7-Wy3kmF0WyG0xiQieckUdW18VGdEuve9","type":"meta","language":"en","description":"Entrance credential","name":"Entrance credential"},{"digest":"ED4PpbQO_B2PkuuPXmxGP1ZHgXN4ur3t2-scipoqBW9v","capture_base":"EMDyoUr57UN7-Wy3kmF0WyG0xiQieckUdW18VGdEuve9","type":"character_encoding","attribute_character_encoding":{"d":"utf-8","i":"utf-8","passed":"utf-8"}},{"digest":"EKZbyzgubAKEFt6MYUlNgxa98Q0aWiirR0MQbJF2RX24","capture_base":"EMDyoUr57UN7-Wy3kmF0WyG0xiQieckUdW18VGdEuve9","type":"conformance","attribute_conformance":{"d":"M","i":"M","passed":"M"}},{"digest":"EE0yQaUPDqsxVxFQzC6Ib67EnMulTeJoDGAMhqc3qrSi","capture_base":"EMDyoUr57UN7-Wy3kmF0WyG0xiQieckUdW18VGdEuve9","type":"label","language":"en","attribute_labels":{"d":"Schema digest","i":"Credential Issuee","passed":"Passed"}},{"digest":"EDuHQ3YEZORvlPOriuM4KdZrz69hjwdgroAkFWe60DF8","capture_base":"EMDyoUr57UN7-Wy3kmF0WyG0xiQieckUdW18VGdEuve9","type":"format","attribute_formats":{"d":"image/jpeg"}},{"digest":"EPA9RgPvEo-Af-8lGa9C5hKTaw6Eh6v0r-tf_wHh0yN-","capture_base":"EMDyoUr57UN7-Wy3kmF0WyG0xiQieckUdW18VGdEuve9","type":"unit","metric_system":"SI","attribute_units":{"i":"m^2","d":"°"}},{"digest":"EFC4-Dl4RtHS6m3cCKAKDNjVxXjdNpeAFE2dmcKQYGCD","capture_base":"EMDyoUr57UN7-Wy3kmF0WyG0xiQieckUdW18VGdEuve9","type":"cardinality","attribute_cardinality":{"list":"1-2"}},{"digest":"ELjGMc0PWZYL6StsNl3zRf_9MVhGEfvpuPj8Dx-BZ95Y","capture_base":"EMDyoUr57UN7-Wy3kmF0WyG0xiQieckUdW18VGdEuve9","type":"entry_code","attribute_entry_codes":{"list":"refs:EJeWVGxkqxWrdGi0efOzwg1YQK8FrA-ZmtegiVEtAVcu","el":["o1","o2","o3"]}},{"digest":"EFwgeBUcFueINWXsMndxg6UFTh7LrMnk4NvWlCUQCns_","capture_base":"EMDyoUr57UN7-Wy3kmF0WyG0xiQieckUdW18VGdEuve9","type":"entry","language":"en","attribute_entries":{"list":"refs:EJeWVGxkqxWrdGi0efOzwg1YQK8FrA-ZmtegiVEtAVcu","el":{"o1":"o1_label","o2":"o2_label","o3":"o3_label"}}}]}
"#;
        let reference_json: serde_json::Value = serde_json::from_str(bundle_json).unwrap();
        let mut oca_bundle = from_ast(None, &oca_ast).unwrap().oca_bundle;
        let mut oca_bundle2 = from_ast(None, &oca_ast).unwrap().oca_bundle;
        oca_bundle.compute_and_fill_digest().unwrap();
        oca_bundle2.compute_and_fill_digest().unwrap();

        let mut overlay_model = oca_bundle.overlays.first().unwrap().clone();
        // compute digest on overlay directly to see if the process is deterministic and equals
        // with compute on bundle
        match overlay_model.fill_digest() {
            Ok(_) => info!("Overlay model digest filled successfully"),
            Err(e) => panic!("Failed to fill overlay model digest: {}", e),
        }
        let meta_said = overlay_model.digest.clone().unwrap();
        let ref_said = "EFFHcSASwQWOku5wbTC9ad40N8478knwxTfe5zFuMlvb";
        assert_eq!(meta_said.to_string(), ref_said.to_string());
        assert_eq!(oca_bundle.version, "OCAS02JSON000961_");

        let said = oca_bundle.digest.clone().unwrap();
        let bundle = OCABundle::from(oca_bundle.clone());
        let oca_bundle_json = serde_json::to_string(&bundle).unwrap();

        println!("OCA Bundle JSON: \n{}", oca_bundle_json);

        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&oca_bundle_json).unwrap(),
            reference_json
        );
        let said2 = oca_bundle2.digest.unwrap();
        // Check if process is deterministic and gives always same SAID
        assert_eq!(said, said2);
    }
}
