pub mod error;
pub mod overlay_registry;
use self::error::ParseError;
use pest::Parser;
use std::collections::HashMap;

#[derive(pest_derive::Parser)]
#[grammar = "overlay.pest"]
struct OverlayParser;

pub type Pair<'a> = pest::iterators::Pair<'a, Rule>;


#[derive(Debug)]
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

#[derive(Debug, Clone)]
pub struct OverlayDef {
    pub namespace: Option<String>,
    pub name: String,
    pub version: String,
    pub attributes: Vec<KeyPairs>,
    pub elements: Vec<OverlayElementDef>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct KeyPairs {
    pub name: String,
    pub kind: AttributeType,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum AttributeType {
    Text,
    Ref,
    Binary,
    Array,
}

#[derive(Debug, Clone)]
pub struct OverlayElementDef {
    pub name: String,
    pub keys: KeyType,
    pub values: ElementType,
}

#[derive(Debug, Clone, Eq, PartialEq)]
/// Type of the keys allowed for Overlay Object
pub enum KeyType {
    /// Keys names needs to correspond to the attribute names from Capture Base
    AttrNames,
    /// Keys are any arbitrary string
    Text
}

#[derive(Debug, Clone, Eq, PartialEq)]
/// Type of the values allowed for Overlay Object
pub enum ElementType {
    Object,
    Array(Option<Vec<ConstraintKind>>),
    Text,
    /// Reference in form of SAID to another object
    Ref,
}

#[derive(Debug,Clone, Eq, PartialEq)]
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
                attributes: Vec::new(),
                elements: Vec::new(),
            };
            for attr in line.into_inner() {
                match attr.as_rule() {
                    Rule::overlay_name => {
                        let (namespace, name): (Option<String>, String) = match attr.as_str().split_once(':') {
                            Some((ns, n)) if !ns.is_empty() => (Some(ns.to_string()), n.to_string()),
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
                        let mut overlay_element = OverlayElementDef {
                            name: "".to_string(),
                            keys: KeyType::AttrNames,
                            values: ElementType::Object,
                        };
                        for e in attr.into_inner() {
                            match e.as_rule() {
                                Rule::overlay_object_header => {
                                    e.into_inner().for_each(|header| {
                                        match header.as_rule() {
                                            Rule::attr_name=> {
                                                overlay_element.name = header.as_str().to_string();
                                            }
                                            _ => { panic!("Missing name for an object: {:?}", header.as_rule()) }
                                        }
                                    });
                                }
                                Rule::overlay_object_body => {
                                    for key in e.into_inner() {
                                        match key.as_rule() {
                                            Rule::key_type => {
                                                match key.into_inner().next() {
                                                    Some(k) => match k.as_rule() {
                                                        Rule::ATTR_NAMES_TYPE => {
                                                          overlay_element.keys = KeyType::AttrNames
                                                        },
                                                        Rule::TEXT_TYPE => {
                                                          overlay_element.keys = KeyType::Text
                                                        }
                                                        Rule::ARRAY_TYPE => todo!(),
                                                        _ => continue,
                                                    },
                                                    None => {
                                                        return Err(ParseError::MetaError("key type is empty".to_string()));
                                                    }
                                                }
                                            },
                                            Rule::value_type => {
                                                match key.into_inner().next() {
                                                    Some(k) => match k.as_rule() {
                                                        Rule::object_type=> {},
                                                        Rule::ARRAY_TYPE => {},
                                                        Rule::TEXT_TYPE => {},
                                                        Rule::REF_TYPE => {},
                                                        _ => continue,
                                                    },
                                                    None => {
                                                        return Err(ParseError::MetaError("value type is empty".to_string()));
                                                    }
                                                }
                                            }
                                            _ => continue,
                                        };
                                        overlay_def.elements.push(overlay_element.clone());
                                    }
                                }
                                _ => {}
                            }
                        }
                        overlay_def.elements.push(overlay_element);
                    }
                    Rule::overlay_attributes => {
                        for a in attr.into_inner() {
                            match a.as_rule() {
                                Rule::key_pair => {
                                    let mut key_pair = KeyPairs {
                                        name: "".to_string(),
                                        kind: AttributeType::Text,
                                    };
                                    a.into_inner().for_each(|kp| {
                                        match kp.as_rule() {
                                            Rule::attr_name => {
                                                key_pair.name = kp.as_str().to_string();
                                            }
                                            Rule::attr_value_type => {
                                                match kp.as_str() {
                                                    "Text" => key_pair.kind = AttributeType::Text,
                                                    "Ref" => key_pair.kind = AttributeType::Ref,
                                                    "Binary" => key_pair.kind = AttributeType::Binary,
                                                    "Array" => key_pair.kind = AttributeType::Array,
                                                    _ => {}
                                                }
                                            }
                                            _ => { panic!("Incorrect key pair for attribute: {:?}", kp.as_rule()) }
                                        }
                                    });
                                    overlay_def.attributes.push(key_pair);
                                }
                                _ => {}
                            }
                        }

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

    Ok(overlays_file)
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
    WITH KEYS Text
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
    WITH KEYS Text
    WITH VALUES Text"#;
        let result = parse_from_string(input.to_string()).unwrap();
        let overlay = result.overlays_def.first().unwrap();
        assert_eq!(overlay.name, "ReferenceValues");
        assert_eq!(overlay.namespace, None);

        let input = r#"
ADD OVERLAY hcf:ReferenceValues
  VERSION 1.0.0
  ADD OBJECT attribute_reference_values
    WITH KEYS Text
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
  ADD OBJECT attribute_information
    WITH KEYS attr-names
    WITH VALUES Text

ADD OVERLAY hcf:Meta
  VERSION 1.2.2
  ADD ATTRIBUTES name=Text description=Text photo=Binary
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
        assert_eq!(information.elements[0].name, "attribute_information");
        assert_eq!(information.elements.len(), 1);
        assert_eq!(information.name, "Information");

        assert_eq!(meta.version, "1.2.2");
        assert_eq!(meta.attributes.len(), 3);
        assert_eq!(meta.attributes[0].name, "name");
        assert_eq!(meta.attributes[0].kind, AttributeType::Text);

        assert_eq!(result.overlays_def.len(), 3);
    }
}
