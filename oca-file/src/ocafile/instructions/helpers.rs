use std::str::FromStr;

use crate::ocafile::{error::ExtractingAttributeError, Pair, Rule};
use indexmap::IndexMap;
use log::debug;
use oca_ast::ast::{
    recursive_attributes::{AttributeTypeResult, NestedAttrTypeFrame},
    AttributeType, NestedAttrType, NestedValue, RefValue,
};
use recursion::ExpandableExt;
use said::SelfAddressingIdentifier;

fn extract_attr_type(input: Pair) -> Result<NestedAttrType, ExtractingAttributeError> {
    let res = AttributeTypeResult::expand_frames(input, |seed| match seed.as_rule() {
        Rule::array_attr_type => match seed.into_inner().next() {
            Some(next) => NestedAttrTypeFrame::Array(next).into(),
            None => {
                ExtractingAttributeError::Unexpected("Missing attribute type".to_string()).into()
            }
        },
        Rule::alias => NestedAttrTypeFrame::Reference(oca_ast::ast::RefValue::Name(
            seed.as_str().to_string(),
        ))
        .into(),
        Rule::said => match SelfAddressingIdentifier::from_str(seed.as_str()) {
            Ok(said) => NestedAttrTypeFrame::Reference(RefValue::Said(said)).into(),
            Err(e) => ExtractingAttributeError::SaidError(e).into(),
        },
        Rule::base_attr_type => {
            let seed_str = seed.as_span().as_str();
            match AttributeType::from_str(seed_str) {
                Ok(attr_type) => NestedAttrTypeFrame::Value(attr_type).into(),
                Err(_) => ExtractingAttributeError::Unexpected(format!(
                    "Unknown attribute type {}",
                    seed_str
                ))
                .into(),
            }
        }
        rule => {
            ExtractingAttributeError::Unexpected(format!("Unexpected pest rule: {:?}", rule)).into()
        }
    });
    res.value()
}

pub fn extract_attribute(
    attr_pair: Pair,
) -> Result<(String, NestedAttrType), ExtractingAttributeError> {
    let mut attr_name = String::new();
    let mut attr_type = NestedAttrType::Value(AttributeType::Text);

    debug!("Extracting the attribute type from: {:?}", attr_pair);
    for item in attr_pair.into_inner() {
        match item.as_rule() {
            Rule::attr_key => {
                debug!("Extracting attribute key {:?}", attr_name);
                attr_name = item.as_str().to_string();
            }
            Rule::attr_type => {
                debug!("Attribute type to parse: {:?}", item);
                let item_field_label = item.as_span().as_str();
                let mut inner = item.into_inner();
                let inner_pair =
                    inner
                        .next()
                        .ok_or(ExtractingAttributeError::Unexpected(format!(
                            "Missing attribute type for {} field",
                            item_field_label
                        )))?;
                attr_type = extract_attr_type(inner_pair)?;
            }
            rule => {
                return Err(ExtractingAttributeError::Unexpected(format!(
                    "Unexpected pest rule: {:?}",
                    rule
                )))
            }
        }
    }
    Ok((attr_name, attr_type))
}

/// Extract attributes key pairs for ADD and MODIFY command
pub fn extract_attribute_key_pairs(attr_pair: Pair) -> Option<(String, NestedValue)> {
    let mut key = String::new();
    let mut value = NestedValue::Value(String::new());

    debug!("Extracting the attribute from: {:?}", attr_pair);
    for item in attr_pair.into_inner() {
        match item.as_rule() {
            Rule::attr_key => {
                key = item.as_str().to_string();
                debug!("Extracting attribute key {:?}", key);
            }
            Rule::key_value => {
                if let Some(nested_item) = item.clone().into_inner().next() {
                    match nested_item.as_rule() {
                        Rule::string => {
                            value = NestedValue::Value(
                                nested_item
                                    .clone()
                                    .into_inner()
                                    .next_back()
                                    .unwrap()
                                    .as_span()
                                    .as_str()
                                    .to_string(),
                            );
                        }
                        _ => {
                            value = NestedValue::Value(item.as_str().to_string());
                        }
                    }
                }
            }
            Rule::json_object => {
                value = extract_json_object(item);
            }
            _ => {
                panic!("Invalid attribute in {:?}", item.as_rule());
            }
        }
    }
    Some((key, value))
}

