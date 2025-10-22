use crate::{
    ast::{Command, CommandType, NestedAttrType, NestedValue, OCAAst, ObjectKind, OverlayContent},
    errors::Error,
};
use indexmap::{indexmap, IndexMap, IndexSet};
use isolang::Language;
use log::debug;
use overlay_file::{ElementType, KeyType};
use regex::Regex;

type CaptureAttributes = IndexMap<String, NestedAttrType>;

/// Validates given commands against existing valid OCA AST
///
/// # Arguments
/// * `ast` - valid OCA AST
/// * `command` - Command to validate against AST
///
/// # Returns
/// * `Result<bool, Error>` - Result of validation
pub trait Validator {
    fn validate(&self, ast: &OCAAst, command: Command) -> Result<bool, Error>;
}

pub struct OCAValidator {}

impl Validator for OCAValidator {
    fn validate(&self, ast: &OCAAst, command: Command) -> Result<bool, Error> {
        let mut errors = Vec::new();
        let mut valid = true;
        match ast.version.as_str() {
            "2.0.0" | "1.0.0" => {
                let version_validator = validate(ast, command);
                if version_validator.is_err() {
                    valid = false;
                    errors.push(version_validator.err().unwrap());
                }
            }
            "" => {
                valid = false;
                errors.push(Error::MissingVersion());
            }
            _ => {
                valid = false;
                errors.push(Error::InvalidVersion(ast.version.to_string()));
            }
        }
        if valid {
            Ok(true)
        } else {
            Err(Error::Validation(errors))
        }
    }
}

fn validate(ast: &OCAAst, command: Command) -> Result<bool, Error> {
    // General rules for ocafile
    // Cannot remove if does not exist on stack
    // Cannot modify if does not exist on stack
    // Cannot add if already exists on stack
    // Attributes must have valid type
    let mut valid = true;
    let mut errors = Vec::new();
    match (&command.kind, &command.object_kind) {
        (CommandType::Add, ObjectKind::CaptureBase(_)) => {
            match rule_add_attr_if_not_exist(ast, command) {
                Ok(result) => {
                    if !result {
                        valid = result;
                    }
                }
                Err(error) => {
                    valid = false;
                    errors.push(error);
                }
            }
        }
        (CommandType::Remove, ObjectKind::CaptureBase(_)) => {
            match rule_remove_attr_if_exist(ast, command) {
                Ok(result) => {
                    if !result {
                        valid = result;
                    }
                }
                Err(error) => {
                    valid = false;
                    errors.push(error);
                }
            }
        }
        (CommandType::Add, ObjectKind::Overlay(_)) => {
            match validate_against_overlay_def(ast, &command) {
                Ok(result) => {
                    if !result {
                        valid = result;
                    }
                }
                Err(error) => {
                    valid = false;
                    errors.push(error);
                }
            }
        }

        _ => {
            // TODO: Add support for FROM, MODIFY with combination of different object kinds
        }
    }
    // CommandType::Modify => {
    //     match rule_modify_if_exist(ast, command) {
    //         Ok(result) => {
    //             if !result {
    //                 valid = result;
    //             }
    //         }
    //         Err(error) => {
    //             valid = false;
    //             errors.push(error);
    //         }
    //     }
    // }

    if valid {
        Ok(true)
    } else {
        Err(Error::Validation(errors))
    }
}

/// Check rules of overlay definition against the provided command
fn validate_against_overlay_def(ast: &OCAAst, command: &Command) -> Result<bool, Error> {
    let mut errors = Vec::new();

    if let ObjectKind::Overlay(overlay_content) = &command.object_kind {
        let cb_attrs = extract_attributes(ast);
        validate_overlay(overlay_content, &cb_attrs, &mut errors);
    }

    if errors.is_empty() {
        Ok(true)
    } else {
        Err(Error::Validation(errors))
    }
}

