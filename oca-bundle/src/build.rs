use crate::state::oca::OCABundle;
use crate::state::{
    attribute::Attribute, oca::OCABox,
};
use oca_ast::ast;
use log::{debug, info};

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

pub fn from_ast(
    from_oca: Option<OCABundle>,
    oca_ast: &ast::OCAAst,
) -> Result<OCABuild, Vec<Error>> {
    let mut errors = vec![];
    let mut steps = vec![];
    let mut parent_said: Option<said::SelfAddressingIdentifier> = match &from_oca {
        Some(oca_bundle) => oca_bundle.said.clone(),
        None => None,
    };
    let mut base: Option<OCABox> = from_oca.clone().map(OCABox::from);
    let default_command_meta = ast::CommandMeta {
        line_number: 0,
        raw_line: "unknown".to_string(),
    };
    for (i, command) in oca_ast.commands.iter().enumerate() {
        let command_index = match &from_oca {
            Some(_) => i + 1,
            None => i,
        };
        // todo pass the references
        let command_meta = oca_ast
            .commands_meta
            .get(&command_index)
            .unwrap_or(&default_command_meta);

        match apply_command(base.clone(), command.clone()) {
            Ok(oca_box) => {
                let mut oca_box_mut = oca_box.clone();
                let oca_bundle = oca_box_mut.generate_bundle();
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
                base = Some(oca_box);
                //}
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
            oca_bundle: base.unwrap().generate_bundle(),
            steps,
        })
    } else {
        Err(errors)
    }
}

