use indexmap::IndexMap;
use overlay_file::OverlayDef;
use said::SelfAddressingIdentifier;
use serde::ser::SerializeStruct;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::hash::Hash;
use std::{collections::HashMap, fmt, str::FromStr};
use strum_macros::Display;
use thiserror::Error;
use wasm_bindgen::prelude::*;

pub use self::attributes::NestedAttrType;

pub mod attributes;
pub mod error;
pub mod recursive_attributes;

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct OCAAst {
    pub version: String,
    pub commands: Vec<Command>,
    pub commands_meta: IndexMap<usize, CommandMeta>,
    pub meta: HashMap<String, String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct Command {
    #[serde(rename = "type")]
    pub kind: CommandType,
    #[serde(flatten)]
    pub object_kind: ObjectKind,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct CommandMeta {
    pub line_number: usize,
    pub raw_line: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub enum CommandType {
    Add,
    Remove,
    Modify,
    From,
}

#[derive(Debug, PartialEq, Clone, Eq)]
pub enum ObjectKind {
    CaptureBase(CaptureContent),
    OCABundle(BundleContent),
    Overlay(OverlayContent),
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ", self.kind)?;

        match &self.object_kind {
            ObjectKind::CaptureBase(content) => {
                write!(f, "ATTRIBUTE")?;
                if let Some(attributes) = &content.attributes {
                    for (key, value) in attributes {
                        write!(f, " {}={}", key, value)?;
                    }
                }
                if let Some(properties) = &content.properties {
                    for (key, value) in properties {
                        write!(f, " {}={}", key, value)?;
                    }
                }
            }
            ObjectKind::OCABundle(content) => {
                write!(f, "OCABUNDLE {}", content.said)?;
            }
            ObjectKind::Overlay(content) => {
                write!(f, "OVERLAY {}", content.overlay_def.get_full_name())?;
                if let Some(properties) = &content.properties {
                    for (key, value) in properties {
                        write!(f, " {}={}", key, value)?;
                    }
                }
            }
        }
        Ok(())
    }
}

// Implement Display for CommandType
impl fmt::Display for CommandType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandType::Add => write!(f, "ADD"),
            CommandType::Remove => write!(f, "REMOVE"),
            CommandType::Modify => write!(f, "MODIFY"),
            CommandType::From => write!(f, "FROM"),
        }
    }
}

// Implement Display for NestedAttrType
impl fmt::Display for NestedAttrType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NestedAttrType::Value(attr_type) => write!(f, "{}", attr_type),
            NestedAttrType::Array(nested) => write!(f, "[{}]", nested),
            NestedAttrType::Null => write!(f, "Null"),
            NestedAttrType::Reference(ref_value) => write!(f, "reference {}", ref_value),
        }
    }
}

// Implement Display for NestedValue
impl fmt::Display for NestedValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NestedValue::Reference(ref_value) => write!(f, "{}", ref_value),
            NestedValue::Value(value) => write!(f, "{}", value),
            NestedValue::Object(object) => {
                write!(f, "{{")?;
                for (i, (key, value)) in object.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "\"{}\": {}", key, value)?;
                }
                write!(f, "}}")
            }
            NestedValue::Array(array) => {
                write!(f, "[")?;
                for (i, value) in array.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", value)?;
                }
                write!(f, "]")
            }
        }
    }
}

impl Serialize for ObjectKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("ObjectKind", 3)?;
        match self {
            ObjectKind::CaptureBase(content) => {
                state.serialize_field("object_kind", "CaptureBase")?;
                state.serialize_field("content", content)?;
            }
            ObjectKind::OCABundle(content) => {
                state.serialize_field("object_kind", "OCABundle")?;
                state.serialize_field("content", content)?;
            }
            ObjectKind::Overlay(content) => {
                state.serialize_field("object_kind", "Overlay")?;
                state.serialize_field("content", content)?;
            }
        }
        state.end()
    }
}