fn validate_overlay(
    overlay_content: &OverlayContent,
    capture_base_attrs: &CaptureAttributes,
    errors: &mut Vec<Error>,
) {
    let new_properties = IndexMap::new();
    let properties = overlay_content
        .properties
        .as_ref()
        .unwrap_or(&new_properties);

    let mut found_elements = IndexSet::new();


    // Validate property names against the overlay definition
    for (prop_name, prop_value) in properties.iter() {
        if let Some(element) = overlay_content
            .overlay_def
            .elements
            .iter()
            .find(|e| e.name == *prop_name)
            .or_else(|| {
                overlay_content
                    .overlay_def
                    .elements
                    .iter()
                    .find(|e| e.name.is_empty())
            })
        {
            println!("Element: {:?}", element);
            found_elements.insert(prop_name.clone());
            match is_valid_property_type(prop_value, &element.values) {
                Ok(true) => {
                    // If element type is AttrNames, validate attribute names against the capture base attributes
                    if element.keys == KeyType::AttrNames {
                        validate_attr_names(prop_value, capture_base_attrs, prop_name, errors);
                    }
                }
                Ok(false) => {
                    errors.push(Error::InvalidPropertyValue(format!(
                        "Property '{}' has an invalid value type",
                        prop_name
                    )));
                }
                Err(err_msg) => {
                    errors.push(Error::InvalidPropertyValue(format!(
                        "Property '{}': {}",
                        prop_name, err_msg
                    )));
                }
            }
        } else {
            errors.push(Error::InvalidProperty(format!(
                "Property '{}' is not allowed by the overlay definition",
                prop_name
            )));
        }

    }
    // Check for missing required properties
    for element in &overlay_content.overlay_def.elements {
        if !element.name.is_empty() && !found_elements.contains(&element.name) {
            errors.push(Error::MissingRequiredAttribute(element.name.clone()));
        }
    }
}

fn validate_attr_names(
    prop_value: &NestedValue,
    capture_base_attributes: &CaptureAttributes,
    prop_name: &str,
    errors: &mut Vec<Error>,
) {
    if let NestedValue::Object(attr_names) = prop_value {
        for attr_name in attr_names.keys() {
            if !capture_base_attributes.contains_key(attr_name) {
                errors.push(Error::InvalidProperty(format!(
                    "Attribute '{}' in '{}' is not present in the capture base",
                    attr_name, prop_name
                )));
            }
        }
    } else {
        errors.push(Error::InvalidPropertyValue(format!(
            "Property '{}' should be an object for AttrNames type",
            prop_name
        )));
    }
}

fn is_valid_property_type(
    value: &NestedValue,
    expected_type: &ElementType,
) -> Result<bool, String> {
    match (value, expected_type) {
        (NestedValue::Value(_), ElementType::Text) => Ok(true),
        (NestedValue::Value(s), ElementType::Lang) => {
            if is_valid_language_code(s) {
                Ok(true)
            } else {
                Err(format!("Invalid language code: '{}'", s))
            }
        }
        (NestedValue::Object(object), et) => {
            for (_, v) in object {
                is_valid_property_type(v, et)?;
            }
            Ok(true)
        }
        (NestedValue::Array(_), ElementType::Array(_)) => Ok(true),
        (NestedValue::Reference(_), ElementType::Ref) => Ok(true),
        (NestedValue::Value(_), ElementType::Binary) => Ok(true),
        (_, ElementType::Complex(types)) => {
            let mut any_valid = false;
            for t in types {
                match is_valid_property_type(value, t) {
                    Ok(true) => {
                        any_valid = true;
                        break;
                    }
                    _ => continue,
                }
            }
            if any_valid {
                Ok(true)
            } else {
                Err(format!(
                    "No valid value {:?} found for complex element: {:?}",
                    value, types
                ))
            }
        }
        _ => Err(format!(
            "Mismatched value type: expected {:?}, got {:?}",
            expected_type, value
        )),
    }
}