pub fn apply_command(base: Option<OCABox>, op: ast::Command) -> Result<OCABox, Vec<String>> {
    let mut errors = vec![];
    let mut oca: OCABox = match base {
        Some(oca) => oca,
        None => OCABox::new(),
    };

    match (op.kind, op.object_kind) {
        (ast::CommandType::From, _) => {
            errors.push(
                "Unsupported FROM command, it should be resolved before applying commands"
                    .to_string(),
            );
        }
        (ast::CommandType::Add, ast::ObjectKind::CaptureBase(content)) => {
            if let Some(ref attributes) = content.attributes {
                for (attr_name, attr_type) in attributes {
                    let mut attribute = Attribute::new(attr_name.clone());
                    attribute.set_attribute_type(attr_type.clone());
                    oca.add_attribute(attribute);
                }
            }
        }
        (ast::CommandType::Add, ast::ObjectKind::Overlay(overlay_type, content)) => {
            oca.add_overlay(overlay_type.clone(), Some(content.clone()));
        }
        (ast::CommandType::Add, ast::ObjectKind::OCABundle(_)) => todo!(),
        (ast::CommandType::Remove, ast::ObjectKind::CaptureBase(content)) => {
            if let Some(ref attributes) = content.attributes {
                for (attr_name, _) in attributes {
                    oca.remove_attribute(attr_name);
                }
            }
        }
        (ast::CommandType::Remove, ast::ObjectKind::OCABundle(_)) => todo!(),
        (ast::CommandType::Remove, ast::ObjectKind::Overlay(_, _)) => todo!(),
        (ast::CommandType::Modify, ast::ObjectKind::CaptureBase(_)) => todo!(),
        (ast::CommandType::Modify, ast::ObjectKind::OCABundle(_)) => todo!(),
        (ast::CommandType::Modify, ast::ObjectKind::Overlay(_, _)) => todo!(),
    }

    if errors.is_empty() {
        Ok(oca)
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::state::oca::overlay::set_global_registry;

    use super::*;
    use indexmap::IndexMap;
    use oca_ast::ast::{AttributeType, CaptureContent};
    use overlay_file::overlay_registry::OverlayLocalRegistry;
    use said::{derivation::HashFunctionCode, sad::SerializationFormats, version::Encode};

    #[test]
    fn test_add_step() -> Result<(), Box<dyn std::error::Error>> {
        let _ = env_logger::builder().is_test(true).try_init();
        let mut commands = vec![];

        let registry = OverlayLocalRegistry::from_dir("../overlay-file/core_overlays/")?;
        set_global_registry(registry);

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
        commands.push(ast::Command {
            kind: ast::CommandType::Add,
            object_kind: ast::ObjectKind::Overlay(
                "Meta/2.0.0".to_string(),
                ast::Content {
                    properties: Some(properties),
                },
            ),
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
        commands.push(ast::Command {
            kind: ast::CommandType::Add,
            object_kind: ast::ObjectKind::Overlay(
                "Label/2.0.0".to_string(),
                ast::Content {
                    properties: Some(attr_labels),
                },
            ),
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
        commands.push(ast::Command {
            kind: ast::CommandType::Add,
            object_kind: ast::ObjectKind::Overlay(
                "Character_Encoding/2.0.0".to_string(),
                ast::Content {
                    properties: Some(properties),
                },
            ),
        });

        let mut properties = IndexMap::new();
        properties.insert("d".to_string(), ast::NestedValue::Value("M".to_string()));
        properties.insert("i".to_string(), ast::NestedValue::Value("M".to_string()));
        properties.insert(
            "passed".to_string(),
            ast::NestedValue::Value("M".to_string()),
        );
        commands.push(ast::Command {
            kind: ast::CommandType::Add,
            object_kind: ast::ObjectKind::Overlay(
                "Conformance/2.0.0".to_string(),
                ast::Content {
                    properties: Some(properties),
                },
            ),
        });

        // todo test if references with name are working
        let mut base: Option<OCABox> = None;
        for command in commands {
            match apply_command(base.clone(), command.clone()) {
                Ok(oca) => {
                    base = Some(oca);
                    let bundle = base.clone().unwrap().generate_bundle();
                    println!("Bundle: {}", serde_json::to_string_pretty(&bundle)?);
                }
                Err(errors) => {
                    println!("Error applying command: {:?}", errors);
                    return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("{:?}", errors))));
                }
            }
        }
        assert_eq!(
            base.as_ref().unwrap().attributes.len(),
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
        let mut commands = vec![];

        let registry = OverlayLocalRegistry::from_dir("../overlay-file/core_overlays/").unwrap();
        set_global_registry(registry);

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
            "list".to_string(),
            ast::NestedAttrType::Value(AttributeType::Text),
        );
        attributes.insert(
            "passed".to_string(),
            ast::NestedAttrType::Value(AttributeType::Boolean),
        );

        commands.push(ast::Command {
            kind: ast::CommandType::Add,
            object_kind: ast::ObjectKind::CaptureBase(ast::CaptureContent {
                attributes: Some(attributes),
                properties: None,
            }),
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
        commands.push(ast::Command {
            kind: ast::CommandType::Add,
            object_kind: ast::ObjectKind::Overlay(
                "Meta/2.0.0".to_string(),
                ast::Content {
                    properties: Some(properties),
                },
            ),
        });

        let mut attr_labels = IndexMap::new();
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
        attr_labels.insert(
            "lang".to_string(),
            ast::NestedValue::Value("en".to_string()),
        );
        attr_labels.insert("attribute_labels".to_string(), ast::NestedValue::Object(properties.clone()));
        commands.push(ast::Command {
            kind: ast::CommandType::Add,
            object_kind: ast::ObjectKind::Overlay(
                "Label/2.0.0".to_string(),
                ast::Content {
                    properties: Some(attr_labels),
                },
            ),
        });


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
        let mut attribute_character_encoding =  IndexMap::new();
        attribute_character_encoding.insert(
            "lang".to_string(),
            ast::NestedValue::Value("en".to_string()),
        );
        attribute_character_encoding.insert(
            "attribute_character_encoding".to_string(),
            ast::NestedValue::Object(properties.clone()),
        );
        commands.push(ast::Command {
            kind: ast::CommandType::Add,
            object_kind: ast::ObjectKind::Overlay(
                "Character_Encoding/2.0.0".to_string(),
                ast::Content {
                    properties: Some(attribute_character_encoding),
                },
            ),
        });

        let mut attributes = IndexMap::new();
        attributes.insert("d".to_string(), ast::NestedValue::Value("M".to_string()));
        attributes.insert("i".to_string(), ast::NestedValue::Value("M".to_string()));
        attributes.insert(
            "passed".to_string(),
            ast::NestedValue::Value("M".to_string()),
        );
        commands.push(ast::Command {
            kind: ast::CommandType::Add,
            object_kind: ast::ObjectKind::Overlay(
                "Conformance/2.0.0".to_string(),
                ast::Content {
                    properties: Some(attributes),
                },
            ),
        });

        let mut attributes = IndexMap::new();
        let mut grouped_elements = IndexMap::new();
        grouped_elements.insert(
            "g1".to_string(),
            ast::NestedValue::Array(vec![ast::NestedValue::Value("el1".to_string())]),
        );
        grouped_elements.insert(
            "g2".to_string(),
            ast::NestedValue::Array(vec![
                ast::NestedValue::Value("el2".to_string()),
                ast::NestedValue::Value("el3".to_string()),
            ]),
        );
        attributes.insert(
            "list".to_string(),
            oca_ast::ast::NestedValue::Object(grouped_elements),
        );
        commands.push(ast::Command {
            kind: ast::CommandType::Add,
            object_kind: ast::ObjectKind::Overlay(
                "Entry_Code/2.0.0".to_string(),
                ast::Content {
                    properties: Some(attributes),
                },
            ),
        });

        let oca_ast = ast::OCAAst {
            version: "2.0.0".to_string(),
            commands,
            commands_meta: IndexMap::new(),
            meta: HashMap::new(),
        };

        let build_result = from_ast(None, &oca_ast);
        match build_result {
            Ok(oca_build) => {
                assert_eq!(oca_build.oca_bundle.overlays.len(), 5);
                assert_eq!(oca_build.oca_bundle.capture_base.attributes.len(), 4);
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