impl<'de> Deserialize<'de> for ObjectKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Raw {
            #[serde(rename = "object_kind")]
            kind: String,
            content: serde_json::Value,
        }

        let raw = Raw::deserialize(deserializer)?;

        match raw.kind.as_str() {
            "CaptureBase" => Ok(ObjectKind::CaptureBase(
                serde_json::from_value(raw.content).map_err(de::Error::custom)?,
            )),
            "OCABundle" => Ok(ObjectKind::OCABundle(
                serde_json::from_value(raw.content).map_err(de::Error::custom)?,
            )),
            "Overlay" => Ok(ObjectKind::Overlay(
                serde_json::from_value(raw.content).map_err(de::Error::custom)?,
            )),
            _ => Err(de::Error::custom(format!(
                "Unknown object kind: {}",
                raw.kind
            ))),
        }
    }
}

impl From<u8> for ObjectKind {
    fn from(val: u8) -> Self {
        match val {
            0 => ObjectKind::OCABundle(BundleContent {
                said: ReferenceAttrType::Reference(RefValue::Name("".to_string())),
            }),
            1 => ObjectKind::CaptureBase(CaptureContent {
                attributes: None,
                properties: None,
            }),
            2 => ObjectKind::Overlay(OverlayContent {
                properties: None,
                overlay_def: OverlayDef::default(),
            }),
            _ => panic!("Invalid ObjectKind value"),
        }
    }
}

impl From<ObjectKind> for u8 {
    fn from(val: ObjectKind) -> Self {
        match val {
            ObjectKind::OCABundle(_) => 0,
            ObjectKind::CaptureBase(_) => 1,
            ObjectKind::Overlay(_) => 2,
        }
    }
}

pub struct OverlayInstance<'a> {
    pub schema: &'a OverlayDef,
    pub content: &'a OverlayContent,
}

impl Hash for ObjectKind {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            ObjectKind::CaptureBase(content) => {
                content.hash(state);
            }
            ObjectKind::OCABundle(content) => {
                content.hash(state);
            }
            // TODO hash over content as well?
            ObjectKind::Overlay(content) => {
                content.overlay_def.hash(state);
                if let Some(properties) = &content.properties {
                    for (key, value) in properties {
                        key.hash(state);
                        value.hash(state);
                    }
                }
            }
        }
    }
}

impl Hash for CaptureContent {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        if let Some(attributes) = &self.attributes {
            for (key, value) in attributes {
                key.hash(state);
                value.hash(state);
            }
        }
        if let Some(properties) = &self.properties {
            for (key, value) in properties {
                key.hash(state);
                value.hash(state);
            }
        }
    }
}

impl ObjectKind {
    pub fn capture_content(&self) -> Option<&CaptureContent> {
        match self {
            ObjectKind::CaptureBase(content) => Some(content),
            _ => None,
        }
    }

    pub fn overlay_content(&self) -> Option<&OverlayContent> {
        match self {
            ObjectKind::Overlay(content) => Some(content),
            _ => None,
        }
    }
    pub fn oca_bundle_content(&self) -> Option<&BundleContent> {
        match self {
            ObjectKind::OCABundle(content) => Some(content),
            _ => None,
        }
    }
}
#[wasm_bindgen]
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Copy, Display, Eq, Hash)]
pub enum AttributeType {
    Boolean,
    Binary,
    Text,
    Numeric,
    DateTime,
}

impl FromStr for AttributeType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Boolean" => Ok(AttributeType::Boolean),
            "Binary" => Ok(AttributeType::Binary),
            "Text" => Ok(AttributeType::Text),
            "Numeric" => Ok(AttributeType::Numeric),
            "DateTime" => Ok(AttributeType::DateTime),
            _ => Err(()),
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Eq, Hash)]
pub struct BundleContent {
    pub said: ReferenceAttrType,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Eq)]