fn is_valid_language_code(code: &str) -> bool {
    // Check if it's a valid ISO 639-1 or ISO 639-3 code
    if Language::from_639_1(code).is_some() || Language::from_639_3(code).is_some() {
        return true;
    }

    // Check if it's a valid ISO 639-1 code with country code (e.g., "en-US")
    let lang_country_regex = Regex::new(r"^[a-z]{2}-[A-Z]{2}$").unwrap();
    if lang_country_regex.is_match(code) {
        let lang_code = &code[0..2];
        return Language::from_639_1(lang_code).is_some();
    }
    false
}
/// Check rule for remove command
/// Rule would be valid if attributes which commands tries to remove exist in the stack
///
/// # Arguments
/// * `ast` - valid OCA AST
/// * `command` - Command to validate against AST
///
/// # Returns
/// * `Result<bool, Error>` - Result of validation
fn rule_remove_attr_if_exist(ast: &OCAAst, command_to_validate: Command) -> Result<bool, Error> {
    let mut errors = Vec::new();

    let attributes = extract_attributes(ast);
    let properties = extract_properties(ast);

    let content = command_to_validate.object_kind.capture_content();

    println!("attributes: {:?}", attributes);
    println!("properties: {:?}", properties);

    match (
        content,
        content.as_ref().and_then(|c| c.attributes.as_ref()),
    ) {
        (Some(_content), Some(attrs_to_remove)) => {
            println!("attr to remove: {:?}", attrs_to_remove);
            let valid = attrs_to_remove
                .keys()
                .all(|key| attributes.contains_key(key));
            if !valid {
                errors.push(Error::InvalidOperation(
                    "Cannot remove attribute if does not exists".to_string(),
                ));
            }
        }
        (None, None) => (),
        (None, Some(_)) => (),
        (Some(_), None) => (),
    }

    match (
        content,
        content.as_ref().and_then(|c| c.properties.as_ref()),
    ) {
        (Some(_content), Some(props_to_remove)) => {
            let valid = props_to_remove
                .keys()
                .all(|key| properties.contains_key(key));
            if !valid {
                errors.push(Error::InvalidOperation(
                    "Cannot remove property if does not exists".to_string(),
                ));
                return Err(Error::Validation(errors));
            }
        }
        (None, None) => (),
        (None, Some(_)) => (),
        (Some(_), None) => (),
    }
    if errors.is_empty() {
        Ok(true)
    } else {
        Err(Error::Validation(errors))
    }
}

/// Check rule for add command
/// Rule would be valid if attributes which commands tries to add do not exist in the stack
///
/// # Arguments
/// * `ast` - valid OCA AST
/// * `command` - Command to validate against AST
///
/// # Returns
/// * `Result<bool, Error>` - Result of validation
fn rule_add_attr_if_not_exist(ast: &OCAAst, command_to_validate: Command) -> Result<bool, Error> {
    let mut errors = Vec::new();
    // Create a list of all attributes ADDed and REMOVEd via commands and check if what left covers needs of new command
    let default_attrs: IndexMap<String, NestedAttrType> = indexmap! {};

    let attributes = extract_attributes(ast);

    let content = command_to_validate.object_kind.capture_content();

    match content {
        Some(content) => {
            let attrs_to_add = content.attributes.clone().unwrap_or(default_attrs);
            debug!("attrs_to_add: {:?}", attrs_to_add);

            let existing_keys: Vec<_> = attrs_to_add
                .keys()
                .filter(|key| attributes.contains_key(*key))
                .collect();

            if !existing_keys.is_empty() {
                errors.push(Error::InvalidOperation(format!(
                    "Cannot add attribute if already exists: {:?}",
                    existing_keys
                )));
                Err(Error::Validation(errors))
            } else {
                Ok(true)
            }
        }
        None => {
            errors.push(Error::InvalidOperation(
                "No attribtues specify to be added".to_string(),
            ));
            Err(Error::Validation(errors))
        }
    }
}

