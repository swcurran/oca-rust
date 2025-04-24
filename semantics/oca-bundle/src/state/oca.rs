use crate::state::oca::overlay::cardinality::Cardinalitys;
use crate::state::oca::overlay::character_encoding::CharacterEncodings;
use crate::state::oca::overlay::conformance::Conformances;
use crate::state::oca::overlay::entry::Entries;
use crate::state::oca::overlay::entry_code::EntryCodes;
use crate::state::oca::overlay::format::Formats;
use crate::state::oca::overlay::label::Labels;
use crate::state::oca::overlay::meta::Metas;
use crate::state::oca::overlay::unit::Units;
use indexmap::IndexMap;
use said::derivation::HashFunctionCode;
use said::sad::{SerializationFormats, SAD};
use said::version::SerializationInfo;
use serde::{ser::SerializeMap, Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
pub mod capture_base;
pub mod overlay;
use crate::state::{
    attribute::Attribute,
    oca::{capture_base::CaptureBase, overlay::Overlay},
};
use convert_case::{Case, Casing};
use isolang::Language;
use oca_ast_semantics::ast::{
    CaptureContent, Command, CommandType, Content, NestedValue, OCAAst, ObjectKind, OverlayType,
};
/// Internal representation of OCA objects in split between non-attributes values and attributes.
/// It is used to build dynamically objects without knowing yet whole structure of the object.
/// Used mainly as a container to hold information while parsing OCAfile.
/// Example of usage:
///
/// let oca = OCABox::new()
/// let attr = Attribute::new("name")
/// oca.add_attribute(attr)
/// oca.get_attribute_by_name("name").setEncoding(Encoding::UTF8)
/// oca.get_attribute_by_name("name").setLabel(Language::English, "Name")
/// oca.get_attribute_by_name("name").setUnit("kg")
/// oca.get_attribute_by_name("name").setStandard("ISO 1234")
/// oca.generate_bundle().unwrap()
///
///
/// TODO:
/// How to add multiple overlays like mapping (how to identify them?)

#[derive(Clone)]
pub struct OCABox {
    pub attributes: HashMap<String, Attribute>,
    pub mappings: Option<Vec<overlay::AttributeMapping>>,
    pub meta: Option<HashMap<Language, HashMap<String, String>>>,
}

impl Default for OCABox {
    fn default() -> Self {
        Self::new()
    }
}

impl OCABox {
    pub fn new() -> Self {
        OCABox {
            attributes: HashMap::new(),
            mappings: None,
            meta: None,
        }
    }
    /// Remove attribute from the OCA Bundle
    /// if attribute does not exist, nothing will happen
    pub fn remove_attribute(&mut self, attr_name: &String) {
        self.attributes.remove(attr_name);
    }
    /// Add an attribute to the OCA Bundle
    /// If the attribute already exists, it will be merged with the new attribute
    /// for simple types: the new value will overwrite the old value
    /// for complex types: the new value will be added to the old value
    pub fn add_attribute(&mut self, attribute: Attribute) {
        if let Some(attr) = self.get_attribute_mut(&attribute.name) {
            attr.merge(&attribute);
        } else {
            self.attributes.insert(attribute.name.clone(), attribute);
        }
    }
    pub fn get_attribute_by_name(&self, name: &str) -> Option<&Attribute> {
        self.attributes.get(name)
    }

    pub fn add_attribute_mapping(&mut self, mapping: overlay::AttributeMapping) {
        match self.mappings {
            Some(ref mut mappings) => mappings.push(mapping),
            None => self.mappings = Some(vec![mapping]),
        }
    }

    pub fn generate_bundle(&mut self) -> OCABundle {
        let mut capture_base = self.generate_capture_base();
        let mut overlays = self.generate_overlays();

        capture_base.calculate_said();

        let cb_said = capture_base.said.as_ref();
        overlays.iter_mut().for_each(|x| x.calculate_said(cb_said.unwrap()));

        let mut oca_bundle = OCABundle {
            said: None,
            capture_base,
            overlays,
        };

        oca_bundle.fill_said();
        oca_bundle
    }

    fn generate_overlays(&mut self) -> Vec<DynOverlay> {
        let mut overlays: Vec<DynOverlay> = Vec::new();
        if let Some(mappings) = &self.mappings {
            for mapping in mappings {
                overlays.push(Box::new(mapping.clone()));
            }
        }
        if let Some(meta) = &self.meta {
            for (lang, attr_pairs) in meta {
                let meta_ov = overlay::Meta::new(*lang, attr_pairs.clone());
                overlays.push(Box::new(meta_ov));
            }
        }

        for attribute in self.attributes.values() {
            let overlay_version = "2.0.0".to_string();
            if attribute.encoding.is_some() {
                let mut encoding_ov = overlays
                    .iter_mut()
                    .find(|x| x.overlay_type().eq(&OverlayType::CharacterEncoding(overlay_version.clone())));
                if encoding_ov.is_none() {
                    overlays.push(Box::new(overlay::CharacterEncoding::new()));
                    encoding_ov = overlays.last_mut();
                }
                if let Some(ov) = encoding_ov {
                    ov.add(attribute);
                }
            }

            if attribute.format.is_some() {
                let mut format_ov = overlays
                    .iter_mut()
                    .find(|x| x.overlay_type().eq(&OverlayType::Format(overlay_version.clone())));
                if format_ov.is_none() {
                    overlays.push(Box::new(overlay::Format::new()));
                    format_ov = overlays.last_mut();
                }
                if let Some(ov) = format_ov {
                    ov.add(attribute);
                }
            }

            if attribute.conformance.is_some() {
                let mut conformance_ov = overlays
                    .iter_mut()
                    .find(|x| x.overlay_type().eq(&OverlayType::Conformance(overlay_version.clone())));
                if conformance_ov.is_none() {
                    overlays.push(Box::new(overlay::Conformance::new()));
                    conformance_ov = overlays.last_mut();
                }
                if let Some(ov) = conformance_ov {
                    ov.add(attribute);
                }
            }

            if attribute.cardinality.is_some() {
                let mut cardinality_ov = overlays
                    .iter_mut()
                    .find(|x| x.overlay_type().eq(&OverlayType::Cardinality(overlay_version.clone())));
                if cardinality_ov.is_none() {
                    overlays.push(Box::new(overlay::Cardinality::new()));
                    cardinality_ov = overlays.last_mut();
                }
                if let Some(ov) = cardinality_ov {
                    ov.add(attribute);
                }
            }

            if attribute.unit.is_some() {
                let mut unit_ov = overlays
                    .iter_mut()
                    .find(|x| x.overlay_type().eq(&OverlayType::Unit(overlay_version.clone())));
                if unit_ov.is_none() {
                    overlays.push(Box::new(overlay::Unit::new()));
                    unit_ov = overlays.last_mut();
                }
                if let Some(ov) = unit_ov {
                    ov.add(attribute);
                }
            }

            if attribute.entry_codes.is_some() {
                let mut entry_code_ov = overlays
                    .iter_mut()
                    .find(|x| x.overlay_type().eq(&OverlayType::EntryCode(overlay_version.clone())));
                if entry_code_ov.is_none() {
                    overlays.push(Box::new(overlay::EntryCode::new()));
                    entry_code_ov = overlays.last_mut();
                }
                if let Some(ov) = entry_code_ov {
                    ov.add(attribute);
                }
            }

            if let Some(entries) = &attribute.entries {
                for lang in entries.keys() {
                    let mut entry_ov = overlays.iter_mut().find(|x| {
                        x.overlay_type().eq(&OverlayType::Entry(overlay_version.clone())) && x.language() == Some(lang)
                    });
                    if entry_ov.is_none() {
                        overlays.push(Box::new(overlay::Entry::new(*lang)));
                        entry_ov = overlays.last_mut();
                    }
                    if let Some(ov) = entry_ov {
                        ov.add(attribute);
                    }
                }
            }

            if let Some(labels) = &attribute.labels {
                for lang in labels.keys() {
                    let mut label_ov = overlays.iter_mut().find(|x| {
                        x.overlay_type().eq(&OverlayType::Label(overlay_version.clone())) && x.language() == Some(lang)
                    });
                    if label_ov.is_none() {
                        overlays.push(Box::new(overlay::Label::new(*lang)));
                        label_ov = overlays.last_mut();
                    }
                    if let Some(ov) = label_ov {
                        ov.add(attribute);
                    }
                }
            }
        }

        overlays
    }
    fn generate_capture_base(&mut self) -> CaptureBase {
        let mut capture_base = CaptureBase::new();
        for attribute in self.attributes.values() {
            capture_base.add(attribute);
        }
        capture_base
    }
    fn get_attribute_mut(&mut self, name: &str) -> Option<&mut Attribute> {
        self.attributes.get_mut(name)
    }
}

pub type DynOverlay = Box<dyn Overlay + Send + Sync + 'static>;

impl<'de> Deserialize<'de> for DynOverlay {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let de_overlay = serde_value::Value::deserialize(deserializer)?;
        if let serde_value::Value::Map(ref overlay) = de_overlay {
            if let Some(de_overlay_type) =
                overlay.get(&serde_value::Value::String("type".to_string()))
            {
                let overlay_type = de_overlay_type
                    .clone()
                    .deserialize_into::<OverlayType>()
                    .map_err(|e| serde::de::Error::custom(format!("Overlay type: {e}")))?;

                match overlay_type {
                    OverlayType::AttributeMapping(_) => {
                        return Ok(Box::new(
                            de_overlay
                                .deserialize_into::<overlay::AttributeMapping>()
                                .map_err(|e| {
                                    serde::de::Error::custom(format!(
                                        "Attribute Mapping overlay: {e}"
                                    ))
                                })?,
                        ));
                    }
                    OverlayType::CharacterEncoding(_) => {
                        return Ok(Box::new(
                            de_overlay
                                .deserialize_into::<overlay::CharacterEncoding>()
                                .map_err(|e| {
                                    serde::de::Error::custom(format!(
                                        "Character Encoding overlay: {e}"
                                    ))
                                })?,
                        ));
                    }
                    OverlayType::Cardinality(_) => {
                        return Ok(Box::new(
                            de_overlay
                                .deserialize_into::<overlay::Cardinality>()
                                .map_err(|e| {
                                    serde::de::Error::custom(format!("Cardinality overlay: {e}"))
                                })?,
                        ));
                    }
                    OverlayType::Conformance(_) => {
                        return Ok(Box::new(
                            de_overlay
                                .deserialize_into::<overlay::Conformance>()
                                .map_err(|e| {
                                    serde::de::Error::custom(format!("Conformance overlay: {e}"))
                                })?,
                        ));
                    }
                    OverlayType::Entry(_) => {
                        return Ok(Box::new(
                            de_overlay
                                .deserialize_into::<overlay::Entry>()
                                .map_err(|e| {
                                    serde::de::Error::custom(format!("Entry overlay: {e}"))
                                })?,
                        ));
                    }
                    OverlayType::EntryCode(_) => {
                        return Ok(Box::new(
                            de_overlay
                                .deserialize_into::<overlay::EntryCode>()
                                .map_err(|e| {
                                    serde::de::Error::custom(format!("Entry Code overlay: {e}"))
                                })?,
                        ));
                    }
                    OverlayType::EntryCodeMapping(_) => {
                        return Ok(Box::new(
                            de_overlay
                                .deserialize_into::<overlay::EntryCodeMapping>()
                                .map_err(|e| {
                                    serde::de::Error::custom(format!(
                                        "Entry Code Mapping overlay: {e}"
                                    ))
                                })?,
                        ));
                    }
                    OverlayType::Unit(_) => {
                        return Ok(Box::new(
                            de_overlay
                                .deserialize_into::<overlay::Unit>()
                                .map_err(|e| {
                                    serde::de::Error::custom(format!("Unit overlay: {e}"))
                                })?,
                        ));
                    }

                    OverlayType::Format(_) => {
                        return Ok(Box::new(
                            de_overlay
                                .deserialize_into::<overlay::Format>()
                                .map_err(|e| {
                                    serde::de::Error::custom(format!("Format overlay: {e}"))
                                })?,
                        ));
                    }

                    OverlayType::Label(_) => {
                        return Ok(Box::new(
                            de_overlay
                                .deserialize_into::<overlay::Label>()
                                .map_err(|e| {
                                    serde::de::Error::custom(format!("Label overlay: {e}"))
                                })?,
                        ));
                    }
                    OverlayType::Meta(_) => {
                        return Ok(Box::new(
                            de_overlay
                                .deserialize_into::<overlay::Meta>()
                                .map_err(|e| {
                                    serde::de::Error::custom(format!("Meta overlay: {e}"))
                                })?,
                        ));
                    }

                    OverlayType::Standard(_) => {
                        return Ok(Box::new(
                            de_overlay
                                .deserialize_into::<overlay::Standard>()
                                .map_err(|e| {
                                    serde::de::Error::custom(format!("Standard overlay: {e}"))
                                })?,
                        ));
                    }
                    _ => {
                        return Err(serde::de::Error::custom(format!(
                            "Overlay type not supported: {:?}",
                            overlay_type
                        )));
                    }
                }
            } else {
                return Err(serde::de::Error::missing_field("type"));
            }
        }

        Err(serde::de::Error::custom(format!(
            "overlay must be an object, got: {de_overlay:?}"
        )))
    }
}