pub fn extract_json_object(object: Pair) -> NestedValue {
    let mut json_object = IndexMap::new();
    for item in object.into_inner() {
        match item.as_rule() {
            Rule::json_pair => {
                let mut key = String::new();
                let mut value = NestedValue::Value(String::new());
                for el in item.clone().into_inner() {
                    match el.as_rule() {
                        Rule::json_key => {
                            key = el
                                .clone()
                                .into_inner()
                                .next_back()
                                .unwrap()
                                .into_inner()
                                .next_back()
                                .unwrap()
                                .as_span()
                                .as_str()
                                .to_lowercase();
                        }
                        Rule::json_value => {
                            if let Some(nested_item) = el.clone().into_inner().next() {
                                match nested_item.as_rule() {
                                    Rule::string => {
                                        value = NestedValue::Value(
                                            nested_item
                                                .clone()
                                                .into_inner()
                                                .next_back()
                                                .unwrap()
                                                .as_span()
                                                .as_str()
                                                .to_string(),
                                        );
                                    }
                                    Rule::json_object => {
                                        value = extract_json_object(nested_item);
                                    }
                                    _ => {
                                        panic!("Invalid json value in {:?}", nested_item.as_rule());
                                    }
                                }
                            }
                        }
                        _ => {
                            panic!("Invalid json pair in {:?}", el.as_rule());
                        }
                    }
                }
                json_object.insert(key, value);
            }
            _ => {
                panic!("Invalid json object in {:?}", item.as_rule());
            }
        }
    }
    NestedValue::Object(json_object)
}

// TODO remove it
pub fn extract_attributes_key_paris(object: Pair) -> Option<IndexMap<String, NestedValue>> {
    let mut attributes: IndexMap<String, NestedValue> = IndexMap::new();

    debug!("Extracting content of the attributes: {:?}", object);
    for attr in object.into_inner() {
        debug!("Inside the object: {:?}", attr);
        match attr.as_rule() {
            // Rule::attr_key_pairs => {
            //     for attr in attr.into_inner() {
            //         debug!("Parsing attribute {:?}", attr);
            //         if let Some((key, value)) = extract_attribute_key_pairs(attr) {
            //             debug!("Parsed attribute: {:?} = {:?}", key, value);
            //             // TODO find out how to parse nested objects
            //             attributes.insert(key, value);
            //         } else {
            //             debug!("Skipping attribute");
            //         }
            //     }
            // }
            Rule::attr_key => {
                debug!("Parsing attribute key {:?}", attr);
                let attr_key = attr.as_str().to_string();
                attributes.insert(attr_key, NestedValue::Value("".to_string()));
            }

            _ => {
                debug!(
                    "Unexpected token: Skipping invalid attribute in instruction {:?}",
                    attr.as_rule()
                );
            }
        }
    }

    Some(attributes)
}

// TODO remove it
/// Extract properties key pairs for any command
pub fn extract_properites_key_pairs(object: Pair) -> Option<IndexMap<String, NestedValue>> {
    let mut properties: IndexMap<String, NestedValue> = IndexMap::new();

    debug!("Extracting properties from the object: {:?}", object);
    for attr in object.into_inner() {
        debug!("Inside the object: {:?}", attr);
        match attr.as_rule() {
            // Rule::prop_key_pairs => {
            //     for prop in attr.into_inner() {
            //         debug!("Parsing property {:?}", prop);
            //         if let Some((key, value)) = extract_attribute_key_pairs(prop) {
            //             debug!("Parsed property: {:?} = {:?}", key, value);
            //             properties.insert(key, value);
            //         } else {
            //             debug!("Skipping property");
            //         }
            //     }
            // }
            Rule::lang => {
                debug!("Parsing language: {:?}", attr.as_str());
                properties.insert(
                    "lang".to_string(),
                    NestedValue::Value(attr.as_str().to_string()),
                );
            }
            Rule::alias => {
                debug!("Parsing target alias: {:?}", attr.as_str());
                properties.insert(
                    "target".to_string(),
                    NestedValue::Reference(RefValue::Name(attr.as_str().to_string())),
                );
            }
            Rule::said => {
                debug!("Parsing target said: {:?}", attr.as_str());
                if let Ok(said) = SelfAddressingIdentifier::from_str(attr.as_str()) {
                    properties.insert(
                        "target".to_string(),
                        NestedValue::Reference(RefValue::Said(said)),
                    );
                }
            }
            _ => {
                debug!(
                    "Unexpected token: Invalid attribute in instruction {:?}",
                    attr.as_rule()
                );
            }
        }
    }
    Some(properties)
}

// TODO Remove it
// Extract content from any instruction related to any overlay
// pub fn extract_content(object: Pair) -> Content {
//     let properties: Option<IndexMap<String, NestedValue>> =
//         extract_properites_key_pairs(object.clone());
//
//     Content {
//         properties,
//         version: None,
//     }
// }
