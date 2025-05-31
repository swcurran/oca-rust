use crate::ocafile::{error::InstructionError, instructions::helpers, Pair, Rule};
use indexmap::IndexMap;
use log::{debug, info};
use oca_ast::ast::{
    CaptureContent, Command, CommandType, NestedAttrType, NestedValue, ObjectKind, OverlayContent,
};
use overlay_file::{overlay_registry::OverlayRegistry, OverlayDef};

pub struct AddInstruction {}

// TODO to chyba powinno być w registry.rs i wystawione jako api
pub fn resolve_overlay_def<'a>(
    registry: &'a dyn OverlayRegistry,
    name: &str,
) -> Result<&'a OverlayDef, InstructionError> {
    match registry.get_by_name(name) {
        Ok(Some(overlay_def)) => Ok(overlay_def),
        Ok(None) => Err(InstructionError::UnknownOverlay(name.to_string())),
        Err(e) => Err(InstructionError::UnknownOverlay(e.to_string())),
    }
}

// TODO: move to helpers.rs
pub fn parse_overlay_body(pair: Pair, overlay_def: OverlayDef) -> IndexMap<String, NestedValue> {
    let mut map = IndexMap::new();
    let mut attributes: IndexMap<String, NestedValue> = IndexMap::new();
    let mut properties: IndexMap<String, NestedValue> = IndexMap::new();

    let attr_elements = overlay_def.get_attr_elements();
    let mut value: Option<NestedValue> = None;
    let mut key: Option<String> = None;

    // Find out what is set as attr-names and thorw it into attributes
    // everything else goes to properties

    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::kv_pair => {
                let mut kv_inner = item.into_inner(); // contains [key_pair]
                let key_pair = kv_inner.next().unwrap(); // rule: key_pair

                let mut key_pair_inner = key_pair.into_inner();
                debug!("Parsing key-value pair: {:?}", key_pair_inner);
                key = Some(
                    key_pair_inner
                        .find(|p| p.as_rule() == Rule::attr_key)
                        .unwrap()
                        .as_str()
                        .to_string(),
                );

                let key_value = key_pair_inner
                    .find(|p| p.as_rule() == Rule::key_value)
                    .unwrap();

                debug!("Parsed key: {:?}, value: {:?}", key, key_value);

                match key_value.clone().into_inner().next().unwrap().as_rule() {
                    Rule::string => {
                        value = Some(NestedValue::Value(
                            key_value
                                .into_inner()
                                .next()
                                .unwrap()
                                .into_inner()
                                .as_str()
                                .to_string(),
                        ));
                        map.insert(key.clone().unwrap(), value.clone().unwrap());
                    }
                    Rule::array => {
                        // TODO add support for nested arrays
                        let values = key_value
                            .into_inner()
                            .next()
                            .unwrap()
                            .into_inner()
                            .map(|v| {
                                NestedValue::Value(
                                    v.into_inner()
                                        .next()
                                        .unwrap()
                                        .into_inner()
                                        .as_str()
                                        .to_string(),
                                )
                            })
                            .collect::<Vec<NestedValue>>();
                        value = Some(NestedValue::Array(values));
                        map.insert(key.clone().unwrap(), value.clone().unwrap());
                    }
                    _ => {
                        debug!(
                            "Unexpected rule in key-value pair: {:?}",
                            key_value.as_rule()
                        );
                        continue; // Skip unexpected rules
                    }
                }
            }
            Rule::nested_block => {
                let mut inner = item.into_inner();
                key = Some(inner.next().unwrap().as_str().to_string());
                let body = inner.next().unwrap();
                let nested = parse_overlay_body(body, overlay_def.clone());
                value = Some(NestedValue::Object(nested));
                map.insert(key.clone().unwrap(), value.clone().unwrap());
            }
            _ => {
                debug!("Unexpected rule in overlay body: {:?}", item.as_rule());
                continue; // Skip unexpected rules
            }
        }
    }
    let key_name = key.clone().unwrap();
    if attr_elements.contains(&key_name) {
        attributes.insert(key_name, value.unwrap());
    } else {
        properties.insert(key_name, value.unwrap());
    }
    map
}

