use crate::state::oca::overlay::Overlay;
use crate::state::oca::OCABundle;
use crate::state::attribute::Attribute;
use oca_ast::ast;

#[derive(Debug)]
pub struct OCABuild {
    pub oca_bundle: OCABundle,
    pub steps: Vec<OCABuildStep>,
}

#[derive(Debug)]
pub struct OCABuildStep {
    pub parent_said: Option<said::SelfAddressingIdentifier>,
    pub command: ast::Command,
    pub result: OCABundle,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FromASTError {
    pub line_number: usize,
    pub raw_line: String,
    pub message: String,
}

#[derive(thiserror::Error, Debug, Clone, serde::Serialize)]
#[serde(untagged)]
pub enum Error {
    #[error("Error at line {line_number} ({raw_line}): {message}")]
    FromASTError {
        #[serde(rename = "ln")]
        line_number: usize,
        #[serde(rename = "c")]
        raw_line: String,
        #[serde(rename = "e")]
        message: String,
    },
}

/// Create a new OCA build from OCA AST
pub fn from_ast(
    from_bundle: Option<OCABundle>,
    oca_ast: &ast::OCAAst,
) -> Result<OCABuild, Vec<Error>> {
    let mut errors = vec![];
    let mut steps = vec![];
    let mut parent_said: Option<said::SelfAddressingIdentifier> = match &from_bundle {
        Some(oca_bundle) => oca_bundle.said.clone(),
        None => None,
    };
    let has_from_bundle = from_bundle.is_some();

    let mut oca_bundle = from_bundle.unwrap_or_else(|| OCABundle::default());

    let default_command_meta = ast::CommandMeta {
        line_number: 0,
        raw_line: "unknown".to_string(),
    };
    for (i, command) in oca_ast.commands.iter().enumerate() {
        let command_index = if has_from_bundle { i + 1 } else { i };

        // todo pass the references
        let command_meta = oca_ast
            .commands_meta
            .get(&command_index)
            .unwrap_or(&default_command_meta);

        match apply_command(&mut oca_bundle, command.clone()) {
            Ok(oca_bundle) => {
                /* if oca_bundle.said == parent_said {
                    errors.push(Error::FromASTError {
                        line_number: command_meta.line_number,
                        raw_line: command_meta.raw_line.clone(),
                        message: "Applying command failed".to_string(),
                    });
                } else { */
                steps.push(OCABuildStep {
                    parent_said: parent_said.clone(),
                    command: command.clone(),
                    result: oca_bundle.clone(),
                });
                parent_said.clone_from(&oca_bundle.said);
            }
            Err(mut err) => {
                errors.extend(err.iter_mut().map(|e| Error::FromASTError {
                    line_number: command_meta.line_number,
                    raw_line: command_meta.raw_line.clone(),
                    message: e.clone(),
                }));
            }
        }
    }
    if errors.is_empty() {
        Ok(OCABuild {
            oca_bundle,
            steps,
        })
    } else {
        Err(errors)
    }
}

pub fn apply_command(base: &mut OCABundle, op: ast::Command) -> Result<&OCABundle, Vec<String>> {
    let mut errors = vec![];

    match (op.kind, op.object_kind) {
        (ast::CommandType::From, _) => {
            errors.push(
                "Unsupported FROM command, it should be resolved before applying commands"
                    .to_string(),
            );
        }
        (ast::CommandType::Add, ast::ObjectKind::CaptureBase(content)) => {
            if let Some(ref attributes) = content.attributes {
                base.capture_base.attributes.extend(attributes.clone());
                for (attr_name, _) in attributes {
                    let attribute = Attribute::new(attr_name.clone());
                    base.attributes.insert(attr_name.to_string(), attribute);
                }
            }
        }
        (ast::CommandType::Add, ast::ObjectKind::Overlay(content)) => {
            let mut overlay = Overlay::new(content.clone());
            overlay.overlay_def = Some(op.overlay_def.clone().unwrap());
            base.overlays.push(overlay);
        }
        (ast::CommandType::Add, ast::ObjectKind::OCABundle(_)) => todo!(),
        (ast::CommandType::Remove, ast::ObjectKind::CaptureBase(content)) => {
            if let Some(ref attributes) = content.attributes {
                for (attr_name, _) in attributes {
                    base.remove_attribute(attr_name);
                }
            }
        }
        (ast::CommandType::Remove, ast::ObjectKind::OCABundle(_)) => todo!(),
        (ast::CommandType::Remove, ast::ObjectKind::Overlay(_)) => todo!(),
        (ast::CommandType::Modify, ast::ObjectKind::CaptureBase(_)) => todo!(),
        (ast::CommandType::Modify, ast::ObjectKind::OCABundle(_)) => todo!(),
        (ast::CommandType::Modify, ast::ObjectKind::Overlay(_)) => todo!(),
    }


    base.compute();

    if errors.is_empty() {
        Ok(base)
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::state::oca::OCAContext;

    use super::*;
    use indexmap::IndexMap;
    use oca_ast::ast::{AttributeType, CaptureContent};
    use oca_file::ocafile::parse_from_string;
    use overlay_file::overlay_registry::{OverlayLocalRegistry, OverlayRegistry};
    use said::{derivation::HashFunctionCode, sad::SerializationFormats, version::Encode};

    #[test]
    fn test_add_step() -> Result<(), Box<dyn std::error::Error>> {
        let _ = env_logger::builder().is_test(true).try_init();
        let mut commands = vec![];

        let registry = OverlayLocalRegistry::from_dir("../overlay-file/core_overlays/")?;

        let mut attributes = IndexMap::new();
        attributes.insert(
            "d".to_string(),
            ast::NestedAttrType::Value(AttributeType::Text),
        );
        attributes.insert(
            "i".to_string(),
            ast::NestedAttrType::Value(AttributeType::Text),
        );
        attributes.insert(
            "passed".to_string(),
            ast::NestedAttrType::Value(AttributeType::Boolean),
        );
        commands.push(ast::Command {
            kind: ast::CommandType::Add,
            object_kind: ast::ObjectKind::CaptureBase(CaptureContent {
                attributes: Some(attributes),
                properties: None,
            }),
            overlay_def: None,
        });

        let mut properties = IndexMap::new();
        properties.insert(
            "lang".to_string(),
            ast::NestedValue::Value("en".to_string()),
        );
        properties.insert(
            "name".to_string(),
            ast::NestedValue::Value("Entrance credential".to_string()),
        );
        properties.insert(
            "description".to_string(),
            ast::NestedValue::Value("Entrance credential".to_string()),
        );
        let meta_ov_def = registry.get_by_fqn("Meta/2.0.0").unwrap();
        commands.push(ast::Command {
            kind: ast::CommandType::Add,
            object_kind: ast::ObjectKind::Overlay(
                ast::OverlayContent {
                    properties: Some(properties),
                    overlay_name: "Label/2.0.0".to_string(),
                },
            ),
            overlay_def: meta_ov_def.cloned(),
        });

        let mut properties = IndexMap::new();
        properties.insert(
            "d".to_string(),
            ast::NestedValue::Value("Schema digest".to_string()),
        );
        properties.insert(
            "i".to_string(),
            ast::NestedValue::Value("Credential Issuee".to_string()),
        );
        properties.insert(
            "passed".to_string(),
            ast::NestedValue::Value("Passed".to_string()),
        );
        let mut attr_labels = IndexMap::new();
        attr_labels.insert(
            "lang".to_string(),
            ast::NestedValue::Value("en".to_string()),
        );
        attr_labels.insert(
            "attribute_labels".to_string(),
            ast::NestedValue::Object(properties.clone()),
        );
        let label_ov_def = registry.get_by_fqn("Label/2.0.0").unwrap();
        commands.push(ast::Command {
            kind: ast::CommandType::Add,
            object_kind: ast::ObjectKind::Overlay(
                ast::OverlayContent {
                    properties: Some(attr_labels),
                    overlay_name: "Label/2.0.0".to_string(),
                },
            ),
            overlay_def: label_ov_def.cloned(),
        });

        let mut attributes = IndexMap::new();
        attributes.insert(
            "d".to_string(),
            ast::NestedValue::Value("Schema digest".to_string()),
        );
        attributes.insert(
            "i".to_string(),
            ast::NestedValue::Value("Credential Issuee".to_string()),
        );
        attributes.insert(
            "passed".to_string(),
            ast::NestedValue::Value("Enables or disables passing".to_string()),
        );
        let mut properties = IndexMap::new();
        properties.insert(
            "lang".to_string(),
            ast::NestedValue::Value("en".to_string()),
        );

        let mut properties = IndexMap::new();
        properties.insert(
            "d".to_string(),
            ast::NestedValue::Value("utf-8".to_string()),
        );
        properties.insert(
            "i".to_string(),
            ast::NestedValue::Value("utf-8".to_string()),
        );
        properties.insert(
            "passed".to_string(),
            ast::NestedValue::Value("utf-8".to_string()),
        );
        let character_encoding_ov_def = registry.get_by_fqn("Character_Encoding/2.0.0").unwrap();
        commands.push(ast::Command {
            kind: ast::CommandType::Add,
            object_kind: ast::ObjectKind::Overlay(
                ast::OverlayContent {
                    properties: Some(properties),
                    overlay_name: "Character_Encoding/2.0.0".to_string(),
                },
            ),
            overlay_def: character_encoding_ov_def.cloned(),
        });

        let mut properties = IndexMap::new();
        properties.insert("d".to_string(), ast::NestedValue::Value("M".to_string()));
        properties.insert("i".to_string(), ast::NestedValue::Value("M".to_string()));
        properties.insert(
            "passed".to_string(),
            ast::NestedValue::Value("M".to_string()),
        );
        let conformance_ov_def = registry.get_by_fqn("Conformance/2.0.0").unwrap();
        commands.push(ast::Command {
            kind: ast::CommandType::Add,
            object_kind: ast::ObjectKind::Overlay(
                ast::OverlayContent {
                    properties: Some(properties),
                    overlay_name: "Conformance/2.0.0".to_string(),
                },
            ),
            overlay_def: conformance_ov_def.cloned(),
        });

        // todo test if references with name are working
        let mut base = OCABundle::default();
        base.set_context(Arc::new(OCAContext::new(registry)));

        for command in commands {
            match apply_command(&mut base, command.clone()) {
                Ok(oca) => {
                    println!("Bundle: {}", serde_json::to_string_pretty(oca)?);
                }
                Err(errors) => {
                    println!("Error applying command: {:?}", errors);
                    return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("{:?}", errors))));
                }
            }
        }
        assert_eq!(
            base.attributes.len(),
            3,
            "Expected 5 attributes in the base after applying commands"
        );
        // TODO check if overlays are created correctly
        // let mut oca = apply_command(base, command);
        // base = Some(oca);
        Ok(())
    }

    #[test]
    fn build_from_ast() {
        let registry = OverlayLocalRegistry::from_dir("../overlay-file/core_overlays").unwrap();

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
        let oca_ast = parse_from_string(unparsed_file.to_string(), &registry).unwrap();

        let mut oca_bundle = OCABundle::default();
        oca_bundle.set_context(Arc::new(OCAContext::new(registry)));

        let build_result = from_ast(Some(oca_bundle), &oca_ast);
        match build_result {
            Ok(oca_build) => {
                assert_eq!(oca_build.oca_bundle.overlays.len(), 9);
                assert_eq!(oca_build.oca_bundle.capture_base.attributes.len(), 10);
                assert!(oca_build.oca_bundle.said.is_some());
                let code = HashFunctionCode::Blake3_256;
                let format = SerializationFormats::JSON;
                let oca_bundle_encoded = oca_build.oca_bundle.encode(&code, &format).unwrap();
                let oca_bundle_json = String::from_utf8(oca_bundle_encoded).unwrap();
                println!("Bundle: {}", oca_bundle_json);
            }
            Err(e) => {
                println!("{:?}", e);
            }
        }
    }
}
