use log::debug;

use crate::{ElementType, KeyType, OverlayDef, OverlayElementDef, OverlayFile};
use std::collections::HashSet;

pub struct OverlayfileValidator;

// TODO create validation rules for overlay definition file
// TODO create validation rules for ocafile against overlay definition file

#[derive(Debug, serde::Serialize)]
pub enum ValidationError {
    MissingRequiredElement(String),
    InvalidKeyType(String),
    InvalidValueType(String),
    DuplicateElement(String),
    UnknownUniqueKey(String),
}

impl OverlayfileValidator {
    pub fn validate(overlay_file: &OverlayFile) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        for overlay_def in &overlay_file.overlays_def {
            errors.extend(Self::validate_overlay_def(overlay_def));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn validate_overlay_def(overlay_def: &OverlayDef) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        let mut element_names = HashSet::new();

        for element in &overlay_def.elements {
            if !element_names.insert(&element.name) {
                debug!("duplicated {:?}", element);
                errors.push(ValidationError::DuplicateElement(element.name.clone()));
            }

            errors.extend(Self::validate_element(element));
        }

        for key in &overlay_def.unique_keys {
            if !element_names.contains(key) {
                errors.push(ValidationError::UnknownUniqueKey(key.clone()));
            }
        }

        errors
    }

    fn validate_element(element: &OverlayElementDef) -> Vec<ValidationError> {
        let errors = Vec::new();

        // Validate key type
        match element.keys {
            KeyType::AttrNames | KeyType::Text => {}
            KeyType::None => {}
        }

        // Validate value type
        match &element.values {
            ElementType::Object(_) | ElementType::Text | ElementType::Binary | ElementType::Ref => {
            }
            ElementType::Array(_constraints) => {
                // Add additional validation for array constraints if needed
            }
            ElementType::Lang => {
                //TODO Additional validation for language element
            }
            ElementType::Complex(_element_types) => {
                debug!("Validation of complex element for overlay");
            }
            ElementType::Any => {
                // Skip validation allow anything
            }
        }

        errors
    }
}
