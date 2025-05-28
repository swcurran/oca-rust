use indexmap::IndexMap;
use overlay_file::{overlay_registry::OverlayRegistry, OverlayDef};
use said::SelfAddressingIdentifier;
use serde::{
    de::{self, DeserializeSeed, Error},
    ser::SerializeStruct,
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::{collections::HashMap, fmt, str::FromStr};
use std::hash::Hash;
use strum_macros::Display;
use thiserror::Error;
use wasm_bindgen::prelude::*;

pub use self::attributes::NestedAttrType;

pub mod attributes;
pub mod error;
pub mod recursive_attributes;

#[derive(Debug, PartialEq, Serialize, Clone)]
pub struct OCAAst {
    pub version: String,
    pub commands: Vec<Command>,
    pub commands_meta: IndexMap<usize, CommandMeta>,
    pub meta: HashMap<String, String>,
}

#[derive(Debug, PartialEq, Serialize, Clone)]
pub struct Command {
    #[serde(rename = "type")]
    pub kind: CommandType,
    #[serde(flatten)]
    pub object_kind: ObjectKind,
}

// Temporary struct used only for deserialization to feed registry context
pub struct CommandSeed<'a> {
    pub registry: &'a dyn OverlayRegistry,
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

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Eq)]
#[serde(untagged)]
pub enum ObjectKind {
    CaptureBase(CaptureContent),
    OCABundle(BundleContent),
    Overlay(String, Content),
}

impl<'de, 'a> DeserializeSeed<'de> for CommandSeed<'a> {
    type Value = Command;

    fn deserialize<D>(self, deserializer: D) -> Result<Command, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Raw {
            #[serde(rename = "type")]
            kind: CommandType,
            object_kind: String,
            content: serde_json::Value,
        }

        let raw = Raw::deserialize(deserializer)?;

        let object_kind = match raw.object_kind.as_str() {
            "CaptureBase" => ObjectKind::CaptureBase(
                serde_json::from_value(raw.content).map_err(de::Error::custom)?,
            ),
            "OCABundle" => ObjectKind::OCABundle(
                serde_json::from_value(raw.content).map_err(de::Error::custom)?,
            ),
            overlay => {
                if self.registry.get_by_name(overlay).unwrap().is_none() {
                    return Err(de::Error::custom(format!("Unknown overlay: {overlay}")));
                }
                let content = serde_json::from_value(raw.content).map_err(de::Error::custom)?;
                ObjectKind::Overlay(overlay.to_string(), content)
            }
        };

        Ok(Command {
            kind: raw.kind,
            object_kind,
        })
    }
}

// Temporary struct used only for serialization to feed registry context
pub struct OCAAstWithRegistry<'a> {
    pub ast: &'a OCAAst,
    pub registry: &'a dyn OverlayRegistry,
}

impl<'a> Serialize for OCAAstWithRegistry<'a> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("OCAAst", 4)?;

        s.serialize_field("version", &self.ast.version)?;

        let wrapped_commands: Vec<_> = self
            .ast
            .commands
            .iter()
            .map(|cmd| CommandWithRegistry {
                command: cmd,
                registry: self.registry,
            })
            .collect();

        s.serialize_field("commands", &wrapped_commands)?;
        s.serialize_field("commands_meta", &self.ast.commands_meta)?;
        s.serialize_field("meta", &self.ast.meta)?;
        s.end()
    }
}

pub struct OverlayInstance<'a> {
    pub schema: &'a OverlayDef,
    pub content: &'a Content,
}

