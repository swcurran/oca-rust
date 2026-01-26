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
    match registry.get_overlay(name) {
        Ok(overlay_def) => Ok(overlay_def),
        Err(e) => Err(InstructionError::UnknownOverlay(format!(
            "Overlay '{}' not found in registry: {}",
            name, e
        ))),
    }
}

pub fn parse_overlay_body(
    pair: Pair,
    overlay_def: OverlayDef,
) -> Result<IndexMap<String, NestedValue>, InstructionError> {
    let mut lines: Vec<(usize, Pair)> = Vec::new();

    // First pass: collect all lines with their indentation levels
    for line in pair.into_inner() {
        if line.as_rule() != Rule::overlay_line {
            continue;
        }

        let mut indent_level = 0;
        let mut content = None;

        for item in line.into_inner() {
            match item.as_rule() {
                Rule::indent => {
                    indent_level = item.as_str().len();
                }
                Rule::kv_pair | Rule::block_header => {
                    content = Some(item);
                }
                _ => {}
            }
        }

        if let Some(content) = content {
            lines.push((indent_level, content));
        }
    }

    // Second pass: group lines into blocks based on indentation
    parse_lines_with_indentation(&lines, 0, overlay_def)
}

fn parse_lines_with_indentation(
    lines: &[(usize, Pair)],
    start_idx: usize,
    overlay_def: OverlayDef,
) -> Result<IndexMap<String, NestedValue>, InstructionError> {
    let mut map = IndexMap::new();
    let mut i = start_idx;

    if lines.is_empty() {
        return Ok(map);
    }

    let base_indent = lines[start_idx].0;

    while i < lines.len() {
        let (indent, ref content) = lines[i];

        // If we've dedented, we're done with this block
        if indent < base_indent {
            break;
        }

        // Skip lines that are more indented (they belong to a sub-block)
        if indent > base_indent {
            i += 1;
            continue;
        }

        debug!("Content: {:?}", content);
        match content.as_rule() {
            Rule::kv_pair => {
                let (key, value) = parse_kv_pair(content.clone())?;
                map.insert(key, value);
                i += 1;
            }
            Rule::block_header => {
                let block_key = extract_block_key(content)?;

                // Find all lines that belong to this block (more indented)
                let block_start = i + 1;
                let mut block_end = block_start;

                if block_start < lines.len() {
                    let expected_indent = lines[block_start].0;

                    while block_end < lines.len() && lines[block_end].0 >= expected_indent {
                        block_end += 1;
                    }

                    // Recursively parse the block content
                    let nested_content =
                        parse_lines_with_indentation(lines, block_start, overlay_def.clone())?;

                    map.insert(block_key, NestedValue::Object(nested_content));
                }

                i = block_end;
            }
            _ => {
                i += 1;
            }
        }
    }

    Ok(map)
}

fn parse_kv_pair(pair: Pair) -> Result<(String, NestedValue), InstructionError> {
    let mut inner = pair.into_inner();

    debug!("Inner content: {:?}", inner);
    let key = inner
        .find(|p| p.as_rule() == Rule::attr_key)
        .ok_or_else(|| InstructionError::Parser("Missing key in kv_pair".to_string()))?
        .as_str()
        .to_string()
        .as_str()
        .trim_matches('"')
        .to_string();

    let value_pair = inner
        .find(|p| p.as_rule() == Rule::key_value)
        .ok_or_else(|| InstructionError::Parser("Missing value in kv_pair".to_string()))?;

    let value = parse_key_value(value_pair)?;

    Ok((key, value))
}

fn extract_block_key(pair: &Pair) -> Result<String, InstructionError> {
    pair.clone()
        .into_inner()
        .find(|p| p.as_rule() == Rule::attr_key)
        .map(|p| p.as_str().trim_matches('"').to_string())
        .ok_or_else(|| InstructionError::Parser("Missing key in block_header".to_string()))
}

