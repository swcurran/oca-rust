use crate::data_storage::DataStorage;
use oca_ast::ast::{self, RefValue};
use oca_bundle::state::oca_bundle::OCABundleModel;

pub fn build_oca(
    db: Box<dyn DataStorage>,
    commands: Vec<ast::Command>,
) -> Result<OCABundleModel, String> {
    let mut base: Option<OCABundleModel> = None;
    for command in commands {
        if let ast::CommandType::From = command.kind {
            let said = match command.clone().object_kind.oca_bundle_content() {
                Some(oca_bundle_content) => match oca_bundle_content.clone().said {
                    ast::ReferenceAttrType::Reference(refs) => match refs {
                        RefValue::Said(said) => said.to_string(),
                        RefValue::Name(_) => return Err("Not implemented".to_string()),
                    },
                },
                None => return Err("Missing bundle content".to_string()),
            };
            // TODO conside to name keys a bit more meaningful like oca.bundle
            let oca_bundle_str = match db.get(&format!("oca.{}", said))? {
                Some(oca_bundle_str) => String::from_utf8(oca_bundle_str).unwrap(),
                None => return Err("OCA not found".to_string()),
            };
            base = Some(serde_json::from_str::<OCABundleModel>(&oca_bundle_str).unwrap());
        } else {
            let mut current_base = base.take().unwrap_or_default();
            let bundle_model_result =
                oca_bundle::build::apply_command(&mut current_base, command.clone());

            match bundle_model_result {
                Ok(bundle_model) => {
                    let command_str = serde_json::to_string(&command).unwrap();

                    let mut input: Vec<u8> = vec![];
                    if let Some(base_said) = bundle_model.digest.as_ref() {
                        input.push(base_said.to_string().len().try_into().unwrap());
                        input.extend(base_said.to_string().as_bytes());
                    } else {
                        input.push(0);
                    }
                    input.push(command_str.len().try_into().unwrap());
                    input.extend(command_str.as_bytes());
                    if let Some(oca_said) = bundle_model.digest.as_ref() {
                        db.insert(&format!("oca.{}.operation", oca_said), &input)?;
                        db.insert(
                            &format!("oca.{}", oca_said),
                            &serde_json::to_vec(&bundle_model).map_err(|e| e.to_string())?,
                        )?;
                    } else {
                        input.push(0);
                    }
                }
                Err(errors) => {
                    println!("{:?}", errors);
                }
            };
        }
    }

    base.ok_or_else(|| "No OCA bundle created".to_string())
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use crate::data_storage::{DataStorage, SledDataStorage};
    use indexmap::IndexMap;
    use oca_ast::ast::{BundleContent, CaptureContent, OverlayContent};
    use overlay_file::OverlayDef;
    use said::SelfAddressingIdentifier;

    #[test]
    #[ignore]
    fn test_ocafile_build() {
        let mut commands = vec![];

        let mut attributes = IndexMap::new();
        attributes.insert(
            "d".to_string(),
            ast::NestedAttrType::Value(ast::AttributeType::Text),
        );
        attributes.insert(
            "i".to_string(),
            ast::NestedAttrType::Value(ast::AttributeType::Text),
        );
        attributes.insert(
            "passed".to_string(),
            ast::NestedAttrType::Value(ast::AttributeType::Boolean),
        );
        commands.push(ast::Command {
            kind: ast::CommandType::Add,
            object_kind: ast::ObjectKind::CaptureBase(CaptureContent {
                attributes: Some(attributes),
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
            object_kind: ast::ObjectKind::Overlay(OverlayContent {
                properties: Some(properties),
                overlay_def: OverlayDef::default(),
            }),
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
            ast::NestedValue::Value("Passed".to_string()),
        );
        let mut properties = IndexMap::new();
        properties.insert(
            "lang".to_string(),
            ast::NestedValue::Value("en".to_string()),
        );
        commands.push(ast::Command {
            kind: ast::CommandType::Add,
            object_kind: ast::ObjectKind::Overlay(OverlayContent {
                properties: Some(properties),
                overlay_def: OverlayDef::default(),
            }),
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

        let mut attributes = IndexMap::new();
        attributes.insert(
            "d".to_string(),
            ast::NestedValue::Value("utf-8".to_string()),
        );
        attributes.insert(
            "i".to_string(),
            ast::NestedValue::Value("utf-8".to_string()),
        );
        attributes.insert(
            "passed".to_string(),
            ast::NestedValue::Value("utf-8".to_string()),
        );
        commands.push(ast::Command {
            kind: ast::CommandType::Add,
            object_kind: ast::ObjectKind::Overlay(OverlayContent {
                properties: None,
                overlay_def: OverlayDef::default(),
            }),
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
            object_kind: ast::ObjectKind::Overlay(OverlayContent {
                properties: None,
                overlay_def: OverlayDef::default(),
            }),
        });

        let db = SledDataStorage::open("db_test");
        let oca = build_oca(Box::new(db), commands);

        let db_read = SledDataStorage::open("db_test");
        let op = db_read
            .get(&format!("oca.{}.operation", oca.unwrap().digest.unwrap()))
            .unwrap();
        println!("{:?}", String::from_utf8_lossy(&op.unwrap()));
        // println!("{}", serde_json::to_string_pretty(&oca.unwrap()).unwrap());

        // assert_eq!(digests, vec![44, 69, 73, 74, 71, 74, 109, 83, 95, 80, 57, 106, 119, 90, 68, 97, 109, 66, 54, 99, 84, 71, 57, 77, 111, 88, 75, 82, 117, 50, 49, 109, 121, 106, 88, 115, 77, 105, 55, 71, 89, 100, 100, 68, 121])
    }

    #[test]
    #[ignore]
    fn test_ocafile_build_from() {
        let mut commands = vec![];

        let said =
            SelfAddressingIdentifier::from_str("EF5ERATRBBN_ewEo9buQbznirhBmvrSSC0O2GIR4Gbfs")
                .unwrap();
        let reference = ast::ReferenceAttrType::Reference(RefValue::Said(said));
        commands.push(ast::Command {
            kind: ast::CommandType::From,
            object_kind: ast::ObjectKind::OCABundle(BundleContent { said: reference }),
        });

        let mut attributes = IndexMap::new();
        attributes.insert(
            "new".to_string(),
            ast::NestedAttrType::Value(ast::AttributeType::Text),
        );
        commands.push(ast::Command {
            kind: ast::CommandType::Add,
            object_kind: ast::ObjectKind::CaptureBase(CaptureContent {
                attributes: Some(attributes),
            }),
        });

        let db = SledDataStorage::open("db_test");
        let oca = build_oca(Box::new(db), commands);

        let db_read = SledDataStorage::open("db_test");
        let op = db_read
            .get(&format!("oca.{}.operation", oca.unwrap().digest.unwrap()))
            .unwrap();
        println!("{:?}", String::from_utf8_lossy(&op.unwrap()));
        // println!("{}", serde_json::to_string_pretty(&oca.unwrap()).unwrap());

        // assert_eq!(digests, vec![44, 69, 73, 74, 71, 74, 109, 83, 95, 80, 57, 106, 119, 90, 68, 97, 109, 66, 54, 99, 84, 71, 57, 77, 111, 88, 75, 82, 117, 50, 49, 109, 121, 106, 88, 115, 77, 105, 55, 71, 89, 100, 100, 68, 121])
    }
}
