use isolang::Language;
use std::collections::{HashMap, HashSet};

use crate::state::oca_bundle::OCABundleModel;

#[derive(Debug)]
pub enum Error {
    Custom(String),
    MissingTranslations(Language),
    MissingMetaTranslation(Language, String),
    UnexpectedTranslations(Language),
    MissingAttributeTranslation(Language, String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Custom(error) => write!(f, "{error}"),
            Error::MissingTranslations(language) => {
                write!(f, "Missing translation in {language} language")
            }
            Error::MissingMetaTranslation(language, attr) => write!(
                f,
                "Missing meta translation for {attr} in {language} language"
            ),
            Error::UnexpectedTranslations(language) => {
                write!(f, "Unexpected translations in {language} language")
            }
            Error::MissingAttributeTranslation(language, attr) => {
                write!(f, "Missing translation for {attr} in {language} language")
            }
        }
    }
}

impl std::error::Error for Error {}

pub enum SemanticValidationStatus {
    Valid,
    Invalid(Vec<Error>),
}

pub fn validate(oca_bundle: &OCABundleModel) -> Result<SemanticValidationStatus, String> {
    let validator = Validator::new();
    match validator.validate(oca_bundle) {
        Ok(_) => Ok(SemanticValidationStatus::Valid),
        Err(errors) => Ok(SemanticValidationStatus::Invalid(errors)),
    }
}

pub struct Validator {
    enforced_translations: Vec<Language>,
}

impl Default for Validator {
    fn default() -> Self {
        Self::new()
    }
}

impl Validator {
    pub fn new() -> Validator {
        Validator {
            enforced_translations: vec![],
        }
    }

    pub fn enforce_translations(mut self, languages: Vec<Language>) -> Validator {
        self.enforced_translations = self
            .enforced_translations
            .into_iter()
            .chain(languages)
            .collect::<Vec<Language>>();
        self
    }

