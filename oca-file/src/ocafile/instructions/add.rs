use crate::ocafile::{Pair, Rule, error::InstructionError, instructions::helpers};
use indexmap::IndexMap;
use log::{debug, info};
use oca_ast::ast::{
    CaptureContent, Command, CommandType, NestedAttrType, NestedValue, ObjectKind, OverlayContent,
    RefValue,
};
use overlay_file::{OverlayDef, overlay_registry::OverlayRegistry};
use said::SelfAddressingIdentifier;

pub struct AddInstruction {}

pub fn resolve_overlay_def<'a>(
    registry: &'a dyn OverlayRegistry,
    name: &str,
) -> Result<&'a OverlayDef, InstructionError> {
    match registry.get_by_name(name) {
        Ok(None) => Err(InstructionError::UnknownOverlay(format!(
            "Overlay '{}' not found in registry",
            name
        ))),
        Ok(overlay_def) => Ok(overlay_def.unwrap()),
        Err(e) => Err(InstructionError::Unknown(e.to_string())),
    }
}

// TODO: move to helpers.rs
pub fn parse_overlay_body(
    pair: Pair,
    overlay_def: OverlayDef,
) -> Result<IndexMap<String, NestedValue>, InstructionError> {
    let mut map = IndexMap::new();
    let mut attributes: IndexMap<String, NestedValue> = IndexMap::new();
    let mut properties: IndexMap<String, NestedValue> = IndexMap::new();

    let attr_elements = overlay_def.get_attr_elements();
    let mut value: Option<NestedValue>;
    let mut key: Option<String>;

    // Find out what is set as attr-names and thorw it into attributes
    // everything else goes to properties

    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::kv_pair => {
                let mut kv_inner = item.into_inner(); // contains [key_pair]
                let key_pair = kv_inner.next().ok_or_else(|| {
                    InstructionError::Parser("Missing key-value pair in overlay body".to_string())
                })?;

                let mut key_pair_inner = key_pair.into_inner();
                key = Some(
                    key_pair_inner
                        .find(|p| p.as_rule() == Rule::attr_key)
                        .ok_or_else(|| {
                            InstructionError::Parser("Missing key in key-value pair".to_string())
                        })?
                        .as_str()
                        .to_string(),
                );

                let key_value = key_pair_inner
                    .find(|p| p.as_rule() == Rule::key_value)
                    .ok_or_else(|| {
                        InstructionError::Parser(format!(
                            "Missing value for key '{}'. Make sure the value is properly quoted (e.g., key=\"value\")",
                            key.as_ref().unwrap()
                        ))
                    })?;

                debug!("Parsed key: {:?}, value: {:?}", key, key_value);

                let key_value_inner = key_value.clone().into_inner().next().ok_or_else(|| {
                    InstructionError::Parser(format!(
                        "Empty or invalid value for key '{}'. Expected a quoted string, array, or reference",
                        key.as_ref().unwrap()
                    ))
                })?;

                match key_value_inner.as_rule() {
                    Rule::string => {
                        let string_value = key_value
                            .into_inner()
                            .next()
                            .ok_or_else(|| {
                                InstructionError::Parser(format!(
                                    "Invalid string value for key '{}'",
                                    key.as_ref().unwrap()
                                ))
                            })?
                            .into_inner()
                            .as_str()
                            .to_string();

                        value = Some(NestedValue::Value(string_value));
                        map.insert(key.clone().unwrap(), value.clone().unwrap());
                    }
                    Rule::array => {
                        let array_inner = key_value.into_inner().next().ok_or_else(|| {
                            InstructionError::Parser(format!(
                                "Invalid array value for key '{}'",
                                key.as_ref().unwrap()
                            ))
                        })?;

                        let values = array_inner
                            .into_inner()
                            .map(|v| {
                                v.into_inner()
                                    .next()
                                    .map(|inner| {
                                        NestedValue::Value(inner.into_inner().as_str().to_string())
                                    })
                                    .ok_or_else(|| {
                                        InstructionError::Parser(format!(
                                            "Invalid array element in key '{}'",
                                            key.as_ref().unwrap()
                                        ))
                                    })
                            })
                            .collect::<Result<Vec<NestedValue>, InstructionError>>()?;

                        value = Some(NestedValue::Array(values));
                        map.insert(key.clone().unwrap(), value.clone().unwrap());
                    }
                    Rule::said => {
                        let said_str = key_value
                            .clone()
                            .into_inner()
                            .next()
                            .ok_or_else(|| {
                                InstructionError::Parser(format!(
                                    "Invalid SAID reference for key '{}'",
                                    key.as_ref().unwrap()
                                ))
                            })?
                            .as_str()
                            .to_string();

                        let said = said_str.parse::<SelfAddressingIdentifier>().map_err(|e| {
                            InstructionError::Parser(format!(
                                "Invalid SAID format for key '{}': {}",
                                key.as_ref().unwrap(),
                                e
                            ))
                        })?;

                        value = Some(NestedValue::Reference(RefValue::Said(said)));
                        map.insert(key.clone().unwrap(), value.clone().unwrap());
                    }
                    _ => {
                        return Err(InstructionError::Parser(format!(
                            "Unsupported value type for key '{}': {:?}. Expected string, array, or SAID reference",
                            key.as_ref().unwrap(),
                            key_value_inner.as_rule()
                        )));
                    }
                }
            }
            Rule::nested_block => {
                let mut inner = item.into_inner();
                key = Some(
                    inner
                        .next()
                        .ok_or_else(|| {
                            InstructionError::Parser("Missing key in nested block".to_string())
                        })?
                        .as_str()
                        .to_string(),
                );

                let body = inner.next().ok_or_else(|| {
                    InstructionError::Parser(format!(
                        "Missing body for nested block '{}'",
                        key.as_ref().unwrap()
                    ))
                })?;

                let nested = parse_overlay_body(body, overlay_def.clone())?;
                value = Some(NestedValue::Object(nested));
                map.insert(key.clone().unwrap(), value.clone().unwrap());
            }
            _ => {
                debug!("Unexpected rule in overlay body: {:?}", item.as_rule());
                continue; // Skip unexpected rules
            }
        }

        let key_name = key.clone().unwrap();
        if attr_elements.contains(&key_name) {
            attributes.insert(key_name, value.unwrap());
        } else {
            properties.insert(key_name, value.unwrap());
        }
    }
    Ok(map)
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
            overlay_def: OverlayDef::default(),
        };

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
                                                    content.overlay_def = od.clone();
                                                }
                                                Err(e) => {
                                                    return Err(InstructionError::Parser(
                                                        e.to_string(),
                                                    ));
                                                }
                                            }
                                        }
                                        _ => {
                                            return Err(InstructionError::UnexpectedToken(
                                                format!(
                                                    "Overlay: unexpected token {:?}",
                                                    header.as_rule()
                                                ),
                                            ));
                                        }
                                    }
                                }
                            }
                            Rule::overlay_body => {
                                debug!("Parsing overlay body: {:?}", overlay);
                                content.properties =
                                    Some(parse_overlay_body(overlay, content.overlay_def.clone())?);
                            }
                            _ => {
                                return Err(InstructionError::UnexpectedToken(format!(
                                    "Overlay: unexpected token {:?}",
                                    overlay.as_rule()
                                )));
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
                                )));
                            }
                        }
                    }
                    debug!("Attributes: {:?}", attributes);
                    object_kind = Some(ObjectKind::CaptureBase(CaptureContent {
                        attributes: Some(attributes),
                    }));
                }
                Rule::comment => continue,
                _ => {
                    return Err(InstructionError::UnexpectedToken(format!(
                        "Overlay: unexpected token {:?}",
                        object.as_rule()
                    )));
                }
            };
        }

        Ok(Command {
            kind,
            object_kind: object_kind.unwrap(),
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
            (
                "ADD ATTRIBUTE documentNumber=[ refs:ENyO7FUBx7oILUYt8FwmLaDVmvOZGETXWHICultMSEpW ]",
                true,
            ),
            (
                "ADD ATTRIBUTE documentNumber=[refn:klient, refs:ENyO7FUBx7oILUYt8FwmLaDVmvOZGETXWHICultMSEpW]",
                false,
            ),
            (
                "ADD ATTRIBUTE documentNumber=snieg documentType=refs:ENyO7FUBx7oILUYt8FwmLaDVmvOZGETXWHICultMSEpW",
                false,
            ),
            (
                "ADD ATTRIBUTE documentNumber=refn:snieg documentType=refs:ENyO7FUBx7oILUYt8FwmLaDVmvOZGETXWHICultMSEpW",
                true,
            ),
            (
                "ADD ATTRIBUTE documentNumber=Text documentType=Numeric",
                true,
            ),
            (
                "ADD ATTRIBUTE documentNumber=Text documentType=Numeric name=Text list=[Numeric]",
                true,
            ),
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
    fn test_add_non_existing_overlay() {
        let _ = env_logger::builder().is_test(true).try_init();

        let instructions = vec![
            "ADD OVERLAY NONEXISTENT\n  language=\"en\"\n  name=\"Test\"",
            "ADD OVERLAY UNKNOWN_OVERLAY\n  attribute=\"value\"",
            "ADD OVERLAY InvalidOverlay\n  field=\"data\"",
        ];

        for instruction in instructions {
            debug!(
                "Testing non-existing overlay instruction: {:?}",
                instruction
            );
            let parsed_instruction = OCAfileParser::parse(Rule::add, instruction);
            debug!("Parsed instruction: {:?}", parsed_instruction);

            match parsed_instruction {
                Ok(mut parsed_instruction) => {
                    let instruction = parsed_instruction.next();
                    assert!(instruction.is_some());

                    if let Some(instruction) = instruction {
                        let registry =
                            OverlayLocalRegistry::from_dir("../overlay-file/core_overlays")
                                .unwrap();
                        let result = AddInstruction::from_record(instruction, 0, &registry);

                        assert!(result.is_err(), "Expected error for non-existing overlay");

                        match result.unwrap_err() {
                            InstructionError::Parser(msg) => {
                                assert!(
                                    msg.contains("Unknown overlay") || msg.contains("not found"),
                                    "Expected 'Unknown overlay' error, got: {}",
                                    msg
                                );
                                debug!("Correctly caught error: {}", msg);
                            }
                            InstructionError::UnknownOverlay(msg) => {
                                debug!("Correctly caught UnknownOverlay error: {}", msg);
                            }
                            other => {
                                panic!("Unexpected error type: {:?}", other);
                            }
                        }
                    }
                }
                Err(e) => {
                    panic!(
                        "Parsing should succeed, but overlay resolution should fail. Got parse error: {:?}",
                        e
                    );
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
                                    .overlay_def
                                    .get_full_name()
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
                                        println!(
                                            "Unknown overlay type: {}",
                                            content.overlay_def.get_full_name()
                                        );
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