pub struct CaptureContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<IndexMap<String, NestedAttrType>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<IndexMap<String, NestedValue>>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Eq)]
pub struct OverlayContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<IndexMap<String, NestedValue>>,
    pub overlay_def: OverlayDef,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Eq, Hash)]
#[serde(untagged)]
/// Enum representing type supported in bundle (From command)
///
/// References: supports ref said and ref name
pub enum ReferenceAttrType {
    Reference(RefValue),
}

impl fmt::Display for ReferenceAttrType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReferenceAttrType::Reference(ref_value) => write!(f, "reference {}", ref_value),
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Eq)]
#[serde(untagged)]
pub enum NestedValue {
    Reference(RefValue),
    Value(String),
    Object(IndexMap<String, NestedValue>),
    Array(Vec<NestedValue>),
}
impl NestedValue {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            NestedValue::Reference(ref_value) => {
                ref_value.hash(state);
            }
            NestedValue::Value(value) => {
                value.hash(state);
            }
            NestedValue::Object(object) => {
                for (key, value) in object {
                    key.hash(state);
                    value.hash(state);
                }
            }
            NestedValue::Array(array) => {
                for value in array {
                    value.hash(state);
                }
            }
        }
    }
    pub fn is_object(&self) -> bool {
        matches!(self, NestedValue::Object(_))
    }

    pub fn is_array(&self) -> bool {
        matches!(self, NestedValue::Array(_))
    }
}

#[derive(Debug, PartialEq, Clone, Hash, Eq)]
pub enum RefValue {
    Said(said::SelfAddressingIdentifier),
    // This type is supported only for local-reference feature from facade (oca)
    Name(String),
}

impl FromStr for RefValue {
    type Err = RefValueParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (tag, rest) = s
            .split_once(':')
            .ok_or(RefValueParsingError::MissingColon)?;
        match tag {
            "refs" => {
                let said = SelfAddressingIdentifier::from_str(rest)?;
                Ok(RefValue::Said(said))
            }
            "refn" => Ok(RefValue::Name(rest.to_string())),
            _ => Err(RefValueParsingError::UnknownTag(tag.to_string())),
        }
    }
}

#[derive(Error, Debug)]

pub enum RefValueParsingError {
    #[error("Missing colon")]
    MissingColon,
    #[error("Unknown tag `{0}`. Referece need to start with `refs` od `refn`")]
    UnknownTag(String),
    #[error("Invalid said: {0}")]
    SaidError(#[from] said::error::Error),
}

impl fmt::Display for RefValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            RefValue::Said(said) => write!(f, "refs:{}", said),
            RefValue::Name(name) => write!(f, "refn:{}", name),
        }
    }
}
impl Serialize for RefValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match &self {
            RefValue::Said(said) => serializer.serialize_str(format!("refs:{}", said).as_str()),
            RefValue::Name(name) => serializer.serialize_str(format!("refn:{}", name).as_str()),
        }
    }
}

impl<'de> Deserialize<'de> for RefValue {
    fn deserialize<D>(deserializer: D) -> Result<RefValue, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let (tag, rest) = s.split_once(':').ok_or(serde::de::Error::custom(format!(
            "invalid reference: {}",
            s
        )))?;
        match tag {
            "refs" => {
                let said = SelfAddressingIdentifier::from_str(rest);
                match said {
                    Ok(said) => Ok(RefValue::Said(said)),
                    Err(_) => Err(serde::de::Error::custom(format!(
                        "invalid reference: {}",
                        s
                    ))),
                }
            }
            "refn" => Ok(RefValue::Name(rest.to_string())),
            _ => Err(serde::de::Error::custom(format!(
                "unknown reference type: {}",
                tag
            ))),
        }
    }
}

impl OCAAst {
    pub fn new() -> Self {
        OCAAst {
            // Version of OCA specification
            version: String::from("2.0.0"),
            commands: Vec::new(),
            commands_meta: IndexMap::new(),
            meta: HashMap::new(),
        }
    }
}