/// TODO find out if this is needed...
impl ObjectKind {
    pub fn as_overlay<'a>(
        &'a self,
        registry: &'a dyn OverlayRegistry,
    ) -> Option<OverlayInstance<'a>> {
        match self {
            ObjectKind::Overlay(name, content) => registry
                .get_by_name(name).unwrap()
                .map(|schema| OverlayInstance { schema, content }),
            _ => None,
        }
    }
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
            ObjectKind::Overlay(overlay_type, _) => {
                overlay_type.hash(state);
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

    pub fn overlay_content(&self) -> Option<&Content> {
        match self {
            ObjectKind::Overlay(_, content) => Some(content),
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
pub struct Content {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<IndexMap<String, NestedValue>>,
    #[serde(skip)]
    pub version: Option<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Eq, Hash)]
#[serde(untagged)]
/// Enum representing type supported in bundle (From command)
///
/// References: supports ref said and ref name
pub enum ReferenceAttrType {
    Reference(RefValue),
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

pub fn parse_oca_ast_with_registry<'de>(
    json: &'de str,
    registry: &dyn OverlayRegistry,
) -> Result<OCAAst, serde_json::Error> {

    let mut map = serde_json::Map::new();
    map.extend(
        serde_json::from_str::<serde_json::Value>(json)?
            .as_object()
            .unwrap()
            .clone(),
    );

    let version = map.remove("version").unwrap();
    let commands = map.remove("commands").unwrap();
    let commands_meta = map.remove("commands_meta").unwrap();
    let meta = map.remove("meta").unwrap();

    let commands = commands
        .as_array()
        .unwrap()
        .iter()
        .map(|v| {
            CommandSeed { registry }
                .deserialize(v.clone())
                .map_err(serde_json::Error::custom)
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(OCAAst {
        version: version.as_str().unwrap().to_string(),
        commands,
        commands_meta: serde_json::from_value(commands_meta)?,
        meta: serde_json::from_value(meta)?,
    })
}

pub struct CommandWithRegistry<'a> {
    pub command: &'a Command,
    pub registry: &'a dyn OverlayRegistry,
}

impl<'a> Serialize for CommandWithRegistry<'a> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;

        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("type", &self.command.kind)?;

        match &self.command.object_kind {
            ObjectKind::CaptureBase(c) => {
                map.serialize_entry("object_kind", "CaptureBase")?;
                map.serialize_entry("content", c)?;
            }
            ObjectKind::OCABundle(c) => {
                map.serialize_entry("object_kind", "OCABundle")?;
                map.serialize_entry("content", c)?;
            }
            ObjectKind::Overlay(name, content) => {
                if self.registry.get_by_name(name).unwrap().is_none() {
                    return Err(serde::ser::Error::custom(format!(
                        "Unknown overlay during serialization: {}",
                        name
                    )));
                }
                map.serialize_entry("object_kind", name)?;
                map.serialize_entry("content", content)?;
            }
        }

        map.end()
    }
}

#[derive(Deserialize)]
struct RawCommand {
    #[serde(rename = "type")]
    kind: CommandType,
    object_kind: String,
    content: serde_json::Value,
}

#[derive(Deserialize)]
struct OCAAstRaw {
    version: String,
    commands: Vec<RawCommand>,
    commands_meta: IndexMap<usize, CommandMeta>,
    meta: HashMap<String, String>,
}
pub fn deserialize_oca_ast_with_registry(
    json: &str,
    registry: &dyn OverlayRegistry,
) -> Result<OCAAst, serde_json::Error> {
    let raw: OCAAstRaw = serde_json::from_str(json)?;

    let commands = raw
        .commands
        .into_iter()
        .map(|raw_cmd| {
            let object_kind = match raw_cmd.object_kind.as_str() {
                "CaptureBase" => {
                    ObjectKind::CaptureBase(serde_json::from_value(raw_cmd.content.clone())?)
                }
                "OCABundle" => {
                    ObjectKind::OCABundle(serde_json::from_value(raw_cmd.content.clone())?)
                }
                overlay => {
                    if registry.get_by_name(overlay).unwrap().is_none() {
                        return Err(serde_json::Error::custom(format!(
                            "Unknown overlay: {overlay}"
                        )));
                    }
                    let content = serde_json::from_value(raw_cmd.content.clone())?;
                    ObjectKind::Overlay(overlay.to_string(), content)
                }
            };

            Ok(Command {
                kind: raw_cmd.kind,
                object_kind,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(OCAAst {
        version: raw.version,
        commands,
        commands_meta: raw.commands_meta,
        meta: raw.meta,
    })
}

impl Default for OCAAst {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use overlay_file::overlay_registry::OverlayLocalRegistry;
    use indexmap::indexmap;

    use super::*;

    #[test]
    fn test_ocaast_serialize() {
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

        let overlay_registry = OverlayLocalRegistry::from_dir("test/").unwrap();
        assert_eq!(overlay_registry.list_all(), vec!["overlays".to_string()]);

        let overlays = overlay_registry.get_by_name("Label/2.0.0").unwrap().unwrap();
        assert_eq!(overlays.name, "label");

        let mut label_props = IndexMap::new();
        label_props.insert("language".to_string(), NestedValue::Value("pl-PL".to_string()));
        let attr_labels = indexmap! { "allowed".to_string() => NestedValue::Value("Dopuszczony".to_string())};
        let labels = NestedValue::Object(attr_labels.clone());
        label_props.insert("attribute_labels".to_string(), labels);

        let lable_command = Command {
            kind: CommandType::Add,
            object_kind: ObjectKind::Overlay(
                overlays.name.clone(),
                Content {
                    properties: Some(label_props),
                    version: None,
                },
            ),
        };

        let mut ocaast = OCAAst::new();
        ocaast.commands.push(command);
        ocaast.commands.push(lable_command);

        let wrapped = OCAAstWithRegistry {
            ast: &ocaast,
            registry: &overlay_registry,
        };

        let serialized = serde_json::to_string(&wrapped).unwrap();

        assert_eq!(
            serialized,
            r#"{"version":"2.0.0","commands":[{"type":"Add","object_kind":"CaptureBase","content":{"attributes":{"allowed":["Boolean"],"test":"Text"},"properties":{"test":"test"}}},{"type":"Add","object_kind":"label","content":{"properties":{"language":"pl-PL","attribute_labels":{"allowed":"Dopuszczony"}}}}],"commands_meta":{},"meta":{}}"#
        );

        let ast = deserialize_oca_ast_with_registry(&serialized, &overlay_registry).unwrap();
        assert_eq!(ocaast.version, ast.version);
        assert_eq!(
            ocaast
                .commands
                .last()
                .unwrap()
                .object_kind
                .as_overlay(&overlay_registry)
                .unwrap()
                .schema
                .name,
            "label"
        );
        let content = ocaast.commands.last().unwrap().object_kind.overlay_content().unwrap();
        let props = content.properties.clone().unwrap();
        let attr_labels = props.get("attribute_labels").unwrap();
        let to_owned = if let NestedValue::Object(obj) = attr_labels {
            obj.get("allowed")
        } else {
            None
        };

        assert_eq!(to_owned.unwrap().clone(), NestedValue::Value("Dopuszczony".to_string()));

        assert_eq!(ocaast, ast);
    }
}