fn extract_attributes(ast: &OCAAst) -> CaptureAttributes {
    let default_attrs: IndexMap<String, NestedAttrType> = indexmap! {};
    let mut attributes: CaptureAttributes = indexmap! {};
    for instruction in &ast.commands {
        match (instruction.kind.clone(), instruction.object_kind.clone()) {
            (CommandType::Remove, ObjectKind::CaptureBase(capture_content)) => {
                let attrs = capture_content
                    .attributes
                    .as_ref()
                    .unwrap_or(&default_attrs);
                attributes.retain(|key, _value| !attrs.contains_key(key));
            }
            (CommandType::Add, ObjectKind::CaptureBase(capture_content)) => {
                let attrs = capture_content
                    .attributes
                    .as_ref()
                    .unwrap_or(&default_attrs);
                attributes.extend(attrs.iter().map(|(k, v)| (k.clone(), v.clone())));
            }
            _ => {}
        }
    }
    attributes
}

fn extract_properties(ast: &OCAAst) -> IndexMap<String, NestedValue> {
    let default_attrs: IndexMap<String, NestedValue> = indexmap! {};
    let mut properties: IndexMap<String, NestedValue> = indexmap! {};
    for instruction in &ast.commands {
        match (instruction.kind.clone(), instruction.object_kind.clone()) {
            (CommandType::Remove, ObjectKind::CaptureBase(capture_content)) => {
                let props = capture_content
                    .properties
                    .as_ref()
                    .unwrap_or(&default_attrs);
                properties.retain(|key, _value| !props.contains_key(key));
            }
            (CommandType::Add, ObjectKind::CaptureBase(capture_content)) => {
                let props = capture_content
                    .properties
                    .as_ref()
                    .unwrap_or(&default_attrs);
                properties.extend(props.iter().map(|(k, v)| (k.clone(), v.clone())));
            }
            _ => {}
        }
    }
    properties
}

#[cfg(test)]
mod tests {
    use indexmap::indexmap;
    use overlay_file::KeyType;

    use super::*;
    use crate::ast::{
        AttributeType, CaptureContent, Command, CommandType, OCAAst, ObjectKind, RefValue,
    };

    #[test]
    fn test_rule_remove_if_exist() {
        let command = Command {
            kind: CommandType::Add,
            object_kind: ObjectKind::CaptureBase(CaptureContent {
                attributes: Some(indexmap! {
                    "name".to_string() => NestedAttrType::Value(AttributeType::Text),
                    "documentType".to_string() => NestedAttrType::Value(AttributeType::Text),
                    "photo".to_string() => NestedAttrType::Value(AttributeType::Binary),
                }),
                properties: Some(indexmap! {}),
            }),
        };

        let command2 = Command {
            kind: CommandType::Add,
            object_kind: ObjectKind::CaptureBase(CaptureContent {
                attributes: Some(indexmap! {
                    "issuer".to_string() => NestedAttrType::Value(AttributeType::Text),
                    "last_name".to_string() => NestedAttrType::Value(AttributeType::Binary),
                }),
                properties: Some(indexmap! {}),
            }),
        };

        let remove_command = Command {
            kind: CommandType::Remove,
            object_kind: ObjectKind::CaptureBase(CaptureContent {
                attributes: Some(indexmap! {
                    "name".to_string() => NestedAttrType::Null,
                    "documentType".to_string() => NestedAttrType::Null,
                }),
                properties: Some(indexmap! {}),
            }),
        };

        let add_command = Command {
            kind: CommandType::Add,
            object_kind: ObjectKind::CaptureBase(CaptureContent {
                attributes: Some(indexmap! {
                    "name".to_string() => NestedAttrType::Value(AttributeType::Text),
                }),
                properties: Some(indexmap! {}),
            }),
        };

        let valid_command = Command {
            kind: CommandType::Remove,
            object_kind: ObjectKind::CaptureBase(CaptureContent {
                attributes: Some(indexmap! {
                    "name".to_string() => NestedAttrType::Null,
                    "issuer".to_string() => NestedAttrType::Null,
                }),
                properties: Some(indexmap! {}),
            }),
        };

        let invalid_command = Command {
            kind: CommandType::Remove,
            object_kind: ObjectKind::CaptureBase(CaptureContent {
                attributes: Some(indexmap! {
                    "documentType".to_string() => NestedAttrType::Null,
                }),
                properties: Some(indexmap! {}),
            }),
        };

        let mut ocaast = OCAAst::new();
        ocaast.commands.push(command);
        ocaast.commands.push(command2);
        ocaast.commands.push(remove_command);
        ocaast.commands.push(add_command);
        let mut result = rule_remove_attr_if_exist(&ocaast, valid_command.clone());
        assert!(result.is_ok());
        ocaast.commands.push(invalid_command.clone());
        result = rule_remove_attr_if_exist(&ocaast, invalid_command);
        assert!(result.is_err());
    }