pub fn serialize_overlays<S>(overlays: &Vec<DynOverlay>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    use serde_value::Value;
    use std::collections::BTreeMap;

    #[derive(Serialize)]
    #[serde(untagged)]
    enum OverlayValue {
        Array(Vec<DynOverlay>),
        Object(Box<dyn Overlay + Send>),
    }

    let overlays_many = [
        "meta",
        "entry",
        "label",
        "link",
    ];

    let mut overlays_map: BTreeMap<Value, OverlayValue> = BTreeMap::new();
    for overlay in overlays {
        let o_type_str = overlay.overlay_type().to_string().to_case(Case::Snake);

        if overlays_many.contains(&o_type_str.as_str()) {
            if let Some(OverlayValue::Array(ov)) =
                overlays_map.get_mut(&Value::String(o_type_str.clone()))
            {
                ov.push(overlay.clone());
            } else {
                overlays_map.insert(
                    Value::String(o_type_str.clone()),
                    OverlayValue::Array(vec![overlay.clone()]),
                );
            }
        } else {
            overlays_map.insert(
                Value::String(o_type_str),
                OverlayValue::Object(overlay.clone()),
            );
        }
    }

    let mut ser = s.serialize_map(Some(overlays_map.len()))?;
    for (ov_type, v) in overlays_map.iter_mut() {
        if let OverlayValue::Array(ov) = v {
            ov.sort_by(|a, b| {
                if let Some(a_said) = a.said() {
                    if let Some(b_said) = b.said() {
                        a_said.to_string().cmp(&b_said.to_string())
                    } else {
                        std::cmp::Ordering::Equal
                    }
                } else {
                    std::cmp::Ordering::Equal
                }
            });
        }

        ser.serialize_entry(ov_type, v)?;
    }
    ser.end()
}