fn parse_key_value(pair: Pair) -> Result<NestedValue, InstructionError> {
    let inner = pair
        .into_inner()
        .next()
        .ok_or_else(|| InstructionError::Parser("Empty key_value".to_string()))?;

    match inner.as_rule() {
        Rule::string => {
            let s = inner
                .into_inner()
                .next()
                .ok_or_else(|| InstructionError::Parser("Invalid string".to_string()))?
                .as_str()
                .to_string();
            // Clean the string value by removing surrounding quotes
            let cleaned = s.trim_matches('"').to_string();
            Ok(NestedValue::Value(cleaned))
        }
        Rule::array => {
            let values = inner
                .into_inner()
                .map(|v| parse_key_value(v))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(NestedValue::Array(values))
        }
        Rule::said => {
            let said = inner
                .as_str()
                .parse::<SelfAddressingIdentifier>()
                .map_err(|e| InstructionError::Parser(format!("Invalid SAID: {}", e)))?;
            Ok(NestedValue::Reference(RefValue::Said(said)))
        }
        Rule::alias => Ok(NestedValue::Reference(RefValue::Name(
            inner.as_str().to_string(),
        ))),
        _ => {
            // Fallback for plain text - also clean quotes if present
            let s = inner.as_str().trim_matches('"').to_string();
            Ok(NestedValue::Value(s))
        }
    }
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
                                                    return Err(InstructionError::UnknownOverlay(
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
            "ADD OVERLAY NONEXISTENT\n  language=\"en\"",
            "ADD OVERLAY UNKNOWN_OVERLAY\n  name=\"test\"",
            "ADD OVERLAY InvalidOverlay\n  description=\"test\"",
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
                            InstructionError::UnknownOverlay(msg) => {
                                debug!("Correctly caught UnknownOverlay error: {}", msg);
                                assert!(
                                    msg.contains("not found"),
                                    "Expected 'not found' in error message, got: {}",
                                    msg
                                );
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
            (
                "ADD OVERLAY ENTRY_CODE\n  attribute_entry_codes\n    gender=[\"o1\",\"o2\"]",
                true,
            ),
            (
                "ADD OVERLAY FORMAT\n attribute_formats\n    name = \"^\\d+$\"",
                true,
            ),
            (
                "ADD OVERLAY CHARACTER_ENCODING\n attribute_character_encodings\n    name=\"utf-16le\"",
                true,
            ),
            (
                "ADD OVERLAY META\n name=\"test\"\n  description=\"desc\"",
                true,
            ),
            ("ADD OVERLAY CODE\n gender=[\"o1\",\"o2\", \"o3\"]", false),
        ];

        let _ = env_logger::builder().is_test(true).try_init();

        for (instruction, is_valid) in instructions {
            let parsed_instruction = OCAfileParser::parse(Rule::add, instruction);

            match parsed_instruction {
                Ok(mut parsed_instruction) => {
                    let instruction = parsed_instruction.next();
                    assert!(instruction.is_some());
                    match instruction {
                        Some(instruction) => {
                            let registry =
                                OverlayLocalRegistry::from_dir("../overlay-file/core_overlays")
                                    .unwrap();
                            let result = AddInstruction::from_record(instruction, 0, &registry);

                            if is_valid {
                                let instruction = result.unwrap();
                                debug!("Parsed instruction: {:?}", instruction);

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
                                        | "meta/2.0.0"
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
                            } else {
                                assert!(result.is_err());
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

    #[test]
    fn test_add_overlay_with_multiple_nested_blocks() {
        let instructions = vec![(
            r#"ADD OVERLAY META
  language="en"
  name="Test Schema"
  description="A test schema with multiple nested blocks"
  credential_help_text
    field1="Help for field 1"
    field2="Help for field 2"
  credential_support_text
    field1="Support info for field 1"
    field2="Support info for field 2"
  credential_hint_text
    field1
        label="Support info for field 1"
        title="HINT"
    field2
        label="Support info for field 2"
        title="HINT"
"#,
            true,
        )];

        let _ = env_logger::builder().is_test(true).try_init();

        for (instruction, is_valid) in instructions {
            debug!(
                "Testing multiple nested blocks instruction: {:?}",
                instruction
            );
            let parsed_instruction = OCAfileParser::parse(Rule::add, instruction);
            debug!("Parsed instruction: {:?}", parsed_instruction);

            match parsed_instruction {
                Ok(mut parsed_instruction) => {
                    let instruction = parsed_instruction.next();
                    assert!(instruction.is_some(), "Instruction should be parsed");

                    match instruction {
                        Some(instruction) => {
                            let registry =
                                OverlayLocalRegistry::from_dir("../overlay-file/core_overlays")
                                    .unwrap();
                            let result = AddInstruction::from_record(instruction, 0, &registry);

                            match result {
                                Ok(command) => {
                                    assert_eq!(command.kind, CommandType::Add);
                                    match command.object_kind {
                                        ObjectKind::Overlay(content) => {
                                            assert!(is_valid, "Instruction should be valid");

                                            // Verify properties exist
                                            assert!(
                                                content.properties.is_some(),
                                                "Properties should be present"
                                            );

                                            let properties = content.properties.unwrap();
                                            debug!("Parsed properties: {:?}", properties);

                                            // Count nested blocks (objects)
                                            let nested_block_count = properties
                                                .values()
                                                .filter(|v| matches!(v, NestedValue::Object(_)))
                                                .count();

                                            assert!(
                                                nested_block_count >= 2,
                                                "Should have at least 2 nested blocks, found {}",
                                                nested_block_count
                                            );

                                            // Verify each nested block has content
                                            for (key, value) in properties.iter() {
                                                if let NestedValue::Object(nested_map) = value {
                                                    assert!(
                                                        !nested_map.is_empty(),
                                                        "Nested block '{}' should not be empty",
                                                        key
                                                    );
                                                    debug!(
                                                        "Nested block '{}' contains {} items",
                                                        key,
                                                        nested_map.len()
                                                    );
                                                }
                                            }

                                            println!(
                                                "Successfully parsed overlay with {} nested blocks",
                                                nested_block_count
                                            );
                                        }
                                        ObjectKind::CaptureBase(_) => {
                                            panic!("Expected Overlay, got CaptureBase");
                                        }
                                        ObjectKind::OCABundle(_) => {
                                            panic!("Expected Overlay, got OCABundle");
                                        }
                                    }
                                }
                                Err(e) => {
                                    if is_valid {
                                        panic!("Expected valid instruction but got error: {:?}", e);
                                    } else {
                                        debug!(
                                            "Correctly caught error for invalid instruction: {:?}",
                                            e
                                        );
                                    }
                                }
                            }
                        }
                        None => {
                            assert!(!is_valid, "Instruction should be invalid");
                        }
                    }
                }
                Err(e) => {
                    if is_valid {
                        panic!("Parsing should succeed but got error: {:?}", e);
                    } else {
                        debug!("Correctly failed to parse invalid instruction: {:?}", e);
                    }
                }
            }
        }
    }

    #[test]
    fn test_add_with_quoted_keys_and_values() {
        let instructions = vec![
            // Test 1: Simple quoted key and value
            (
                r#"ADD ATTRIBUTE "person.name"=Text"#,
                true,
                vec![("person.name", "Text")],
            ),
            // Test 2: Multiple quoted keys and values
            (
                r#"ADD ATTRIBUTE "first.name"=Text "last.name"=Text"#,
                true,
                vec![("first.name", "Text"), ("last.name", "Text")],
            ),
            // Test 3: Quoted key with special characters
            (
                r#"ADD ATTRIBUTE "person.email@domain"=Text"#,
                true,
                vec![("person.email@domain", "Text")],
            ),
            // Test 4: Quoted key with spaces in value
            (
                r#"ADD ATTRIBUTE "full name"=Text"#,
                true,
                vec![("full name", "Text")],
            ),
            // Test 5: Mixed quoted and unquoted (unquoted should still work)
            (
                r#"ADD ATTRIBUTE name=Text "person.age"=Numeric"#,
                true,
                vec![("name", "Text"), ("person.age", "Numeric")],
            ),
            // Test 6: Overlay with quoted properties
            (
                r#"ADD OVERLAY LABEL
  language="en"
  attributes:
    "person.name"="Full Name"
    "person.age"="Age""#,
                true,
                vec![],
            ),
            // Test 7: Nested object with quoted keys
            (
                r#"ADD OVERLAY META
  language="en"
  "credential_help_text"
    "field.one"="Help text for field one"
    "field.two"="Help text for field two""#,
                true,
                vec![],
            ),
            // Test 9: Complex nested structure with quotes
            (
                r#"ADD OVERLAY META
  language="en"
  name="Test Schema"
  "credential.hint.text"
    "user.email"
        label="Email address hint"
        title="Email Hint"
    "user.phone"
        label="Phone number hint"
        title="Phone Hint""#,
                true,
                vec![],
            ),
            // Test 10: Quoted keys with dots and underscores
            (
                r#"ADD ATTRIBUTE "experiment.range.original_values"=Text "test_field.sub.value"=Numeric"#,
                true,
                vec![
                    ("experiment.range.original_values", "Text"),
                    ("test_field.sub.value", "Numeric"),
                ],
            ),
        ];

        let _ = env_logger::builder().is_test(true).try_init();

        for (idx, (instruction, is_valid, expected_pairs)) in instructions.into_iter().enumerate() {
            debug!("Test case {}: {:?}", idx + 1, instruction);
            let parsed_instruction = OCAfileParser::parse(Rule::add, instruction);
            debug!("Parsed instruction: {:?}", parsed_instruction);

            match parsed_instruction {
                Ok(mut parsed_instruction) => {
                    let instruction = parsed_instruction.next();
                    assert!(
                        instruction.is_some(),
                        "Test case {}: Instruction should be parsed",
                        idx + 1
                    );

                    match instruction {
                        Some(instruction) => {
                            let registry =
                                OverlayLocalRegistry::from_dir("../overlay-file/core_overlays")
                                    .unwrap();
                            let result = AddInstruction::from_record(instruction, 0, &registry);

                            if is_valid {
                                let command = result.unwrap_or_else(|e| {
                                    panic!("Test case {}: Expected valid instruction but got error: {:?}", idx + 1, e)
                                });

                                assert_eq!(
                                    command.kind,
                                    CommandType::Add,
                                    "Test case {}: Should be Add command",
                                    idx + 1
                                );

                                match command.object_kind {
                                    ObjectKind::CaptureBase(content) => {
                                        if let Some(attributes) = content.attributes {
                                            debug!(
                                                "Test case {}: Parsed attributes: {:?}",
                                                idx + 1,
                                                attributes
                                            );

                                            // Verify that keys don't have quotes
                                            for (key, _) in attributes.iter() {
                                                assert!(
                                                    !key.starts_with('"') && !key.ends_with('"'),
                                                    "Test case {}: Key '{}' should not contain quotes",
                                                    idx + 1,
                                                    key
                                                );
                                            }

                                            // Verify expected key-value pairs for attribute tests
                                            if !expected_pairs.is_empty() {
                                                for (expected_key, expected_value) in
                                                    expected_pairs.iter()
                                                {
                                                    assert!(
                                                        attributes.contains_key(*expected_key),
                                                        "Test case {}: Should contain key '{}'",
                                                        idx + 1,
                                                        expected_key
                                                    );

                                                    // For simple type attributes, verify the value
                                                    if !expected_value.is_empty()
                                                        && let Some(attr_type) =
                                                            attributes.get(*expected_key)
                                                    {
                                                        match attr_type {
                                                            NestedAttrType::Value(v) => {
                                                                assert_eq!(
                                                                    format!("{:?}", v),
                                                                    *expected_value,
                                                                    "Test case {}: Value for key '{}' should be '{}'",
                                                                    idx + 1,
                                                                    expected_key,
                                                                    expected_value
                                                                );
                                                            }
                                                            _ => {
                                                                // For complex types, just verify key exists
                                                                debug!(
                                                                    "Test case {}: Complex type for key '{}'",
                                                                    idx + 1,
                                                                    expected_key
                                                                );
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    ObjectKind::Overlay(content) => {
                                        debug!(
                                            "Test case {}: Parsed overlay: {:?}",
                                            idx + 1,
                                            content.overlay_def.get_full_name()
                                        );

                                        if let Some(properties) = content.properties {
                                            debug!(
                                                "Test case {}: Overlay properties: {:?}",
                                                idx + 1,
                                                properties
                                            );

                                            // Verify that property keys don't have quotes
                                            for (key, value) in properties.iter() {
                                                assert!(
                                                    !key.starts_with('"') && !key.ends_with('"'),
                                                    "Test case {}: Property key '{}' should not contain quotes",
                                                    idx + 1,
                                                    key
                                                );

                                                // Verify nested values don't have quotes
                                                match value {
                                                    NestedValue::Value(v) => {
                                                        assert!(
                                                            !v.starts_with('"')
                                                                && !v.ends_with('"'),
                                                            "Test case {}: Value '{}' should not contain quotes",
                                                            idx + 1,
                                                            v
                                                        );
                                                    }
                                                    NestedValue::Object(nested_map) => {
                                                        for (nested_key, nested_value) in
                                                            nested_map.iter()
                                                        {
                                                            assert!(
                                                                !nested_key.starts_with('"')
                                                                    && !nested_key.ends_with('"'),
                                                                "Test case {}: Nested key '{}' should not contain quotes",
                                                                idx + 1,
                                                                nested_key
                                                            );

                                                            if let NestedValue::Value(v) =
                                                                nested_value
                                                            {
                                                                assert!(
                                                                    !v.starts_with('"')
                                                                        && !v.ends_with('"'),
                                                                    "Test case {}: Nested value '{}' should not contain quotes",
                                                                    idx + 1,
                                                                    v
                                                                );
                                                            }
                                                        }
                                                    }
                                                    NestedValue::Array(arr) => {
                                                        for item in arr.iter() {
                                                            if let NestedValue::Value(v) = item {
                                                                assert!(
                                                                    !v.starts_with('"')
                                                                        && !v.ends_with('"'),
                                                                    "Test case {}: Array value '{}' should not contain quotes",
                                                                    idx + 1,
                                                                    v
                                                                );
                                                            }
                                                        }
                                                    }
                                                    _ => {}
                                                }
                                            }
                                        }
                                    }
                                    ObjectKind::OCABundle(_) => {
                                        panic!("Test case {}: Unexpected OCABundle", idx + 1);
                                    }
                                }

                                println!("✓ Test case {} passed", idx + 1);
                            } else {
                                assert!(
                                    result.is_err(),
                                    "Test case {}: Expected error but got success",
                                    idx + 1
                                );
                                println!("✓ Test case {} correctly failed", idx + 1);
                            }
                        }
                        None => {
                            assert!(
                                !is_valid,
                                "Test case {}: Instruction should be invalid",
                                idx + 1
                            );
                        }
                    }
                }
                Err(e) => {
                    if is_valid {
                        panic!(
                            "Test case {}: Parsing should succeed but got error: {:?}",
                            idx + 1,
                            e
                        );
                    } else {
                        debug!("Test case {}: Correctly failed to parse: {:?}", idx + 1, e);
                        println!("✓ Test case {} correctly failed at parse stage", idx + 1);
                    }
                }
            }
        }
    }
}