    #[test]
    fn test_rule_add_if_not_exist() {
        let command = Command {
            kind: CommandType::Add,
            object_kind: ObjectKind::CaptureBase(CaptureContent {
                attributes: Some(indexmap! {
                    "name".to_string() => NestedAttrType::Value(AttributeType::Text),
                    "documentType".to_string() => NestedAttrType::Value(AttributeType::Text),
                    "photo".to_string() => NestedAttrType::Value(AttributeType::Binary),
                }),
                properties: Some(indexmap! {}),
            }),
        };

        let command2 = Command {
            kind: CommandType::Add,
            object_kind: ObjectKind::CaptureBase(CaptureContent {
                attributes: Some(indexmap! {
                    "issuer".to_string() => NestedAttrType::Value(AttributeType::Text),
                    "last_name".to_string() => NestedAttrType::Value(AttributeType::Binary),
                }),
                properties: Some(indexmap! {}),
            }),
        };

        let valid_command = Command {
            kind: CommandType::Add,
            object_kind: ObjectKind::CaptureBase(CaptureContent {
                attributes: Some(indexmap! {
                    "first_name".to_string() => NestedAttrType::Value(AttributeType::Text),
                    "address".to_string() => NestedAttrType::Value(AttributeType::Text),
                }),
                properties: Some(indexmap! {}),
            }),
        };

        let invalid_command = Command {
            kind: CommandType::Add,
            object_kind: ObjectKind::CaptureBase(CaptureContent {
                attributes: Some(indexmap! {
                    "name".to_string() => NestedAttrType::Value(AttributeType::Text),
                    "phone".to_string() => NestedAttrType::Value(AttributeType::Text),
                }),
                properties: Some(indexmap! {}),
            }),
        };

        let mut ocaast = OCAAst::new();
        ocaast.commands.push(command);
        ocaast.commands.push(command2);
        let mut result = rule_add_attr_if_not_exist(&ocaast, valid_command.clone());
        assert!(result.is_ok());
        ocaast.commands.push(invalid_command.clone());
        result = rule_add_attr_if_not_exist(&ocaast, invalid_command.clone());
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_overlay_against_definition() {
        use overlay_file::{ElementType, OverlayDef, OverlayElementDef};

        let label_overlay_def = OverlayDef {
            name: "Label".to_string(),
            elements: vec![
                OverlayElementDef {
                    name: "attr_labels".to_string(),
                    values: ElementType::Text,
                    keys: KeyType::AttrNames,
                },
                OverlayElementDef {
                    name: "language".to_string(),
                    values: ElementType::Lang,
                    keys: KeyType::None,
                },
            ],
            namespace: Some("hcf".to_string()),
            version: "2.0.0".to_string(),
        };

        let meta_overlay_def = OverlayDef {
            name: "meta".to_string(),
            elements: vec![
                OverlayElementDef {
                    name: "language".to_string(),
                    values: ElementType::Lang,
                    keys: KeyType::None,
                },
                OverlayElementDef {
                    name: "description".to_string(),
                    values: ElementType::Text,
                    keys: KeyType::None,
                },
                OverlayElementDef {
                    name: "name".to_string(),
                    values: ElementType::Text,
                    keys: KeyType::None,
                },
                // Empty name means that any name is allowed with specific types
                OverlayElementDef {
                    name: "".to_string(),
                    values: ElementType::Text,
                    keys: KeyType::None,
                },
            ],
            namespace: Some("hcf".to_string()),
            version: "2.0.0".to_string(),
        };

        let entry_overlay_def = OverlayDef {
            name: "Entry".to_string(),
            elements: vec![
                OverlayElementDef {
                    name: "attribute_entries".to_string(),
                    values: ElementType::Complex(vec![ElementType::Ref, ElementType::Array(None)]),
                    keys: KeyType::AttrNames,
                },
                OverlayElementDef {
                    name: "language".to_string(),
                    values: ElementType::Lang,
                    keys: KeyType::None,
                },
            ],
            namespace: Some("hcf".to_string()),
            version: "2.0.0".to_string(),
        };

        let entry_code_overlay_def = OverlayDef {
            name: "Entry_Code".to_string(),
            elements: vec![OverlayElementDef {
                name: "attribute_entry_codes".to_string(),
                values: ElementType::Complex(vec![ElementType::Ref, ElementType::Array(None)]),
                keys: KeyType::AttrNames,
            }],
            namespace: Some("hcf".to_string()),
            version: "2.0.0".to_string(),
        };

        let capture_base = Command {
            kind: CommandType::Add,
            object_kind: ObjectKind::CaptureBase(CaptureContent {
                attributes: Some(indexmap! {
                    "first_name".to_string() => NestedAttrType::Value(AttributeType::Text),
                    "last_name".to_string() => NestedAttrType::Value(AttributeType::Text),
                    "address".to_string() => NestedAttrType::Value(AttributeType::Text),
                    "sex".to_string() => NestedAttrType::Value(AttributeType::Text),
                }),
                properties: Some(indexmap! {}),
            }),
        };

        let mut ocaast = OCAAst::new();
        ocaast.commands.push(capture_base.clone());

        // Test case 1: Valid overlay
        let valid_overlay = Command {
            kind: CommandType::Add,
            object_kind: ObjectKind::Overlay(OverlayContent {
                properties: Some(indexmap! {
                    "attr_labels".to_string() => NestedValue::Object(indexmap! {
                        "first_name".to_string() => NestedValue::Value("First name".to_string()),
                        "last_name".to_string() => NestedValue::Value("Last name".to_string()),
                    }),
                    "language".to_string() => NestedValue::Value("en-UK".to_string()),
                }),
                overlay_def: label_overlay_def.clone(),
            }),
        };

        let result = validate_against_overlay_def(&ocaast, &valid_overlay);
        match result {
            Ok(_) => assert!(true, "Valid overlay should pass validation"),
            Err(Error::Validation(errors)) => {
                assert!(false, "Unexpected validation errors: {:?}", errors);
            }
            Err(e) => assert!(false, "Unexpected error: {:?}", e),
        }

        ocaast.commands.push(valid_overlay.clone());

        // Test case 2: Invalid overlay (missing required field and wrong types )
        let invalid_overlay_missing_field = Command {
            kind: CommandType::Add,
            object_kind: ObjectKind::Overlay(OverlayContent {
                overlay_def: label_overlay_def.clone(),
                properties: Some(indexmap! {
                    "attr_labels".to_string() => NestedValue::Object(indexmap! {
                        "address".to_string() => NestedValue::Reference(RefValue::Name("passport".to_string())),
                    }),
                    "lang".to_string() => NestedValue::Value("pl".to_string()),
                }),
            }),
        };

        let result = validate_against_overlay_def(&ocaast, &invalid_overlay_missing_field);
        match result {
            Ok(_) => assert!(
                false,
                "Overlay with missing required field should fail validation"
            ),
            Err(Error::Validation(errors)) => {
                assert_eq!(errors.len(), 3);
                assert_eq!(
                    errors[2].to_string(),
                    "Missing required attribute in Overlay: language"
                );
                assert_eq!(
                    errors[0].to_string(),
                    "Invalid Property Value: Property 'attr_labels': Mismatched value type: expected Text, got Reference(Name(\"passport\"))"
                );
                assert_eq!(
                    errors[1].to_string(),
                    "Invalid Property: Property 'lang' is not allowed by the overlay definition"
                );
            }
            Err(e) => assert!(false, "Unexpected error: {:?}", e),
        }

        // Test case 3: validate custom fileds and check their values
        let meta_overlay = Command {
            kind: CommandType::Add,
            object_kind: ObjectKind::Overlay(OverlayContent {
                overlay_def: meta_overlay_def.clone(),
                properties: Some(indexmap! {
                    "language".to_string() => NestedValue::Value("en-UK".to_string()),
                    "description".to_string() => NestedValue::Value("Some description".to_string()),
                    "name".to_string() => NestedValue::Value("Some name".to_string()),
                    "custom1".to_string() => NestedValue::Value("Custom value 1".to_string()),
                    "custom2".to_string() => NestedValue::Array(vec![NestedValue::Value("Custom value 2".to_string()), NestedValue::Value("Custom value 3".to_string())]),
                }),
            }),
        };

        let result = validate_against_overlay_def(&ocaast, &meta_overlay);
        match result {
            Ok(_) => assert!(true, "Meta overlay should pass validation"),
            Err(Error::Validation(errors)) => {
                assert_eq!(
                    errors[0].to_string(),
                    "Invalid Property Value: Property 'custom2': Mismatched value type: expected Text, got Array([Value(\"Custom value 2\"), Value(\"Custom value 3\")])"
                );
            }
            Err(e) => assert!(false, "Unexpected error: {:?}", e),
        }

        // Test case 4: validate complex types
        let entry_code_overlay = Command {
            kind: CommandType::Add,
            object_kind: ObjectKind::Overlay(OverlayContent {
                overlay_def: entry_code_overlay_def.clone(),
                properties: Some(indexmap! {
                    "attribute_entry_codes".to_string() => NestedValue::Object(indexmap! {
                        "sex".to_string() => NestedValue::Array(vec![NestedValue::Value("Male".to_string()), NestedValue::Value("Female".to_string())]),
                    }),
                }),
            }),
        };

        let result = validate_against_overlay_def(&ocaast, &entry_code_overlay);
        match result {
            Ok(_) => assert!(true, "Entry code overlay should pass validation"),
            Err(e) => assert!(false, "Unexpected error: {:?}", e),
        }

        // Test case 5: validate complex types part 2 - refs
        let entry_code_overlay = Command {
            kind: CommandType::Add,
            object_kind: ObjectKind::Overlay(OverlayContent {
                overlay_def: entry_code_overlay_def.clone(),
                properties: Some(indexmap! {
                    "attribute_entry_codes".to_string() => NestedValue::Object(indexmap! {
                        "sex".to_string() => NestedValue::Reference(RefValue::Name("nazwa".to_string())),
                    }),
                }),
            }),
        };

        let result = validate_against_overlay_def(&ocaast, &entry_code_overlay);
        match result {
            Ok(_) => assert!(true, "Entry code overlay should pass validation"),
            Err(e) => assert!(false, "Unexpected error: {:?}", e),
        }
        // Test case 5: validate complex types: Array
        let entry_overlay = Command {
            kind: CommandType::Add,
            object_kind: ObjectKind::Overlay(OverlayContent {
                overlay_def: entry_overlay_def.clone(),
                properties: Some(indexmap! {
                    "attribute_entries".to_string() => NestedValue::Object(indexmap! {
                        "sex".to_string() => NestedValue::Array(vec![NestedValue::Reference(RefValue::Name("entry_code1".to_string())), NestedValue::Reference(RefValue::Name("entry_code2".to_string()))]),
                    }),
                    "language".to_string() => NestedValue::Value("en-UK".to_string()),
                }),
            }),
        };

        let result = validate_against_overlay_def(&ocaast, &entry_overlay);
        match result {
            Ok(_) => assert!(true, "Entry overlay should pass validation"),
            Err(Error::Validation(errors)) => {
                assert_eq!(errors.len(), 1);
                assert_eq!(
                    errors[0].to_string(),
                    "Invalid Property Value: Property 'attribute_entries': Mismatched value type: expected Array([Reference(Name(\"entry_code1\")), Reference(Name(\"entry_code2\"))]), got Object({\"entry1\": Array([Reference(Name(\"entry_code1\")), Reference(Name(\"entry_code2\"))])})"
                );
            }
            Err(e) => assert!(false, "Unexpected error: {:?}", e),
        }
    }
}