    pub fn validate(self, _oca_bundle: &OCABundleModel) -> Result<(), Vec<Error>> {
        let _enforced_langs: HashSet<_> = self.enforced_translations.iter().collect();
        let mut errors: Vec<Error> = vec![];
        self.validate_unique_keys(_oca_bundle, &mut errors);

        /* let oca_bundle: OCABundle = serde_json::from_str(oca_str.as_str())
                   .map_err(|e| vec![Error::Custom(e.to_string())])?;
        */
        // let mut recalculated_oca_bundle = oca_bundle.clone();
        // recalculated_oca_bundle.fill_said();
        //
        // if oca_bundle.said.ne(&recalculated_oca_bundle.said) {
        //     errors.push(Error::Custom("OCA Bundle: Malformed SAID".to_string()));
        // }
        //
        // let capture_base = &oca_bundle.capture_base;
        //
        // let mut recalculated_capture_base = capture_base.clone();
        // recalculated_capture_base.calculate_said();
        //
        // if capture_base.said.ne(&recalculated_capture_base.said) {
        //     errors.push(Error::Custom("capture_base: Malformed SAID".to_string()));
        // }

        // for o in &oca_bundle.overlays {
        //     let mut recalculated_overlay = o.clone();
        //     recalculated_overlay.fill_said();
        //     if o.digest.ne(&recalculated_overlay.digest) {
        //         // let msg = match o.language() {
        //         //     Some(lang) => format!("{} ({}): Malformed SAID", o.overlay_type(), lang),
        //         //     None => format!("{}: Malformed SAID", o.overlay_type()),
        //         // };
        //         let msg = format!("{}: Malformed SAID", o.name);
        //         errors.push(Error::Custom(msg));
        //     }
        //
        //     if o.capture_base.ne(&capture_base.said) {
        //         // let msg = match o.language() {
        //         //     Some(lang) => {
        //         //         format!("{} ({}): Mismatch capture_base SAI", o.overlay_type(), lang)
        //         //     }
        //         //     None => format!("{}: Mismatch capture_base SAI", o.overlay_type()),
        //         // };
        //         let msg = format!("{}: Mismatch capture_base SAI", o.name);
        //         errors.push(Error::Custom(msg));
        //     }
        // }

        // if !enforced_langs.is_empty() {
        //     let meta_overlays = oca_bundle
        //         .overlays
        //         .iter()
        //         .filter_map(|x| x.as_any().downcast_ref::<overlay::Meta>())
        //         .collect::<Vec<_>>();
        //
        //     if !meta_overlays.is_empty() {
        //         if let Err(meta_errors) = self.validate_meta(&enforced_langs, meta_overlays) {
        //             errors = errors
        //                 .into_iter()
        //                 .chain(meta_errors.into_iter().map(|e| {
        //                     if let Error::UnexpectedTranslations(lang) = e {
        //                         Error::Custom(format!(
        //                             "meta overlay: translations in {lang:?} language are not enforced"
        //                         ))
        //                     } else if let Error::MissingTranslations(lang) = e {
        //                         Error::Custom(format!(
        //                             "meta overlay: translations in {lang:?} language are missing"
        //                         ))
        //                     } else if let Error::MissingMetaTranslation(lang, attr) = e {
        //                         Error::Custom(format!(
        //                             "meta overlay: for '{attr}' translation in {lang:?} language is missing"
        //                         ))
        //                     } else {
        //                         e
        //                     }
        //                 }))
        //                 .collect();
        //         }
        //     }
        //
        //     for overlay_type in &["Entry", "Label"] {
        //         let typed_overlays: Vec<_> = oca_bundle
        //             .overlays
        //             .iter()
        //             .filter(|x| x.overlay_type().to_string().eq(&overlay_type.to_string()))
        //             .collect();
        //         if typed_overlays.is_empty() {
        //             continue;
        //         }
        //
        //         if let Err(translation_errors) =
        //             self.validate_translations(&enforced_langs, typed_overlays)
        //         {
        //             errors = errors.into_iter().chain(
        //                 translation_errors.into_iter().map(|e| {
        //                     if let Error::UnexpectedTranslations(lang) = e {
        //                         Error::Custom(
        //                             format!("{overlay_type} overlay: translations in {lang:?} language are not enforced")
        //                         )
        //                     } else if let Error::MissingTranslations(lang) = e {
        //                         Error::Custom(
        //                             format!("{overlay_type} overlay: translations in {lang:?} language are missing")
        //                         )
        //                     } else if let Error::MissingAttributeTranslation(lang, attr_name) = e {
        //                         Error::Custom(
        //                             format!("{overlay_type} overlay: for '{attr_name}' attribute missing translations in {lang:?} language")
        //                         )
        //                     } else {
        //                         e
        //                     }
        //                 })
        //             ).collect();
        //         }
        //     }
        // }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn validate_unique_keys(&self, oca_bundle: &OCABundleModel, errors: &mut Vec<Error>) {
        let mut seen: HashMap<String, HashSet<String>> = HashMap::new();

        for overlay in &oca_bundle.overlays {
            let overlay_def = match &overlay.overlay_def {
                Some(def) => def,
                None => continue,
            };
            if overlay_def.unique_keys.is_empty() {
                continue;
            }
            let properties = match &overlay.properties {
                Some(props) => props,
                None => {
                    errors.push(Error::Custom(format!(
                        "Overlay {} is missing properties for unique keys",
                        overlay_def.get_full_name()
                    )));
                    continue;
                }
            };

            let mut parts = Vec::new();
            let mut missing = Vec::new();
            for key in &overlay_def.unique_keys {
                match properties.get(key) {
                    Some(value) => {
                        let value_str =
                            serde_json::to_string(value).unwrap_or_else(|_| value.to_string());
                        parts.push(format!("{}={}", key, value_str));
                    }
                    None => missing.push(key.clone()),
                }
            }

            if !missing.is_empty() {
                errors.push(Error::Custom(format!(
                    "Overlay {} is missing unique keys: {}",
                    overlay_def.get_full_name(),
                    missing.join(", ")
                )));
                continue;
            }

            let signature = parts.join("|");
            let entry = seen.entry(overlay_def.get_full_name()).or_default();
            if !entry.insert(signature.clone()) {
                errors.push(Error::Custom(format!(
                    "Duplicate overlay {} with unique keys {}",
                    overlay_def.get_full_name(),
                    signature
                )));
            }
        }
    }

