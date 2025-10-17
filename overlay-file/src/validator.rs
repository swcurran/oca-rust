use crate::{ElementType, KeyType, OverlayDef, OverlayElementDef, OverlayFile};
use std::collections::HashSet;

pub struct OverlayValidator;

#[derive(Debug, serde::Serialize)]
pub enum ValidationError {
    MissingRequiredElement(String),
    InvalidKeyType(String),
    InvalidValueType(String),
    DuplicateElement(String),
}

impl OverlayValidator {
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
                errors.push(ValidationError::DuplicateElement(element.name.clone()));
            }

            errors.extend(Self::validate_element(element));
        }

        errors
    }

    fn validate_element(element: &OverlayElementDef) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        // Validate key type
        match element.keys {
            KeyType::AttrNames | KeyType::Text => {}
            _ => errors.push(ValidationError::InvalidKeyType(element.name.clone())),
        }

        // Validate value type
        match &element.values {
            ElementType::Object | ElementType::Text | ElementType::Binary | ElementType::Ref => {}
            ElementType::Array(constraints) => {
                // Add additional validation for array constraints if needed
            }
            ElementType::Lang => {
                //TODO Additional validation for language element
            }
        }

        errors
    }
}