fn deserialize_overlays<'de, D>(deserializer: D) -> Result<Vec<DynOverlay>, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    struct OverlaysVisitor;

    impl<'de> serde::de::Visitor<'de> for OverlaysVisitor {
        type Value = Vec<DynOverlay>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("vector of overlays")
        }

        fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
        where
            V: serde::de::MapAccess<'de>,
        {
            let mut overlays = vec![];

            while let Some((_, value)) = map.next_entry::<String, serde_value::Value>()? {
                if let serde_value::Value::Seq(ov) = value {
                    for o in ov {
                        overlays.push(o.deserialize_into().unwrap());
                    }
                } else if let serde_value::Value::Map(_) = value {
                    overlays.push(value.deserialize_into().unwrap());
                }
            }

            Ok(overlays)
        }
    }

    deserializer.deserialize_any(OverlaysVisitor)
}

impl std::fmt::Debug for DynOverlay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DynOverlay {{ overlay_type: {}, attributes: {:?} }}",
            self.overlay_type(),
            self.attributes()
        )
    }
}

#[derive(SAD, Serialize, Debug, Deserialize, Clone)]
#[version(protocol = "OCAS", major = 1, minor = 1)]
pub struct OCABundle {
    #[said]
    #[serde(rename = "digest")]
    pub said: Option<said::SelfAddressingIdentifier>,
    pub capture_base: CaptureBase,
    #[serde(
        serialize_with = "serialize_overlays",
        deserialize_with = "deserialize_overlays"
    )]
    pub overlays: Vec<DynOverlay>,
}

