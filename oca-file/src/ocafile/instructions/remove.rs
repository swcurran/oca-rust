use crate::ocafile::{Pair, Rule, error::InstructionError};
use indexmap::IndexMap;
use log::debug;
use oca_ast::ast::{
    CaptureContent, Command, CommandType, NestedAttrType, NestedValue, ObjectKind, OverlayContent,
};
use overlay_file::OverlayDef;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RemoveInstruction {}

impl RemoveInstruction {
    pub(crate) fn from_record(record: Pair, _index: usize) -> Result<Command, InstructionError> {
        let mut object_kind = None;

        debug!("Parsing remove instruction: {:?}", record);
        for object in record.into_inner() {
            match object.as_rule() {
                Rule::remove_overlay => {
                    // TODO how to fill overlay definition? see ADD instruction for example
                    object_kind = Some(ObjectKind::Overlay(OverlayContent {
                        properties: Some(extract_properties_pairs(object)),
                        overlay_def: OverlayDef::default(),
                    }));
                }
                Rule::remove_attribute => {
                    let mut attributes: IndexMap<String, NestedAttrType> = IndexMap::new();
                    for key in object.into_inner() {
                        debug!("Parsing key to remove: {:?}", key.as_str());
                        attributes.insert(key.as_str().to_string(), NestedAttrType::Null);
                    }
                    object_kind = Some(ObjectKind::CaptureBase(CaptureContent {
                        attributes: Some(attributes),
                    }));
                }
                _ => {
                    return Err(InstructionError::UnexpectedToken(format!(
                        "unexpected token {:?}",
                        object.as_rule()
                    )));
                }
            }
        }

        Ok(Command {
            kind: CommandType::Remove,
            object_kind: object_kind.unwrap(),
        })
    }
}

/// TODO do zaorania...
fn extract_properties_pairs(_object: Pair) -> IndexMap<String, NestedValue> {
    let properties: IndexMap<String, NestedValue> = IndexMap::new();
    properties
}

// fn extract_attribute_pairs(object: Pair) -> IndexMap<String, NestedValue> {
//     let mut attributes: IndexMap<String, NestedValue> = IndexMap::new();
//     for attr_pairs in object.into_inner() {
//         match attr_pairs.as_rule() {
//             Rule::attr_key => {
//                 debug!("Parsed attribute: {:?}", attr_pairs);
//                 // TODO find out how to parse nested objects
//                 attributes.insert(
//                     attr_pairs.as_str().to_string(),
//                     NestedValue::Value("".to_string()),
//                 );
//             }
//             _ => {
//                 return attributes;
//             }
//         }
//     }
//     attributes
// }