    // fn validate_meta(
    //     &self,
    //     enforced_langs: &HashSet<&Language>,
    //     meta_overlays: Vec<&overlay::Meta>,
    // ) -> Result<(), Vec<Error>> {
    //     let mut errors: Vec<Error> = vec![];
    //     let translation_langs: HashSet<_> = meta_overlays
    //         .iter()
    //         .map(|o| o.language().unwrap())
    //         .collect();
    //
    //     let missing_enforcement: HashSet<&_> =
    //         translation_langs.difference(enforced_langs).collect();
    //     for m in missing_enforcement {
    //         errors.push(Error::UnexpectedTranslations(**m));
    //     }
    //
    //     let missing_translations: HashSet<&_> =
    //         enforced_langs.difference(&translation_langs).collect();
    //     for m in missing_translations {
    //         errors.push(Error::MissingTranslations(**m));
    //     }
    //
    //     let attributes = meta_overlays
    //         .iter()
    //         .flat_map(|o| o.attr_pairs.keys())
    //         .collect::<HashSet<_>>();
    //
    //     for meta_overlay in meta_overlays {
    //         attributes.iter().for_each(|attr| {
    //             if !meta_overlay.attr_pairs.contains_key(*attr) {
    //                 errors.push(Error::MissingMetaTranslation(
    //                     *meta_overlay.language().unwrap(),
    //                     attr.to_string(),
    //                 ));
    //             }
    //         });
    //     }
    //
    //     if errors.is_empty() {
    //         Ok(())
    //     } else {
    //         Err(errors)
    //     }
    // }

    // fn validate_translations(
    //     &self,
    //     enforced_langs: &HashSet<&Language>,
    //     overlays: Vec<&DynOverlay>,
    // ) -> Result<(), Vec<Error>> {
    //     let mut errors: Vec<Error> = vec![];
    //
    //     let overlay_langs: HashSet<_> = overlays.iter().map(|x| x.language().unwrap()).collect();
    //
    //     let missing_enforcement: HashSet<&_> = overlay_langs.difference(enforced_langs).collect();
    //     for m in missing_enforcement {
    //         errors.push(Error::UnexpectedTranslations(**m)); // why we have && here?
    //     }
    //
    //     let missing_translations: HashSet<&_> = enforced_langs.difference(&overlay_langs).collect();
    //     for m in missing_translations {
    //         errors.push(Error::MissingTranslations(**m)); // why we have && here?
    //     }
    //
    //     let all_attributes: HashSet<&String> =
    //         overlays.iter().flat_map(|o| o.attributes()).collect();
    //     for overlay in overlays.iter() {
    //         let attributes: HashSet<_> = overlay.attributes().into_iter().collect();
    //
    //         let missing_attr_translation: HashSet<&_> =
    //             all_attributes.difference(&attributes).collect();
    //         for m in missing_attr_translation {
    //             errors.push(Error::MissingAttributeTranslation(
    //                 *overlay.language().unwrap(),
    //                 m.to_string(),
    //             ));
    //         }
    //     }
    //
    //     if errors.is_empty() {
    //         Ok(())
    //     } else {
    //         Err(errors)
    //     }
    // }
}

#[cfg(test)]
mod tests {
    use indexmap::IndexMap;
    use oca_ast::ast::{NestedValue, OverlayContent};
    use overlay_file::overlay_registry::OverlayLocalRegistry;
    use overlay_file::parse_from_string;

    use super::*;
    use crate::controller::load_oca;
    use crate::state::oca_bundle::OCABundleModel;
    use crate::state::oca_bundle::capture_base::CaptureBase;
    use crate::state::oca_bundle::overlay::OverlayModel;