impl From<OCABundle> for OCABox {
    fn from(oca_bundle: OCABundle) -> Self {
        let mut oca_box = OCABox::new();

        let mut attributes: HashMap<String, Attribute> = HashMap::new();
        for (attr_name, attr_type) in oca_bundle.capture_base.attributes {
            let attr = Attribute {
                name: attr_name.clone(),
                attribute_type: Some(attr_type),
                // TODO find out how to make sure that said or Array said would be in a attr type
                // reference_sai: ref_said,
                ..Default::default()
            };
            attributes.insert(attr_name.clone(), attr);
        }

        let meta_overlays = oca_bundle
            .overlays
            .iter()
            .filter_map(|x| x.as_any().downcast_ref::<overlay::Meta>())
            .collect::<Vec<_>>();
        for overlay in meta_overlays {
            for (meta_name, meta_value) in overlay.attr_pairs.iter() {
                oca_box.add_meta(
                    *overlay.language().unwrap(),
                    meta_name.clone(),
                    meta_value.clone(),
                );
            }
        }

        let character_encoding_overlays = oca_bundle
            .overlays
            .iter()
            .filter_map(|x| x.as_any().downcast_ref::<overlay::CharacterEncoding>())
            .collect::<Vec<_>>();
        for overlay in character_encoding_overlays {
            for (attr_name, encoding) in overlay.attribute_character_encoding.iter() {
                attributes
                    .get_mut(attr_name)
                    .unwrap()
                    .set_encoding(*encoding);
            }
        }

        let conformance_overlays = oca_bundle
            .overlays
            .iter()
            .filter_map(|x| x.as_any().downcast_ref::<overlay::Conformance>())
            .collect::<Vec<_>>();
        for overlay in conformance_overlays {
            for (attr_name, conformance) in overlay.attribute_conformance.iter() {
                attributes
                    .get_mut(attr_name)
                    .unwrap()
                    .set_conformance(conformance.clone());
            }
        }

        let cardinality_overlays = oca_bundle
            .overlays
            .iter()
            .filter_map(|x| x.as_any().downcast_ref::<overlay::Cardinality>())
            .collect::<Vec<_>>();
        for overlay in cardinality_overlays {
            for (attr_name, cardinality) in overlay.attribute_cardinality.iter() {
                attributes
                    .get_mut(attr_name)
                    .unwrap()
                    .set_cardinality(cardinality.clone());
            }
        }

        {
            let format_overlays = oca_bundle
                .overlays
                .iter()
                .filter_map(|x| x.as_any().downcast_ref::<overlay::Format>())
                .collect::<Vec<_>>();
            for overlay in format_overlays {
                for (attr_name, format) in overlay.attribute_formats.iter() {
                    attributes
                        .get_mut(attr_name)
                        .unwrap()
                        .set_format(format.clone());
                }
            }
        }

        let unit_overlays = oca_bundle
            .overlays
            .iter()
            .filter_map(|x| x.as_any().downcast_ref::<overlay::Unit>())
            .collect::<Vec<_>>();
        for overlay in unit_overlays {
            for (attr_name, unit) in overlay.attribute_unit.iter() {
                attributes
                    .get_mut(attr_name)
                    .unwrap()
                    .set_unit(unit.clone());
            }
        }

        let entry_code_overlays = oca_bundle
            .overlays
            .iter()
            .filter_map(|x| x.as_any().downcast_ref::<overlay::EntryCode>())
            .collect::<Vec<_>>();
        for overlay in entry_code_overlays {
            for (attr_name, entry_code) in overlay.attribute_entry_codes.iter() {
                attributes
                    .get_mut(attr_name)
                    .unwrap()
                    .set_entry_codes(entry_code.clone());
            }
        }

        let entry_overlays = oca_bundle
            .overlays
            .iter()
            .filter_map(|x| x.as_any().downcast_ref::<overlay::Entry>())
            .collect::<Vec<_>>();
        for overlay in entry_overlays {
            for (attr_name, entries) in overlay.attribute_entries.iter() {
                attributes
                    .get_mut(attr_name)
                    .unwrap()
                    .set_entry(*overlay.language().unwrap(), entries.clone());
            }
        }

        let label_overlays = oca_bundle
            .overlays
            .iter()
            .filter_map(|x| x.as_any().downcast_ref::<overlay::Label>())
            .collect::<Vec<_>>();
        for overlay in label_overlays {
            for (attr_name, label) in overlay.attribute_labels.iter() {
                attributes
                    .get_mut(attr_name)
                    .unwrap()
                    .set_label(*overlay.language().unwrap(), label.clone());
            }
        }

        for (_, attribute) in attributes {
            oca_box.add_attribute(attribute);
        }

        oca_box
    }
}