impl AddInstruction {
    pub(crate) fn from_record(
        record: Pair,
        _index: usize,
        registry: &dyn OverlayRegistry,
    ) -> Result<Command, InstructionError> {
        let mut object_kind = None;
        let kind = CommandType::Add;
        let mut content = OverlayContent {
            properties: None,
            overlay_name: "".to_string(),
        };
        let mut overlay_def: Option<OverlayDef> = None;

        debug!("Parsing add instruction from the record: {:?}", record);
        for object in record.into_inner() {
            match object.as_rule() {
                Rule::overlay => {
                    debug!("Parsing overlay block: {:?}", object);

                    for overlay in object.into_inner() {
                        match overlay.as_rule() {
                            Rule::overlay_header => {
                                for header in overlay.into_inner() {
                                    match header.as_rule() {
                                        Rule::overlay_name => {
                                            debug!("Parsing overlay name: {:?}", header);
                                            let name = header.as_str();
                                            match resolve_overlay_def(registry, name) {
                                                Ok(od) => {
                                                    overlay_def = Some(od.clone());
                                                    info!(
                                                        "Found overlay definition: {:?}",
                                                        overlay_def
                                                    );
                                                    content.overlay_name = overlay_def
                                                        .clone()
                                                        .unwrap()
                                                        .get_full_name();
                                                }
                                                Err(e) => {
                                                    return Err(InstructionError::Parser(
                                                        e.to_string(),
                                                    ));
                                                }
                                            }
                                        }
                                        _ => {
                                            return Err(InstructionError::UnexpectedToken(format!(
                                                "Overlay: unexpected token {:?}",
                                                header.as_rule()
                                            )))
                                        }
                                    }
                                }
                            }
                            Rule::overlay_body => {
                                debug!("Parsing overlay body: {:?}", overlay);
                                content.properties =
                                    Some(parse_overlay_body(overlay, overlay_def.clone().unwrap()));
                            }
                            _ => {
                                return Err(InstructionError::UnexpectedToken(format!(
                                    "Overlay: unexpected token {:?}",
                                    overlay.as_rule()
                                )))
                            }
                        }
                    }
                    object_kind = Some(ObjectKind::Overlay(content.clone()));
                }
                Rule::capture_base => {
                    let mut attributes: IndexMap<String, NestedAttrType> = IndexMap::new();
                    for attr_pairs in object.into_inner() {
                        match attr_pairs.as_rule() {
                            Rule::attr_pairs => {
                                debug!("Attribute pairs: {:?}", attr_pairs);
                                for attr in attr_pairs.into_inner() {
                                    debug!("Parsing attribute pair {:?}", attr);
                                    let (key, value) = helpers::extract_attribute(attr)?;
                                    info!("Parsed attribute: {:?} = {:?}", key, value);

                                    attributes.insert(key, value);
                                }
                            }
                            _ => {
                                return Err(InstructionError::UnexpectedToken(format!(
                                    "Invalid attributes in ATTRIBUTE instruction {:?}",
                                    attr_pairs.as_rule()
                                )))
                            }
                        }
                    }
                    debug!("Attributes: {:?}", attributes);
                    object_kind = Some(ObjectKind::CaptureBase(CaptureContent {
                        properties: None,
                        attributes: Some(attributes),
                    }));
                }
                Rule::comment => continue,
                _ => {
                    return Err(InstructionError::UnexpectedToken(format!(
                        "Overlay: unexpected token {:?}",
                        object.as_rule()
                    )))
                }
            };
        }

        Ok(Command {
            kind,
            object_kind: object_kind.unwrap(),
            overlay_def,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ocafile::OCAfileParser;
    use overlay_file::overlay_registry::OverlayLocalRegistry;
    use pest::Parser;

    #[test]
    fn test_add_attribute_instruction() {
        // test vector with example instruction and boolean if they should be valid or not
        let instructions = vec![
            ("ADD ATTRIBUTE documentNumber = [refn:dokument]", true),
            ("ADD ATTRIBUTE documentNumber=[[[refn:dokument]]]", true),
            ("ADD ATTRIBUTE documentNumber=[ refs:ENyO7FUBx7oILUYt8FwmLaDVmvOZGETXWHICultMSEpW ]", true),
            ("ADD ATTRIBUTE documentNumber=[refn:klient, refs:ENyO7FUBx7oILUYt8FwmLaDVmvOZGETXWHICultMSEpW]", false),
            ("ADD ATTRIBUTE documentNumber=snieg documentType=refs:ENyO7FUBx7oILUYt8FwmLaDVmvOZGETXWHICultMSEpW", false),
            ("ADD ATTRIBUTE documentNumber=refn:snieg documentType=refs:ENyO7FUBx7oILUYt8FwmLaDVmvOZGETXWHICultMSEpW", true),
            ("ADD ATTRIBUTE documentNumber=Text documentType=Numeric", true),
            ("ADD ATTRIBUTE documentNumber=Text documentType=Numeric name=Text list=[Numeric]", true),
            ("ADD ATTRIBUTE name=Text", false),
            ("ADD ATTR name=Text", false),
            ("ADD attribute name=Text", true),
            ("add attribute name=Text", true),
            ("add attribute name=Random", false),
        ];
        let _ = env_logger::builder().is_test(true).try_init();

        // loop over instructions to check if the are meeting the requirements
        for (instruction, is_valid) in instructions {
            debug!("Instruction: {:?}", instruction);
            let parsed_instruction = OCAfileParser::parse(Rule::add, instruction);
            debug!("Parsed instruction: {:?}", parsed_instruction);

            match parsed_instruction {
                Ok(mut parsed_instruction) => {
                    let instruction = parsed_instruction.next();
                    assert!(instruction.is_some());
                    match instruction {
                        Some(instruction) => {
                            let registry =
                                OverlayLocalRegistry::from_dir("../overlay-file/core_overlays")
                                    .unwrap();
                            let instruction =
                                AddInstruction::from_record(instruction, 0, &registry).unwrap();

                            assert_eq!(instruction.kind, CommandType::Add);
                            match instruction.object_kind {
                                ObjectKind::CaptureBase(content) => {
                                    if content.attributes.is_some() {
                                        assert!(!content.attributes.unwrap().is_empty());
                                    }
                                    if content.properties.is_some() {
                                        assert!(!content.properties.unwrap().is_empty());
                                    }
                                }
                                _ => {
                                    assert!(!is_valid, "Instruction is not valid");
                                }
                            }
                        }
                        None => {
                            assert!(!is_valid, "Instruction is not valid");
                        }
                    }
                }
                Err(e) => {
                    println!("Error: {:?}", e);
                    assert!(!is_valid, "Instruction should be invalid");
                }
            }
        }
    }

    #[test]
    fn test_add_overlay_instructions() {
        let instructions = vec![
            (
                "ADD OVERLAY LABEL\n  language=\"pl\"\n  attributes:\n gender=\"Opcja\"",
                true,
            ),
            ("ADD OVERLAY ENTRY_CODE\n  gender=[\"o1\",\"o2\"]", true),
            (
                "ADD OVERLAY ENTRY_CODE\n gender=[\"o1\",\"o2\", \"o3\"]",
                true,
            ),
            ("ADD OVERLAY FORMAT\n name = \"^\\d+$\"", true),
            ("ADD OVERLAY CHARACTER_ENCODING\n name=\"utf-16le\"", true),
        ];

        let _ = env_logger::builder().is_test(true).try_init();

        for (instruction, is_valid) in instructions {
            debug!("Instruction: {:?}", instruction);
            let parsed_instruction = OCAfileParser::parse(Rule::add, instruction);
            debug!("Parsed instruction: {:?}", parsed_instruction);

            match parsed_instruction {
                Ok(mut parsed_instruction) => {
                    let instruction = parsed_instruction.next();
                    assert!(instruction.is_some());
                    match instruction {
                        Some(instruction) => {
                            let registry =
                                OverlayLocalRegistry::from_dir("../overlay-file/core_overlays")
                                    .unwrap();
                            let instruction =
                                AddInstruction::from_record(instruction, 0, &registry).unwrap();

                            assert_eq!(instruction.kind, CommandType::Add);
                            match instruction.object_kind {
                                ObjectKind::Overlay(content) => match content
                                    .overlay_name
                                    .to_lowercase()
                                    .as_str()
                                {
                                    "label/2.0.0"
                                    | "entry_code/2.0.0"
                                    | "format/2.0.0"
                                    | "character_encoding/2.0.0" => {
                                        println!("Parsed overlay label: {:?}", content);
                                    }
                                    _ => {
                                        println!("Unknown overlay type: {}", content.overlay_name);
                                        assert!(!is_valid, "Instruction is not valid");
                                    }
                                },
                                ObjectKind::CaptureBase(_) => todo!(),
                                ObjectKind::OCABundle(_) => todo!(),
                            }
                        }
                        None => {
                            assert!(!is_valid, "Instruction is not valid");
                        }
                    }
                }
                Err(e) => {
                    assert!(!is_valid, "Instruction should be invalid");
                    println!("Error: {:?}", e);
                }
            }
        }
    }
}
