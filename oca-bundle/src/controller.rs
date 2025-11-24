use std::io::Read;

use log::info;
use overlay_file::overlay_registry::OverlayRegistry;

use crate::state::oca_bundle::{OCABundle, OCABundleModel, OCABundleWithRegistry};

pub type GenericError = Box<dyn std::error::Error + Sync + Send>;
pub type GenericResult<T> = Result<T, GenericError>;

/// Loads an OCA Bundle JSON representation from a reader.
/// validate created OCA Bundle Model with overlay definitions provided by overlay overlay_registry
/// Return error or validated OCA Bundle Model
pub fn load_oca(
    source: &mut dyn Read,
    overlay_registry: &overlay_file::overlay_registry::OverlayLocalRegistry,
) -> GenericResult<OCABundleModel> {
    let mut oca: OCABundle = serde_json::from_reader(source)?;

    // fill overlay_def
    for overlay in &mut oca.overlays {
        let overlay_type = &overlay.model.name;
        match overlay_registry.get_by_fqn(overlay_type) {
            Ok(overlay_def) => {
                overlay.model.overlay_def = Some(overlay_def.clone());
            }
            Err(e) => {
                return Err(format!(
                    "Failed to find overlay definition for {}: {}",
                    overlay_type, e
                )
                .into());
            }
        }
    }

    let bundle_with_registry = OCABundleWithRegistry {
        bundle: oca,
        registry: overlay_registry.clone(),
    };
    // Convert OCABundle to OCABundleModel
    let oca_bundle_model = OCABundleModel::from(bundle_with_registry);

    match validate_bundle(&oca_bundle_model) {
        Ok(()) => {
            // When validation passes convert to Oca Bundle model
            Ok(oca_bundle_model)
        }
        Err(e) => Err(e),
    }
}

/// Validates an OCA bundle against provided overlay definitions.
/// This should be called after `load_oca` when overlay definitions are available.
fn validate_bundle(bundle: &OCABundleModel) -> GenericResult<()> {
    let said = bundle.digest.clone();
    let mut mut_bundle = bundle.clone();

    // Validate each overlay against its definition
    match mut_bundle.compute_and_fill_digest() {
        Ok(_) => info!("Digest filled successfully"),
        Err(e) => panic!("Failed to fill digest: {}", e),
    }
    if said == mut_bundle.digest {
        Ok(())
    } else {
        Err("Digests do not match".into())
    }
}

#[cfg(test)]
mod tests {
    use overlay_file::overlay_registry::OverlayLocalRegistry;

    use super::load_oca;

    #[test]
    fn load_oca_bundle_json_from_str() {
        let data = r#"
{"v":"OCAS02JSON0009e3_","digest":"EP79WPhSehW5kVwy67UR-bJEoGMUWcN5cK99THUtnBIm","capture_base":{"digest":"EMDyoUr57UN7-Wy3kmF0WyG0xiQieckUdW18VGdEuve9","type":"capture_base/2.0.0","attributes":{"age":"Numeric","car":["refs:EJeWVGxkqxWrdGi0efOzwg1YQK8FrA-ZmtegiVEtAVcu"],"d":"Text","el":"Text","i":"Text","incidentals_spare_parts":[["refs:EJeWVGxkqxWrdGi0efOzwg1YQK8FrA-ZmtegiVEtAVcu"]],"list":["Text"],"name":"Text","passed":"Boolean"}},"overlays":[{"digest":"EEk6wQBfPuqddeVOPFLgSY9qv1ZorGCvip_oQtFdD9GV","capture_base":"EMDyoUr57UN7-Wy3kmF0WyG0xiQieckUdW18VGdEuve9","type":"overlay/meta/2.0.0","language":"en","description":"Entrance credential","name":"Entrance credential"},{"digest":"EPVOc4fR5Nwe2yHzFS-4wBf3kcm7C5D4XNjY9cxnFaQh","capture_base":"EMDyoUr57UN7-Wy3kmF0WyG0xiQieckUdW18VGdEuve9","type":"overlay/character_encoding/2.0.0","attribute_character_encodings":{"d":"utf-8","i":"utf-8","passed":"utf-8"}},{"digest":"EIHoDc5WM8Yxxhvqnc9348DL-OU1FCb9K5eXUuiISztT","capture_base":"EMDyoUr57UN7-Wy3kmF0WyG0xiQieckUdW18VGdEuve9","type":"overlay/conformance/2.0.0","attribute_conformances":{"d":"M","i":"M","passed":"M"}},{"digest":"EEy4mJ4SIxauAyk8FI1QqBa26qG1Fqn2uhN_Vf4RMIbL","capture_base":"EMDyoUr57UN7-Wy3kmF0WyG0xiQieckUdW18VGdEuve9","type":"overlay/label/2.0.0","language":"en","attribute_labels":{"d":"Schema digest","i":"Credential Issuee","passed":"Passed"}},{"digest":"EJi35V6qV5tUhnjDR3qiB2irAKLkbQVu-rU_hehkhop1","capture_base":"EMDyoUr57UN7-Wy3kmF0WyG0xiQieckUdW18VGdEuve9","type":"overlay/format/2.0.0","attribute_formats":{"d":"image/jpeg"}},{"digest":"EPdl6CuC9i9IszrkqvEkv9qZPM-WnX47DOD80dwGiHpL","capture_base":"EMDyoUr57UN7-Wy3kmF0WyG0xiQieckUdW18VGdEuve9","type":"overlay/unit/2.0.0","metric_system":"SI","attribute_units":{"i":"m^2","d":"°"}},{"digest":"EFbS7GQMBi_RCk2Q8cJKR2ohCE--248bH1OQnwiFzmer","capture_base":"EMDyoUr57UN7-Wy3kmF0WyG0xiQieckUdW18VGdEuve9","type":"overlay/cardinality/2.0.0","attribute_cardinalities":{"list":"1-2"}},{"digest":"ED6ktKLPYEmJfYTEo7-YR-xyPwHUgpEOdEwOe_Kr6c22","capture_base":"EMDyoUr57UN7-Wy3kmF0WyG0xiQieckUdW18VGdEuve9","type":"overlay/entry_code/2.0.0","attribute_entry_codes":{"list":"refs:EJeWVGxkqxWrdGi0efOzwg1YQK8FrA-ZmtegiVEtAVcu","el":["o1","o2","o3"]}},{"digest":"EIMaWbfJ98gO1sTucmYdgaZu_u94djMa75BYl8lzkvfc","capture_base":"EMDyoUr57UN7-Wy3kmF0WyG0xiQieckUdW18VGdEuve9","type":"overlay/entry/2.0.0","language":"en","attribute_entries":{"list":"refs:EJeWVGxkqxWrdGi0efOzwg1YQK8FrA-ZmtegiVEtAVcu","el":{"o1":"o1_label","o2":"o2_label","o3":"o3_label"}}}]}
        "#;

        let registry = OverlayLocalRegistry::from_dir("../overlay-file/core_overlays/").unwrap();
        let oca_bundle = load_oca(&mut data.as_bytes(), &registry);
        println!(">>>> {:?}", oca_bundle);
        assert!(oca_bundle.is_ok());
        let oca = oca_bundle.unwrap();
        assert_eq!(
            oca.digest.as_ref().map(|d| d.to_string()).as_deref(),
            Some("EP79WPhSehW5kVwy67UR-bJEoGMUWcN5cK99THUtnBIm")
        );
        assert_eq!(oca.capture_base.attributes.len(), 9);
    }
}
