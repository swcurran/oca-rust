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
        if self.namespace.is_some() {
            format!("{}:{}/{}", self.namespace.as_ref().unwrap(), self.name, self.version)
        } else {
            format!("{}/{}", self.name, self.version)
        }
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
    /// Key type for attribtues which are simple pairs, and do not have keys
    None,
}

#[derive(Hash, Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
/// Type of the values allowed for Overlay Object
pub enum ElementType {
    Object(Option<Box<OverlayElementDef>>),
    Array(Option<Vec<ConstraintKind>>),
    Binary,
    Text,
    /// Language code according to ISO 639-1 or ISO 639-3
    Lang,
    /// Reference in form of SAID to another object
    Ref,
    Complex(Vec<ElementType>),
    /// Allow for any type
    Any,
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
                        match namespace {
                            Some(ns) => {
                                overlay_def.namespace = Some(ns.to_lowercase());
                            }
                            None => {
                                overlay_def.namespace = None;
                            }
                        };
                        overlay_def.name = name.to_lowercase();
                    }
                    Rule::version => {
                        overlay_def.version = attr.as_str().to_string();
                    }
                    Rule::overlay_object => {
                        let mut name: Option<String> = None;
                        let mut keys_type: Option<KeyType> = None;
                        let mut values_type: Option<ElementType> = None;
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
                                    (keys_type, values_type) = parse_object_body(e)?;
                                }
                                _ => {}
                            }
                        }
                        let overlay_element = OverlayElementDef {
                            name: name.clone().unwrap_or_default(),
                            keys: keys_type.clone().unwrap_or(KeyType::Text),
                            values: values_type.clone().unwrap_or(ElementType::Object(None)),
                        };
                        overlay_def.elements.push(overlay_element);
                    }
                    Rule::overlay_attributes => {
                        let attrs = process_overlay_attributes(attr);
                        overlay_def.elements.extend(attrs);
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
                                        Rule::array_type => {
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
                            values: values.clone().unwrap_or(ElementType::Object(None)),
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

fn process_overlay_attributes(attr: Pair) -> Vec<OverlayElementDef> {
    debug!("Parsing overlay attribute: {:?}", attr);
    let mut elements = Vec::new();
    for a in attr.into_inner() {
        match a.as_rule() {
            Rule::key_pair => {
                let element = process_key_pair(a);
                debug!("-------- Found element {:?}", element);
                elements.push(element);
            }
            Rule::ATTR_ARRAY => {
                let result = process_attributes_array(a);
                elements.extend(result);
            }
            _ => unreachable!(),
        }
    }
    elements
}

fn parse_object_body(
    object_body: Pair,
) -> Result<(Option<KeyType>, Option<ElementType>), ParseError> {
    let mut keys_type: Option<KeyType> = None;
    let mut values_type: Option<ElementType> = None;

    for key in object_body.into_inner() {
        match key.as_rule() {
            Rule::key_type => match key.into_inner().next() {
                Some(k) => match k.as_rule() {
                    Rule::ATTR_NAMES_TYPE => keys_type = Some(KeyType::AttrNames),
                    Rule::TEXT_TYPE => keys_type = Some(KeyType::Text),
                    Rule::ARRAY_KEY_TYPE => todo!(),
                    _ => continue,
                },
                None => {
                    return Err(ParseError::MetaError("key type is empty".to_string()));
                }
            },
            Rule::value_type => match key.into_inner().next() {
                Some(k) => match k.as_rule() {
                    Rule::object_type => {
                        let (nested_keys, nested_values) =
                            parse_object_body(k.into_inner().next().unwrap())?;
                        let nested_def = OverlayElementDef {
                            name: "".to_string(), // Nested objects don't have a name in this context
                            keys: nested_keys.unwrap_or(KeyType::Text),
                            values: nested_values.unwrap_or(ElementType::Any),
                        };
                        values_type = Some(ElementType::Object(Some(Box::new(nested_def))));
                    }
                    Rule::array_type => {
                        todo!()
                    }
                    Rule::TEXT_TYPE => {
                        values_type = Some(ElementType::Text);
                    }
                    Rule::REF_TYPE => {
                        values_type = Some(ElementType::Ref);
                    }
                    Rule::LANG_TYPE => {
                        values_type = Some(ElementType::Lang);
                    }
                    Rule::ANY_TYPE => {
                        values_type = Some(ElementType::Any);
                    }
                    Rule::complex_value_type => {
                        let mut complex_types = Vec::new();
                        for complex_type in k.into_inner() {
                            match complex_type.as_rule() {
                                Rule::array_type => complex_types.push(ElementType::Array(None)),
                                Rule::object_type => {
                                    let (nested_keys, nested_values) = parse_object_body(
                                        complex_type.into_inner().next().unwrap(),
                                    )?;
                                    let nested_def = OverlayElementDef {
                                        name: "".to_string(),
                                        keys: nested_keys.unwrap_or(KeyType::Text),
                                        values: nested_values.unwrap_or(ElementType::Any),
                                    };
                                    complex_types
                                        .push(ElementType::Object(Some(Box::new(nested_def))));
                                }
                                Rule::REF_TYPE => complex_types.push(ElementType::Ref),
                                Rule::TEXT_TYPE => complex_types.push(ElementType::Text),
                                Rule::LANG_TYPE => complex_types.push(ElementType::Lang),
                                _ => {}
                            }
                        }
                        values_type = Some(ElementType::Complex(complex_types));
                    }
                    _ => {
                        todo!("Value type not supported yet: {:?}", k)
                    }
                },
                None => {
                    return Err(ParseError::MetaError("value type is empty".to_string()));
                }
            },
            _ => continue,
        };
    }
    Ok((keys_type, values_type))
}

fn process_attributes_array(attr: Pair) -> Vec<OverlayElementDef> {
    let mut attributes: Vec<OverlayElementDef> = Vec::new();
    // Parse the list of attributes from array
    for array in attr.into_inner() {
        match array.as_rule() {
            Rule::ARRAY_KEY_TYPE => {
                for item in array.into_inner() {
                    match item.as_rule() {
                        Rule::array_content => {
                            let mut items = Vec::new();
                            let mut has_ellipsis = false;
                            debug!("Parsing array items: {:?}", item);

                            for content in item.into_inner() {
                                match content.as_rule() {
                                    Rule::array_items => {
                                        for item in content.into_inner() {
                                            if let Rule::key_item = item.as_rule() {
                                                items.push(item.as_str().to_string());
                                            }
                                        }
                                    }
                                    Rule::trailing_ellipsis => {
                                        has_ellipsis = true;
                                    }
                                    _ => {
                                        panic!(
                                            "Unexpected rule in array content: {:?}",
                                            content.as_rule()
                                        );
                                    }
                                }
                            }

                            for i in items {
                                attributes.push(OverlayElementDef {
                                    name: i.clone(),
                                    keys: KeyType::None,
                                    values: ElementType::Text, // Default value which would be overrided when we process WITH VALUES if present
                                });
                            }
                            // if ellipsis is present in array we crate an empty string element in attributes list representing any element
                            if has_ellipsis {
                                attributes.push(OverlayElementDef {
                                    name: "".to_string(),
                                    keys: KeyType::None,
                                    values: ElementType::Text,
                                });
                            }
                        }
                        _ => {
                            debug!("Unexpected rule in array type: {:?}", item.as_rule());
                        }
                    }
                }
            }
            Rule::keys_with_values => {
                debug!("Parsing overlay array with values: {:?}", array);
                for el in array.into_inner() {
                    match el.as_rule() {
                        Rule::value_type => {
                            debug!("process value type for attribute array");
                            let mut value_type = ElementType::Text;
                            match el.as_str().to_lowercase().as_str() {
                                "text" => value_type = ElementType::Text,
                                "ref" => value_type = ElementType::Ref,
                                "binary" => value_type = ElementType::Binary,
                                "array" => value_type = ElementType::Array(None),
                                "lang" => value_type = ElementType::Lang,
                                "any" => value_type = ElementType::Any,
                                _ => {}
                            }
                            for attribute in &mut attributes {
                                attribute.values = value_type.clone();
                            }
                        }
                        _ => unreachable!(),
                    }
                }
            }
            _ => unreachable!(),
        }
    }
    attributes
}

fn process_key_pair(pair: Pair) -> OverlayElementDef {
    let mut key_pair = KeyPair {
        name: "".to_string(),
        kind: ElementType::Text,
    };
    pair.into_inner().for_each(|kp| match kp.as_rule() {
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
            panic!("Incorrect key pair for attribute: {:?}", kp.as_rule())
        }
    });

    OverlayElementDef {
        name: key_pair.name.clone(),
        keys: KeyType::Text,
        values: key_pair.kind.clone(),
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
        assert_eq!(overlay.name, "referencevalues");
        assert_eq!(overlay.namespace, None);

        let input = r#"
ADD OVERLAY :ReferenceValues
  VERSION 1.0.0
  ADD OBJECT attribute_reference_values
    WITH KEYS attr-names
    WITH VALUES Text"#;
        let result = parse_from_string(input.to_string()).unwrap();
        let overlay = result.overlays_def.first().unwrap();
        assert_eq!(overlay.name, "referencevalues");
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
        assert_eq!(overlay.name, "referencevalues");
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
  ADD ATTRIBUTES language=Lang photo=Binary
  ADD ATTRIBUTES [name, description]
    WITH VALUES TEXT
  ADD ATTRIBUTES [...]
    WITH VALUES ANY

ADD OVERLAY ENTRY
  VERSION 1.2.2
  ADD ATTRIBUTES language=Lang
  ADD OBJECT attr_entries
    with keys attr-names
    with values Object
      with keys Text
      with values Object
        with keys Text
        with values Any

"#;

        let result = parse_from_string(input.to_string()).unwrap();

        let ref_overlay = result.overlays_def.first().unwrap();
        let information = result.overlays_def.get(1).unwrap();
        let meta = result.overlays_def.get(2).unwrap();
        let entry = result.overlays_def.get(3).unwrap();
        assert_eq!(result.overlays_def.len(), 4);
        assert_eq!(ref_overlay.name, "referencevalues");
        assert_eq!(ref_overlay.version, "1.0.1");
        assert_eq!(ref_overlay.namespace, None);
        assert_eq!(ref_overlay.elements.len(), 1);
        assert_eq!(ref_overlay.elements[0].name, "attribute_reference_values");
        assert_eq!(ref_overlay.elements[0].keys, KeyType::AttrNames);
        assert_eq!(
            ref_overlay.elements[0].values,
            ElementType::Object(Some(Box::new(OverlayElementDef {
                name: "".to_string(),
                keys: KeyType::Text,
                values: ElementType::Text
            })))
        );

        assert_eq!(information.version, "1.2.2");
        assert_eq!(information.namespace.clone().unwrap(), "hcf".to_string());
        debug!(">>>> {:?}", information);
        assert_eq!(information.elements[0].name, "language");
        assert_eq!(information.elements[0].values, ElementType::Lang);
        assert_eq!(information.elements[2].name, "attribute_information");
        assert_eq!(information.elements.len(), 3);
        assert_eq!(information.name, "information");

        assert_eq!(meta.version, "1.2.2");
        assert_eq!(meta.elements.len(), 5);
        assert_eq!(
            meta.elements
                .iter()
                .find(|e| e.name == "name")
                .unwrap()
                .values,
            ElementType::Text
        );
        assert_eq!(meta.elements.first().unwrap().name, "language");
        assert_eq!(meta.elements.first().unwrap().values, ElementType::Lang);
        assert_eq!(meta.elements.get(2).unwrap().name, "name");
        assert_eq!(meta.elements.get(2).unwrap().values, ElementType::Text);
        assert_eq!(meta.elements.last().unwrap().name, "");
        assert_eq!(meta.elements.last().unwrap().values, ElementType::Any);

        assert_eq!(entry.version, "1.2.2");
        assert_eq!(entry.elements.len(), 2);
        assert_eq!(entry.elements[1].name, "attr_entries");
        assert_eq!(
            entry.elements[1].values,
            ElementType::Object(Some(Box::new(OverlayElementDef {
                name: "".to_string(),
                keys: KeyType::Text,
                values: ElementType::Object(Some(Box::new(OverlayElementDef {
                    name: "".to_string(),
                    keys: KeyType::Text,
                    values: ElementType::Any
                })))
            })))
        );
    }
}