    #[test]
    fn validate_valid_oca() {
        // let validator = Validator::new().enforce_translations(vec![Language::Eng, Language::Pol]);
        //
        // // let mut oca = cascade! {
        // //     OCABox::new();
        // //     // ..add_meta(Language::Eng, "name".to_string(), "Driving Licence".to_string());
        // //     // ..add_meta(Language::Eng, "description".to_string(), "DL".to_string());
        // //     // ..add_meta(Language::Pol, "name".to_string(), "Prawo Jazdy".to_string());
        // //     // ..add_meta(Language::Pol, "description".to_string(), "PJ".to_string());
        // // };
        //
        // let attribute = cascade! {
        //     Attribute::new("name".to_string());
        //     ..set_attribute_type(NestedAttrType::Value(AttributeType::Text));
        //     // ..set_encoding(Encoding::Utf8);
        //     // ..set_label(Language::Eng, "Name: ".to_string());
        //     // ..set_label(Language::Pol, "Imię: ".to_string());
        // };
        //
        // oca.add_attribute(attribute);
        //
        // let attribute_2 = cascade! {
        //     Attribute::new("age".to_string());
        //     ..set_attribute_type(NestedAttrType::Value(AttributeType::Numeric));
        //     // ..set_label(Language::Eng, "Age: ".to_string());
        //     // ..set_label(Language::Pol, "Wiek: ".to_string());
        // };
        //
        // oca.add_attribute(attribute_2);
        //
        // let oca_bundle = oca.generate_bundle();
        //
        // let result = validator.validate(&oca_bundle);
        //
        // if let Err(ref errors) = result {
        //     println!("{errors:?}");
        // }
        //assert!(result.is_ok());
    }

    #[test]
    fn validate_oca_with_missing_name_translation() {
        // let validator = Validator::new().enforce_translations(vec![Language::Eng, Language::Pol]);
        //
        // let mut oca = cascade! {
        //     OCABox::new();
        //     // ..add_meta(Language::Eng, "name".to_string(), "Driving Licence".to_string());
        // };
        //
        // let oca_bundle = oca.generate_bundle();
        //
        // let result = validator.validate(&oca_bundle);
        //
        // assert!(result.is_err());
        // if let Err(errors) = result {
        //     assert_eq!(errors.len(), 1);
        // }
    }

