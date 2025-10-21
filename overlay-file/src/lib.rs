pub mod error;
pub mod overlay_registry;
pub mod validator;

use self::error::ParseError;
use self::validator::OverlayfileValidator;
use log::debug;
use pest::Parser;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(pest_derive::Parser)]
#[grammar = "overlay.pest"]
struct OverlayParser;

pub type Pair<'a> = pest::iterators::Pair<'a, Rule>;

#[derive(Debug, Clone)]
pub struct OverlayFile {
    pub overlays_def: Vec<OverlayDef>,
    pub meta: HashMap<String, String>,
}

impl OverlayFile {
    pub fn new() -> Self {
        OverlayFile {
            overlays_def: Vec::new(),
            meta: HashMap::new(),
        }
    }
}

impl Default for OverlayFile {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Hash, Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct OverlayDef {
    pub namespace: Option<String>,
    pub name: String,
    pub version: String,
    /// enahnce attributes from capture base with semantic information
    pub elements: Vec<OverlayElementDef>,
}

impl Default for OverlayDef {
    fn default() -> Self {
        OverlayDef {
            namespace: None,
            name: String::new(),
            version: "1.0.0".to_string(),
            elements: Vec::new(),
        }
    }
}

impl OverlayDef {
    /// Return elements that are defined as capture base attributes
    pub fn get_attr_elements(&self) -> Vec<String> {
        self.elements
            .iter()
            .filter(|el| el.keys == KeyType::AttrNames)
            .map(|el| el.name.clone())
            .collect()
    }
    /// Return ordered list of element names as they should appear in serialization
    pub fn get_ordered_element_names(&self) -> Vec<String> {
        self.elements.iter().map(|el| el.name.clone()).collect()
    }
    pub fn get_full_name(&self) -> String {
        format!("{}/{}", self.name, self.version)
    }
    pub fn get_name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct KeyPair {
    pub name: String,
    pub kind: ElementType,
}

#[derive(Hash, Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct OverlayElementDef {
    pub name: String,
    pub keys: KeyType,
    pub values: ElementType,
}

#[derive(Hash, Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
/// Type of the keys allowed for Overlay Object
pub enum KeyType {
    /// Keys names needs to correspond to the attribute names from Capture Base
    AttrNames,
    /// Keys are any arbitrary string
    Text,
}

#[derive(Hash, Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
/// Type of the values allowed for Overlay Object
pub enum ElementType {
    Object,
    Array(Option<Vec<ConstraintKind>>),
    Binary,
    Text,
    /// Language code according to ISO 639-1 or ISO 639-3
    Lang,
    /// Reference in form of SAID to another object
    Ref,
}

#[derive(Hash, Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum ConstraintKind {
    /// Only defined values are allowed
    Closed(Option<Vec<String>>),
    /// Defined values are allowed and any other value
    Open(Option<Vec<String>>),
    /// Any value is allowed
    None,
}

#[derive(Debug)]
pub struct FieldConstraint {
    pub name: String,
    pub required: bool,
}