impl Default for OCAAst {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use indexmap::indexmap;
    use overlay_file::overlay_registry::{OverlayLocalRegistry, OverlayRegistry};

    use super::*;

    #[test]
    fn test_ocaast_serialize() {
        let _ = env_logger::builder().is_test(true).try_init();
        let mut attributes = IndexMap::new();
        let mut properties = IndexMap::new();

        let arr = NestedAttrType::Array(Box::new(NestedAttrType::Value(AttributeType::Boolean)));
        attributes.insert("allowed".to_string(), arr);
        attributes.insert(
            "test".to_string(),
            NestedAttrType::Value(AttributeType::Text),
        );

        properties.insert("test".to_string(), NestedValue::Value("test".to_string()));
        let command = Command {
            kind: CommandType::Add,
            object_kind: ObjectKind::CaptureBase(CaptureContent {
                attributes: Some(attributes),
                properties: Some(properties),
            }),
        };

        let overlay_registry =
            OverlayLocalRegistry::from_dir("../overlay-file/core_overlays/").unwrap();
        assert_eq!(overlay_registry.list_all().len(), 11);

        let label_overlay_def = overlay_registry.get_by_fqn("Label/2.0.0").unwrap();
        assert_eq!(label_overlay_def.get_full_name(), "label/2.0.0");

        let mut label_props = IndexMap::new();
        label_props.insert(
            "language".to_string(),
            NestedValue::Value("pl-PL".to_string()),
        );
        let attr_labels =
            indexmap! { "allowed".to_string() => NestedValue::Value("Dopuszczony".to_string())};
        let labels = NestedValue::Object(attr_labels.clone());
        label_props.insert("attribute_labels".to_string(), labels);

        let lable_command = Command {
            kind: CommandType::Add,
            object_kind: ObjectKind::Overlay(OverlayContent {
                properties: Some(label_props),
                overlay_def: label_overlay_def.clone(),
            }),
        };

        let mut ocaast = OCAAst::new();
        ocaast.commands.push(command);
        ocaast.commands.push(lable_command);

        // let wrapped = OCAAstWithRegistry {
        //     ast: &ocaast,
        //     registry: &overlay_registry,
        // };

        let serialized = serde_json::to_string(&ocaast).unwrap();

        assert_eq!(
            serialized,
            r#"{"version":"2.0.0","commands":[{"type":"Add","object_kind":"CaptureBase","content":{"attributes":{"allowed":["Boolean"],"test":"Text"},"properties":{"test":"test"}}},{"type":"Add","object_kind":"Overlay","content":{"properties":{"language":"pl-PL","attribute_labels":{"allowed":"Dopuszczony"}},"overlay_def":{"namespace":null,"name":"label","version":"2.0.0","elements":[{"name":"language","keys":"Text","values":"Text"},{"name":"attr_labels","keys":"AttrNames","values":"Text"}]}}}],"commands_meta":{},"meta":{}}"#
        );

        let ast = serde_json::from_str::<OCAAst>(&serialized).unwrap();
        assert_eq!(ocaast.version, ast.version);
        assert_eq!(
            ocaast
                .commands
                .last()
                .unwrap()
                .object_kind
                .overlay_content()
                .unwrap()
                .overlay_def
                .get_full_name(),
            "label/2.0.0"
        );
        let content = ocaast
            .commands
            .last()
            .unwrap()
            .object_kind
            .overlay_content()
            .unwrap();
        let props = content.properties.clone().unwrap();
        let attr_labels = props.get("attribute_labels").unwrap();
        let to_owned = if let NestedValue::Object(obj) = attr_labels {
            obj.get("allowed")
        } else {
            None
        };

        assert_eq!(
            to_owned.unwrap().clone(),
            NestedValue::Value("Dopuszczony".to_string())
        );

        assert_eq!(ocaast, ast);
    }
}