impl OCABundle {
    pub fn fill_said(&mut self) {
        let code = HashFunctionCode::Blake3_256;
        let format = SerializationFormats::JSON;
        self.compute_digest(&code, &format);
    }

    pub fn to_ast(&self) -> OCAAst {
        let mut ast = OCAAst::new();

        let properties = None;

        let mut attributes = IndexMap::new();
        self.capture_base
            .attributes
            .iter()
            .for_each(|(attr_name, attr_type)| {
                attributes.insert(attr_name.clone(), attr_type.clone());
            });

        let command = Command {
            kind: CommandType::Add,
            object_kind: ObjectKind::CaptureBase(CaptureContent {
                // TODO find out if we can use indexmap in capture base to simplify stuff
                attributes: Some(self.capture_base.attributes.clone().into_iter().collect()),
                properties,
            }),
        };
        ast.commands.push(command);

        self.overlays.iter().for_each(|overlay| {
            match overlay.overlay_type() {
                OverlayType::CharacterEncoding(_) => {
                    let character_encoding = overlay
                        .as_any()
                        .downcast_ref::<overlay::CharacterEncoding>()
                        .unwrap();
                    let mut attributes = IndexMap::new();
                    for (attr_name, encoding) in
                        character_encoding.attribute_character_encoding.iter()
                    {
                        let encoding_val = serde_json::to_value(encoding).unwrap();
                        attributes.insert(
                            attr_name.clone(),
                            NestedValue::Value(encoding_val.as_str().unwrap().to_string()),
                        );
                    }
                    let command = Command {
                        kind: CommandType::Add,
                        object_kind: ObjectKind::Overlay(
                            overlay.overlay_type().clone(),
                            Content {
                                attributes: Some(attributes),
                                properties: None,
                            },
                        ),
                    };
                    ast.commands.push(command);
                }
                OverlayType::Format(_) => {
                    let format = overlay.as_any().downcast_ref::<overlay::Format>().unwrap();
                    let mut attributes = IndexMap::new();
                    for (attr_name, format) in format.attribute_formats.iter() {
                        attributes.insert(attr_name.clone(), NestedValue::Value(format.clone()));
                    }
                    let command = Command {
                        kind: CommandType::Add,
                        object_kind: ObjectKind::Overlay(
                            overlay.overlay_type().clone(),
                            Content {
                                attributes: Some(attributes),
                                properties: None,
                            },
                        ),
                    };
                    ast.commands.push(command);
                }
                OverlayType::Meta(_) => {
                    let meta = overlay.as_any().downcast_ref::<overlay::Meta>().unwrap();
                    let mut properties = IndexMap::new();
                    properties.insert(
                        "lang".to_string(),
                        NestedValue::Value(
                            meta.language().unwrap().to_639_1().unwrap().to_string(),
                        ),
                    );
                    for (meta_name, meta_value) in meta.attr_pairs.iter() {
                        properties
                            .insert(meta_name.clone(), NestedValue::Value(meta_value.clone()));
                    }
                    let command = Command {
                        kind: CommandType::Add,
                        object_kind: ObjectKind::Overlay(
                            overlay.overlay_type().clone(),
                            Content {
                                attributes: None,
                                properties: Some(properties),
                            },
                        ),
                    };
                    ast.commands.push(command);
                }
                OverlayType::Label(_) => {
                    let label = overlay.as_any().downcast_ref::<overlay::Label>().unwrap();
                    let mut properties = IndexMap::new();
                    properties.insert(
                        "lang".to_string(),
                        NestedValue::Value(
                            label.language().unwrap().to_639_1().unwrap().to_string(),
                        ),
                    );
                    let mut attributes = IndexMap::new();
                    for (attr_name, label) in label.attribute_labels.iter() {
                        attributes.insert(attr_name.clone(), NestedValue::Value(label.clone()));
                    }
                    let command = Command {
                        kind: CommandType::Add,
                        object_kind: ObjectKind::Overlay(
                            overlay.overlay_type().clone(),
                            Content {
                                attributes: Some(attributes),
                                properties: Some(properties),
                            },
                        ),
                    };
                    ast.commands.push(command);
                }
                OverlayType::Conformance(_) => {
                    let conformance = overlay
                        .as_any()
                        .downcast_ref::<overlay::Conformance>()
                        .unwrap();
                    let mut attributes = IndexMap::new();
                    for (attr_name, conformance) in conformance.attribute_conformance.iter() {
                        attributes
                            .insert(attr_name.clone(), NestedValue::Value(conformance.clone()));
                    }
                    let command = Command {
                        kind: CommandType::Add,
                        object_kind: ObjectKind::Overlay(
                            overlay.overlay_type().clone(),
                            Content {
                                attributes: Some(attributes),
                                properties: None,
                            },
                        ),
                    };
                    ast.commands.push(command);
                }
                OverlayType::EntryCode(_) => {
                    let entry_code = overlay
                        .as_any()
                        .downcast_ref::<overlay::EntryCode>()
                        .unwrap();
                    let mut attributes = IndexMap::new();
                    for (attr_name, entry_code) in entry_code.attribute_entry_codes.iter() {
                        match entry_code {
                            crate::state::entry_codes::EntryCodes::Sai(said) => {
                                attributes.insert(
                                    attr_name.clone(),
                                    NestedValue::Value(said.to_string()),
                                );
                            }
                            crate::state::entry_codes::EntryCodes::Array(entry_codes) => {
                                attributes.insert(
                                    attr_name.clone(),
                                    NestedValue::Array(
                                        entry_codes
                                            .iter()
                                            .map(|code| NestedValue::Value(code.clone()))
                                            .collect(),
                                    ),
                                );
                            }
                            crate::state::entry_codes::EntryCodes::Object(grouped_entry_codes) => {
                                attributes.insert(
                                    attr_name.clone(),
                                    NestedValue::Object(
                                        grouped_entry_codes
                                            .iter()
                                            .map(|(k, v)| {
                                                let codes = v
                                                    .iter()
                                                    .map(|code| NestedValue::Value(code.clone()))
                                                    .collect();
                                                (k.clone(), NestedValue::Array(codes))
                                            })
                                            .collect(),
                                    ),
                                );
                            }
                        }
                    }
                    let command = Command {
                        kind: CommandType::Add,
                        object_kind: ObjectKind::Overlay(
                            overlay.overlay_type().clone(),
                            Content {
                                attributes: Some(attributes),
                                properties: None,
                            },
                        ),
                    };
                    ast.commands.push(command);
                }
                OverlayType::Entry(_) => {
                    let entry = overlay.as_any().downcast_ref::<overlay::Entry>().unwrap();
                    let mut properties = IndexMap::new();
                    properties.insert(
                        "lang".to_string(),
                        NestedValue::Value(
                            entry.language().unwrap().to_639_1().unwrap().to_string(),
                        ),
                    );
                    let mut attributes = IndexMap::new();
                    for (attr_name, entries) in entry.attribute_entries.iter() {
                        match entries {
                            crate::state::entries::EntriesElement::Sai(said) => {
                                attributes.insert(
                                    attr_name.clone(),
                                    NestedValue::Value(said.to_string()),
                                );
                            }
                            crate::state::entries::EntriesElement::Object(entries) => {
                                attributes.insert(
                                    attr_name.clone(),
                                    NestedValue::Object(
                                        entries
                                            .iter()
                                            .map(|(k, v)| {
                                                (k.clone(), NestedValue::Value(v.clone()))
                                            })
                                            .collect(),
                                    ),
                                );
                            }
                        }
                    }
                    let command = Command {
                        kind: CommandType::Add,
                        object_kind: ObjectKind::Overlay(
                            overlay.overlay_type().clone(),
                            Content {
                                attributes: Some(attributes),
                                properties: Some(properties),
                            },
                        ),
                    };
                    ast.commands.push(command);
                }
                OverlayType::Cardinality(_) => {
                    let cardinality = overlay
                        .as_any()
                        .downcast_ref::<overlay::Cardinality>()
                        .unwrap();
                    let mut attributes = IndexMap::new();
                    for (attr_name, cardinality) in cardinality.attribute_cardinality.iter() {
                        attributes
                            .insert(attr_name.clone(), NestedValue::Value(cardinality.clone()));
                    }
                    let command = Command {
                        kind: CommandType::Add,
                        object_kind: ObjectKind::Overlay(
                            overlay.overlay_type().clone(),
                            Content {
                                attributes: Some(attributes),
                                properties: None,
                            },
                        ),
                    };
                    ast.commands.push(command);
                }
                OverlayType::Unit(_) => {
                    let unit_ov = overlay.as_any().downcast_ref::<overlay::Unit>().unwrap();
                    let mut attributes = IndexMap::new();
                    for (attr_name, unit) in unit_ov.attribute_unit.iter() {
                        let unit_val = serde_json::to_value(unit).unwrap();
                        attributes.insert(
                            attr_name.clone(),
                            NestedValue::Value(unit_val.as_str().unwrap().to_string()),
                        );
                    }
                    let command = Command {
                        kind: CommandType::Add,
                        object_kind: ObjectKind::Overlay(
                            overlay.overlay_type().clone(),
                            Content {
                                attributes: Some(attributes),
                                properties: None,
                            },
                        ),
                    };
                    ast.commands.push(command);
                }
                _ => {}
            }
        });

        ast
    }
}