pub fn parse_from_string(unparsed_file: String) -> Result<OverlayFile, ParseError> {
    let file = OverlayParser::parse(Rule::file, &unparsed_file)
        .map_err(|e| {
            let (line_number, column_number) = match e.line_col {
                pest::error::LineColLocation::Pos((line, column)) => (line, column),
                pest::error::LineColLocation::Span((line, column), _) => (line, column),
            };
            ParseError::GrammarError {
                line_number,
                column_number,
                raw_line: e.line().to_string(),
                message: e.variant.to_string(),
            }
        })?
        .next()
        .unwrap();

    let mut overlays_file = OverlayFile::new();

    // TODO let validator = ... create validator for definition syntax

    for line in file.into_inner() {
        if let Rule::EOI = line.as_rule() {
            continue;
        }
        if let Rule::comment = line.as_rule() {
            continue;
        }
        if let Rule::meta_comment = line.as_rule() {
            let mut key = "".to_string();
            let mut value = "".to_string();
            for attr in line.into_inner() {
                match attr.as_rule() {
                    Rule::meta_attr_key => {
                        key = attr.as_str().to_string();
                    }
                    Rule::meta_attr_value => {
                        value = attr.as_str().to_string();
                    }
                    _ => {
                        return Err(ParseError::MetaError(attr.as_str().to_string()));
                    }
                }
            }
            if key.is_empty() {
                return Err(ParseError::MetaError("key is empty".to_string()));
            }
            if value.is_empty() {
                return Err(ParseError::MetaError("value is empty".to_string()));
            }
            overlays_file.meta.insert(key, value);
            continue;
        }
        if let Rule::overlay_block = line.as_rule() {
            let mut overlay_def = OverlayDef {
                namespace: None,
                name: "".to_string(),
                version: "".to_string(),
                elements: Vec::new(),
            };
            for attr in line.into_inner() {
                match attr.as_rule() {
                    Rule::overlay_name => {
                        let (namespace, name): (Option<String>, String) =
                            match attr.as_str().split_once(':') {
                                Some((ns, n)) if !ns.is_empty() => {
                                    (Some(ns.to_string()), n.to_string())
                                }
                                Some((_, n)) => (None, n.to_string()),
                                None => (None, attr.as_str().to_string()),
                            };
                        overlay_def.namespace = namespace;
                        overlay_def.name = name;
                    }
                    Rule::version => {
                        overlay_def.version = attr.as_str().to_string();
                    }
                    Rule::overlay_object => {
                        let mut name: Option<String> = None;
                        let mut keys: Option<KeyType> = None;
                        let mut values: Option<ElementType> = None;
                        debug!("Parsing overlay object: {:?}", attr);

                        for e in attr.into_inner() {
                            match e.as_rule() {
                                Rule::overlay_object_header => {
                                    e.into_inner().for_each(|header| match header.as_rule() {
                                        Rule::attr_name => {
                                            name = Some(header.as_str().to_string());
                                        }
                                        _ => {
                                            panic!(
                                                "Missing name for an object: {:?}",
                                                header.as_rule()
                                            )
                                        }
                                    });
                                }
                                Rule::overlay_object_body => {
                                    for key in e.into_inner() {
                                        match key.as_rule() {
                                            Rule::key_type => match key.into_inner().next() {
                                                Some(k) => match k.as_rule() {
                                                    Rule::ATTR_NAMES_TYPE => {
                                                        keys = Some(KeyType::AttrNames)
                                                    }
                                                    Rule::TEXT_TYPE => keys = Some(KeyType::Text),
                                                    Rule::ARRAY_TYPE => todo!(),
                                                    _ => continue,
                                                },
                                                None => {
                                                    return Err(ParseError::MetaError(
                                                        "key type is empty".to_string(),
                                                    ));
                                                }
                                            },
                                            Rule::value_type => match key.into_inner().next() {
                                                Some(k) => match k.as_rule() {
                                                    Rule::object_type => {
                                                        values = Some(ElementType::Object);
                                                    }
                                                    Rule::ARRAY_TYPE => {
                                                        todo!()
                                                    }
                                                    Rule::TEXT_TYPE => {
                                                        values = Some(ElementType::Text);
                                                    }
                                                    Rule::REF_TYPE => {
                                                        values = Some(ElementType::Ref);
                                                    }
                                                    _ => continue,
                                                },
                                                None => {
                                                    return Err(ParseError::MetaError(
                                                        "value type is empty".to_string(),
                                                    ));
                                                }
                                            },
                                            _ => continue,
                                        };
                                    }
                                }
                                _ => {}
                            }
                        }
                        let overlay_element = OverlayElementDef {
                            name: name.clone().unwrap_or_default(),
                            keys: keys.clone().unwrap_or(KeyType::Text),
                            values: values.clone().unwrap_or(ElementType::Object),
                        };
                        overlay_def.elements.push(overlay_element);
                    }
                    Rule::overlay_attributes => {
                        for a in attr.into_inner() {
                            if a.as_rule() == Rule::key_pair {
                                let mut key_pair = KeyPair {
                                    name: "".to_string(),
                                    kind: ElementType::Text,
                                };
                                a.into_inner().for_each(|kp| match kp.as_rule() {
                                    Rule::attr_name => {
                                        key_pair.name = kp.as_str().to_string();
                                    }
                                    Rule::attr_value_type => match kp.as_str() {
                                        "Text" => key_pair.kind = ElementType::Text,
                                        "Ref" => key_pair.kind = ElementType::Ref,
                                        "Binary" => key_pair.kind = ElementType::Binary,
                                        "Array" => key_pair.kind = ElementType::Array(None),
                                        "Lang" => key_pair.kind = ElementType::Lang,
                                        _ => {}
                                    },
                                    _ => {
                                        panic!(
                                            "Incorrect key pair for attribute: {:?}",
                                            kp.as_rule()
                                        )
                                    }
                                });
                                let element = OverlayElementDef {
                                    name: key_pair.name.clone(),
                                    keys: KeyType::Text,
                                    values: key_pair.kind.clone(),
                                };
                                overlay_def.elements.push(element);
                            }
                        }
                    }
                    Rule::overlay_array => {
                        let mut name: Option<String> = None;
                        let mut values: Option<ElementType> = None;
                        debug!("Parsing overlay array: {:?}", attr);
                        for a in attr.into_inner() {
                            match a.as_rule() {
                                Rule::attr_name => {
                                    name = Some(a.as_str().to_string());
                                }
                                Rule::value_type => match a.into_inner().next() {
                                    Some(v) => match v.as_rule() {
                                        Rule::ARRAY_TYPE => {
                                            values = Some(ElementType::Array(None));
                                        }
                                        _ => continue,
                                    },
                                    None => {
                                        return Err(ParseError::MetaError(
                                            "value type is empty".to_string(),
                                        ));
                                    }
                                },
                                _ => continue,
                            }
                        }
                        let overlay_element = OverlayElementDef {
                            name: name.clone().unwrap_or_default(),
                            keys: KeyType::Text,
                            values: values.clone().unwrap_or(ElementType::Object),
                        };
                        overlay_def.elements.push(overlay_element);
                    }
                    _ => {}
                }
            }
            overlays_file.overlays_def.push(overlay_def);
            continue;
        }
        if let Rule::empty_line = line.as_rule() {
            continue;
        }
    }

    let overlays_file = OverlayFile {
        overlays_def: overlays_file.overlays_def,
        meta: overlays_file.meta,
    };

    // Validate the parsed OverlayFile
    match OverlayfileValidator::validate(&overlays_file) {
        Ok(_) => Ok(overlays_file),
        Err(validation_errors) => Err(ParseError::ValidationError(validation_errors)),
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_overlay_name() {
        let input = r#"
ADD OVERLAY ReferenceValues
  VERSION 1.0.0
  ADD OBJECT attribute_reference_values
    WITH KEYS attr-names
    WITH VALUES Text
"#;
        let result = parse_from_string(input.to_string()).unwrap();
        let overlay = result.overlays_def.first().unwrap();
        assert_eq!(overlay.name, "ReferenceValues");
        assert_eq!(overlay.namespace, None);

        let input = r#"
ADD OVERLAY :ReferenceValues
  VERSION 1.0.0
  ADD OBJECT attribute_reference_values
    WITH KEYS attr-names
    WITH VALUES Text"#;
        let result = parse_from_string(input.to_string()).unwrap();
        let overlay = result.overlays_def.first().unwrap();
        assert_eq!(overlay.name, "ReferenceValues");
        assert_eq!(overlay.namespace, None);

        let input = r#"
ADD OVERLAY hcf:ReferenceValues
  VERSION 1.0.0
  ADD OBJECT attribute_reference_values
    WITH KEYS attr-names
    WITH VALUES Text
"#;
        let result = parse_from_string(input.to_string()).unwrap();
        let overlay = result.overlays_def.first().unwrap();
        assert_eq!(overlay.name, "ReferenceValues");
        assert_eq!(overlay.namespace.clone().unwrap(), "hcf".to_string());
    }
    #[test]
    fn test_parsing_overlayfile() {
        let input = r#"
--name=HCF
# Create new overlay
ADD OVERLAY ReferenceValues
  VERSION 1.0.1
  ADD OBJECT attribute_reference_values
    WITH KEYS attr-names
    WITH VALUES OBJECT
      WITH KEYS Text
      WITH VALUES Text

ADD OVERLAY hcf:Information
  VERSION 1.2.2
  ADD ATTRIBUTES language=Lang
  ADD OBJECT attr
    WITH KEYS Text
    WITH VALUES Text
  ADD OBJECT attribute_information
    WITH KEYS attr-names
    WITH VALUES Text

ADD OVERLAY hcf:Meta
  VERSION 1.2.2
  ADD ATTRIBUTES name=Text description=Text photo=Binary language=Lang
"#;

        let result = parse_from_string(input.to_string()).unwrap();

        let ref_overlay = result.overlays_def.first().unwrap();
        let information = result.overlays_def.get(1).unwrap();
        let meta = result.overlays_def.last().unwrap();
        assert_eq!(ref_overlay.name, "ReferenceValues");
        assert_eq!(ref_overlay.version, "1.0.1");
        assert_eq!(ref_overlay.namespace, None);
        assert_eq!(ref_overlay.elements.len(), 1);
        assert_eq!(ref_overlay.elements[0].name, "attribute_reference_values");
        assert_eq!(ref_overlay.elements[0].keys, KeyType::AttrNames);
        assert_eq!(ref_overlay.elements[0].values, ElementType::Object);

        assert_eq!(information.version, "1.2.2");
        assert_eq!(information.namespace.clone().unwrap(), "hcf".to_string());
        assert_eq!(information.elements[0].name, "language");
        assert_eq!(information.elements[0].values, ElementType::Lang);
        assert_eq!(information.elements[2].name, "attribute_information");
        assert_eq!(information.elements.len(), 3);
        assert_eq!(information.name, "Information");

        assert_eq!(meta.version, "1.2.2");
        assert_eq!(meta.elements.len(), 4);
        assert_eq!(
            meta.elements
                .iter()
                .find(|e| e.name == "name")
                .unwrap()
                .values,
            ElementType::Text
        );
        assert_eq!(meta.elements.last().unwrap().name, "language");
        assert_eq!(meta.elements.last().unwrap().values, ElementType::Lang);

        assert_eq!(result.overlays_def.len(), 3);
    }
}