    #[test]
    #[ignore]
    fn validate_oca_with_invalid_saids() {
        let validator = Validator::new();
        let data = r#"
{
  "v": "OCAS02JSON0007c1_",
  "digest": "EDTaoqiaaL504P-HTxYWuiniwhrzGcP9ji-mPeJgudLk",
  "capture_base": {
    "digest": "EA0l-Sazi2X9cLn2pbVLr6C-t4-lVsSx3E_yJyEwTwum",
    "type": "capture_base/2.0.0",
    "attributes": {
      "d": "Text",
      "el": "Text",
      "i": "Text",
      "list": [
        "Text"
      ],
      "passed": "Boolean"
    }
  },
  "overlays": [
    {
      "digest": "EKN9PGIHxLuZe92ZDyrZulScFgTfAdjEc9xXEVb_WULX",
      "capture_base": "EA0l-Sazi2X9cLn2pbVLr6C-t4-lVsSx3E_yJyEwTwum",
      "type": "Meta/2.0.0",
      "description": "Entrance credential",
      "name": "Entrance credential"
    },
    {
      "digest": "EFOAxxDMSnOiuah9OwoCdwkns8EfsurcHXF57-XdGnen",
      "capture_base": "EA0l-Sazi2X9cLn2pbVLr6C-t4-lVsSx3E_yJyEwTwum",
      "type": "Character_Encoding/2.0.0",
      "d": "utf-8",
      "i": "utf-8",
      "passed": "utf-8"
    },
    {
      "digest": "ELivUa6QlCOpidnqLDs9Il1uqILb9pBUj2rLdGgqWDwv",
      "capture_base": "EA0l-Sazi2X9cLn2pbVLr6C-t4-lVsSx3E_yJyEwTwum",
      "type": "conformance/2.0.0",
      "d": "M",
      "i": "M",
      "passed": "M"
    },
    {
      "digest": "ECsW-Zb7A0TfG_M_HNH9wwKqil3rSiyKEfPE4398aQdC",
      "capture_base": "EA0l-Sazi2X9cLn2pbVLr6C-t4-lVsSx3E_yJyEwTwum",
      "type": "label/2.0.0",
      "d": "Schema digest",
      "i": "Credential Issuee",
      "passed": "Passed"
    },
    {
      "digest": "EJcEfNE3s_lZeUF1C_tez3qbThSsIJq4qV6WHlo-hmIL",
      "capture_base": "EA0l-Sazi2X9cLn2pbVLr6C-t4-lVsSx3E_yJyEwTwum",
      "type": "format/2.0.0",
      "d": "image/jpeg"
    },
    {
      "digest": "EICOF_bxwUyKC7W-blp51-YPPieJxqDPL7wrSkeT8jOg",
      "capture_base": "EA0l-Sazi2X9cLn2pbVLr6C-t4-lVsSx3E_yJyEwTwum",
      "type": "unit/2.0.0",
      "i": "m"
    },
    {
      "digest": "EPT1EDp2ofO1xJSQFehyZb8kCfMqXV8giTs0MeqQOp2a",
      "capture_base": "EA0l-Sazi2X9cLn2pbVLr6C-t4-lVsSx3E_yJyEwTwum",
      "type": "cardinality/2.0.0",
      "list": "1-2"
    },
    {
      "digest": "EKJ1z6PIFXqfP7wy6Hj21Of23HcoiT-b5P1qs_DgYJHo",
      "capture_base": "EA0l-Sazi2X9cLn2pbVLr6C-t4-lVsSx3E_yJyEwTwum",
      "type": "ENTRY_CODE/2.0.0",
      "el": [
        "o1",
        "o2",
        "o3"
      ],
      "list": "entry_code_said"
    },
    {
      "digest": "EKohdNuyxHWPZ1dy-Om5Rx4RxufHM5jjDKBa3jyRvp52",
      "capture_base": "EA0l-Sazi2X9cLn2pbVLr6C-t4-lVsSx3E_yJyEwTwum",
      "type": "ENTRY/2.0.0",
      "el": {
        "o1": "o1_label",
        "o2": "o2_label",
        "o3": "o3_label"
      },
      "list": "refs:ENrf7niTCnz7HD-Ci88rlxHlxkpQ2NIZNNv08fQnXANI"
    }
  ]
}
"#;

        let registry = OverlayLocalRegistry::from_dir("../overlay-file/core_overlays/").unwrap();
        let oca_bundle = load_oca(&mut data.as_bytes(), &registry);
        match oca_bundle {
            Ok(oca_bundle) => {
                let result = validator.validate(&oca_bundle);
                assert!(result.is_err());
                if let Err(errors) = result {
                    println!("{:?}", errors);
                    assert_eq!(errors.len(), 4);
                }
            }
            Err(e) => {
                println!("{:?}", e);
                panic!("Failed to load OCA bundle");
            }
        }
    }

    #[test]
    fn validate_unique_keys_multiple_and_duplicate_overlay() {
        let overlay_file = r#"
--name=Test
ADD OVERLAY ReferenceValues
  VERSION 1.0.1
  UNIQUE KEYS [language, region]
  ADD ATTRIBUTES language=Lang
  ADD ATTRIBUTES region=Text
"#;

        let overlay_def = parse_from_string(overlay_file.to_string())
            .unwrap()
            .overlays_def
            .remove(0);
        assert_eq!(
            overlay_def.unique_keys,
            vec!["language".to_string(), "region".to_string()]
        );

        let mut properties = IndexMap::new();
        properties.insert("language".to_string(), NestedValue::Value("en".to_string()));
        properties.insert("region".to_string(), NestedValue::Value("US".to_string()));

        let overlay_1 = OverlayModel::new(OverlayContent {
            properties: Some(properties.clone()),
            overlay_def: overlay_def.clone(),
        });
        let overlay_2 = OverlayModel::new(OverlayContent {
            properties: Some(properties),
            overlay_def,
        });

        let oca_bundle = OCABundleModel::new(CaptureBase::new(), vec![overlay_1, overlay_2]);
        let validator = Validator::new();
        let result = validator.validate(&oca_bundle);

        assert!(result.is_err());
        if let Err(errors) = result {
            assert!(
                errors
                    .iter()
                    .any(|error| error.to_string().contains("Duplicate overlay"))
            );
        }
    }
}