#[cfg(test)]
mod tests {
    use maplit::hashmap;
    use oca_ast_semantics::ast::{NestedAttrType, RefValue};
    use said::SelfAddressingIdentifier;

    use super::*;
    use crate::state::{attribute::AttributeType, entries::EntriesElement};

    #[test]
    fn build_oca_bundle() {
        let mut oca = OCABox::new();
        oca.add_meta(Language::Eng, "name".to_string(), "test name".to_string());
        oca.add_meta(
            Language::Eng,
            "description".to_string(),
            "test desc".to_string(),
        );
        let mut attr_remove = Attribute::new("removeme".to_string());
        attr_remove.set_attribute_type(NestedAttrType::Value(AttributeType::Text));
        oca.add_attribute(attr_remove);

        let mut attr = Attribute::new("first_name".to_string());
        attr.set_attribute_type(NestedAttrType::Value(AttributeType::Text));
        oca.add_attribute(attr);

        let mut attr2 = Attribute::new("gender".to_string());
        let entries = EntriesElement::Object(hashmap! {});
        attr2.set_entry(Language::Eng, entries);
        oca.remove_attribute(&"removeme".to_string());

        let mut attr = Attribute::new("last_name".to_string());
        attr.set_attribute_type(NestedAttrType::Value(AttributeType::Text));
        oca.add_attribute(attr);
        // oca.add_attribute(Attribute::new("last_name".to_string()));
        let oca_bundle = oca.generate_bundle();
        /* let oca_bundle_encoded = oca_bundle.encode().unwrap();
        let oca_bundle_json = String::from_utf8(oca_bundle_encoded).unwrap();
        println!("{}", oca_bundle_json); */
        let said = oca_bundle.said;
        let oca_bundle = oca.generate_bundle();
        let oca_bundle_json = serde_json::to_string_pretty(&oca_bundle).unwrap();
        let said2 = oca_bundle.said;
        println!("{}", oca_bundle_json);
        assert_eq!(said, said2);
    }

    #[test]
    fn load_oca_box_from_oca_bundle() {
        let mut oca = OCABox::new();
        oca.add_meta(Language::Eng, "name".to_string(), "test name".to_string());
        oca.add_meta(
            Language::Eng,
            "description".to_string(),
            "test desc".to_string(),
        );
        let mut attr = Attribute::new("first_name".to_string());
        attr.set_attribute_type(NestedAttrType::Value(AttributeType::Text));
        oca.add_attribute(attr);

        let mut attr = Attribute::new("last_name".to_string());
        attr.set_attribute_type(NestedAttrType::Value(AttributeType::Text));
        oca.add_attribute(attr);

        let mut attr = Attribute::new("ref".to_string());
        let said = SelfAddressingIdentifier::default();
        attr.set_attribute_type(NestedAttrType::Reference(RefValue::Said(said)));
        oca.add_attribute(attr);

        let oca_bundle = oca.generate_bundle();
        let said = oca_bundle.said.clone();

        let mut oca_box = OCABox::from(oca_bundle);
        let oca_bundle = oca_box.generate_bundle();
        let said2 = oca_bundle.said;

        assert_eq!(said, said2);
    }
}
