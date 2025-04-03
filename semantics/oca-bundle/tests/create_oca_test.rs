use isolang::Language;
use oca_bundle_semantics::state::{
    attribute::{Attribute, AttributeType},
    encoding::Encoding,
    entries::EntriesElement,
    entry_codes::EntryCodes as EntryCodesValue,
    oca::overlay::cardinality::Cardinalitys,
    oca::overlay::character_encoding::CharacterEncodings,
    oca::overlay::conditional::Conditionals,
    oca::overlay::conformance::Conformances,
    oca::overlay::entry::Entries,
    oca::overlay::entry_code::EntryCodes,
    oca::overlay::information::Information,
    oca::overlay::label::Labels,
    oca::overlay::meta::Metas,
    oca::overlay::unit::Units,
    oca::OCABox,
};

#[cfg(feature = "format_overlay")]
use oca_bundle_semantics::state::oca::overlay::format::Formats;

use cascade::cascade;
use maplit::hashmap;

#[test]
fn create_oca() {
    let mut oca = cascade! {
        OCABox::new();
        ..add_meta(Language::Eng, "name".to_string(), "Test".to_string());
        ..add_meta(Language::Eng, "description".to_string(), "Test case OCA".to_string());
    };

    let mut attribute = cascade! {
        Attribute::new("name".to_string());
        ..set_attribute_type(oca_ast_semantics::ast::NestedAttrType::Value(AttributeType::Text));
        ..set_encoding(Encoding::Utf8);
        ..set_cardinality("1".to_string());
        ..set_conformance("O".to_string());
        ..set_label(isolang::Language::Eng, "Name".to_string());
        ..set_information(isolang::Language::Eng, "name information".to_string());
        ..set_entry_codes(EntryCodesValue::Array(vec!["a".to_string(), "b".to_string()]));
        ..set_entry(isolang::Language::Pol, EntriesElement::Object(hashmap! {
            "a".to_string() => "Opcja A".to_string(),
            "b".to_string() => "Opcja B".to_string(),
        }));
        ..set_entry(isolang::Language::Eng, EntriesElement::Object(hashmap! {
            "a".to_string() => "Option A".to_string(),
            "b".to_string() => "Option B".to_string(),
        }));
        ..set_unit("kg".to_string());
    };
    #[cfg(feature = "format_overlay")]
    attribute.set_format("^[a-zA-Z]*$".to_string());

    oca.add_attribute(attribute);

    let mut attribute_2 = cascade! {
        Attribute::new("age".to_string());
        ..set_attribute_type(oca_ast_semantics::ast::NestedAttrType::Value(AttributeType::Numeric));
        ..set_encoding(Encoding::Utf8);
        ..set_cardinality("2".to_string());
        ..set_conformance("M".to_string());
        ..set_condition("${name} ~= nil and ${name} ~= ''".to_string());
        ..set_label(isolang::Language::Eng, "Age".to_string());
        ..set_information(isolang::Language::Eng, "age information".to_string());
        ..set_entry_codes(EntryCodesValue::Array(vec!["a".to_string(), "b".to_string()]));
        ..set_entry(isolang::Language::Eng, EntriesElement::Object(hashmap! {
            "a".to_string() => "Option A".to_string(),
            "b".to_string() => "Option B".to_string(),
        }));
        ..set_unit("kg".to_string());
    };
    #[cfg(feature = "format_overlay")]
    attribute_2.set_format("^[a-zA-Z]*$".to_string());

    oca.add_attribute(attribute_2);

    let oca_bundle = oca.generate_bundle();
    assert_eq!(oca_bundle.said, oca.generate_bundle().said);
    println!("{:#?}", oca_bundle);

    assert_eq!(oca_bundle.capture_base.attributes.len(), 2);

    #[cfg(not(feature = "format_overlay"))]
    assert_eq!(oca_bundle.overlays.len(), 11);
    #[cfg(feature = "format_overlay")]
    assert_eq!(oca_bundle.overlays.len(), 12);

    let serialized_bundle = serde_json::to_string_pretty(&oca_bundle).unwrap();
    println!("{}", serialized_bundle);

    let expected = if cfg!(feature = "format_overlay") {
        r#"{
  "d": "EMVt_-xNfr5DbxqNklc5AvgOtEwZpMUwROWFn18s_Xwk",
  "capture_base": {
    "d": "EKxVMSYCnIoPUfZHsKf8OTOhsNgJppZPLH8yHz2FdB9z",
    "type": "spec/capture_base/1.1",
    "attributes": {
      "age": "Numeric",
      "name": "Text"
    }
  },
  "overlays": {
    "cardinality": {
      "d": "EJFjpi67XgNXHlDilb-UoyFcZcf9m4jnqRPiQnx4vUAA",
      "capture_base": "EKxVMSYCnIoPUfZHsKf8OTOhsNgJppZPLH8yHz2FdB9z",
      "type": "spec/overlays/cardinality/1.1",
      "attribute_cardinality": {
        "age": "2",
        "name": "1"
      }
    },
    "character_encoding": {
      "d": "EGcXsw3LaRTlmnCVbxLYI2xB7WMkig2qI8pW5P98W0Mz",
      "capture_base": "EKxVMSYCnIoPUfZHsKf8OTOhsNgJppZPLH8yHz2FdB9z",
      "type": "spec/overlays/character_encoding/1.1",
      "attribute_character_encoding": {
        "age": "utf-8",
        "name": "utf-8"
      }
    },
    "conditional": {
      "d": "EHF7zB6Ajrxr9zCTg8SstIZE5YHWioG1bzWjzAuBJyaC",
      "capture_base": "EKxVMSYCnIoPUfZHsKf8OTOhsNgJppZPLH8yHz2FdB9z",
      "type": "spec/overlays/conditional/1.1",
      "attribute_conditions": {
        "age": "${0} ~= nil and ${0} ~= ''"
      },
      "attribute_dependencies": {
        "age": [
          "name"
        ]
      }
    },
    "conformance": {
      "d": "ED1DI-zvv-Jm3v319ksQIFdKPCDkp4-d4gky2w2WPOk9",
      "capture_base": "EKxVMSYCnIoPUfZHsKf8OTOhsNgJppZPLH8yHz2FdB9z",
      "type": "spec/overlays/conformance/1.1",
      "attribute_conformance": {
        "age": "M",
        "name": "O"
      }
    },
    "entry": [
      {
        "d": "EJk_7snxZ24OebeEN3BFtjcZ_-nBGjMXt4YHX07yXP6x",
        "capture_base": "EKxVMSYCnIoPUfZHsKf8OTOhsNgJppZPLH8yHz2FdB9z",
        "type": "spec/overlays/entry/1.1",
        "language": "eng",
        "attribute_entries": {
          "age": {
            "a": "Option A",
            "b": "Option B"
          },
          "name": {
            "a": "Option A",
            "b": "Option B"
          }
        }
      },
      {
        "d": "EKO-fGzwp7omXv8RJ4WMSoGI7e0DKHklbXbawvau3imH",
        "capture_base": "EKxVMSYCnIoPUfZHsKf8OTOhsNgJppZPLH8yHz2FdB9z",
        "type": "spec/overlays/entry/1.1",
        "language": "pol",
        "attribute_entries": {
          "name": {
            "a": "Opcja A",
            "b": "Opcja B"
          }
        }
      }
    ],
    "entry_code": {
      "d": "EHXbVeI2onaOhmu2D-F63PXRPx2V0iHw71La_OCCzH2j",
      "capture_base": "EKxVMSYCnIoPUfZHsKf8OTOhsNgJppZPLH8yHz2FdB9z",
      "type": "spec/overlays/entry_code/1.1",
      "attribute_entry_codes": {
        "age": [
          "a",
          "b"
        ],
        "name": [
          "a",
          "b"
        ]
      }
    },
    "format": {
      "d": "EOJ-m-j3ieqptwpcUo_rztya33WODAqVqykuywW2PEft",
      "capture_base": "EKxVMSYCnIoPUfZHsKf8OTOhsNgJppZPLH8yHz2FdB9z",
      "type": "spec/overlays/format/1.1",
      "attribute_formats": {
        "age": "^[a-zA-Z]*$",
        "name": "^[a-zA-Z]*$"
      }
    },
    "information": [
      {
        "d": "EHliDSsOvIRV0Wfm7-O8Gyo514BvoCg_QD14fVQCHIha",
        "capture_base": "EKxVMSYCnIoPUfZHsKf8OTOhsNgJppZPLH8yHz2FdB9z",
        "type": "spec/overlays/information/1.1",
        "language": "eng",
        "attribute_information": {
          "age": "age information",
          "name": "name information"
        }
      }
    ],
    "label": [
      {
        "d": "EIaG22w9wI1Hz5KnDkbVfnBXRdUUeLiH1pK8fr33RnBf",
        "capture_base": "EKxVMSYCnIoPUfZHsKf8OTOhsNgJppZPLH8yHz2FdB9z",
        "type": "spec/overlays/label/1.1",
        "language": "eng",
        "attribute_categories": [],
        "attribute_labels": {
          "age": "Age",
          "name": "Name"
        },
        "category_labels": {}
      }
    ],
    "meta": [
      {
        "d": "EJsJpRSMmQcLCBysWWBGQIdBdXEBEem7i0ZJ8ShxvQ5l",
        "capture_base": "EKxVMSYCnIoPUfZHsKf8OTOhsNgJppZPLH8yHz2FdB9z",
        "type": "spec/overlays/meta/1.1",
        "language": "eng",
        "description": "Test case OCA",
        "name": "Test"
      }
    ],
    "unit": {
      "d": "EMXt63BvnAvX-ESmFLjNpEMElog-DDOd8xl4iL3QEZMA",
      "capture_base": "EKxVMSYCnIoPUfZHsKf8OTOhsNgJppZPLH8yHz2FdB9z",
      "type": "spec/overlays/unit/1.1",
      "attribute_unit": {
        "age": "kg",
        "name": "kg"
      }
    }
  }
}"#
    } else {
        r#"{
  "d": "EMVt_-xNfr5DbxqNklc5AvgOtEwZpMUwROWFn18s_Xwk",
  "capture_base": {
    "d": "EKxVMSYCnIoPUfZHsKf8OTOhsNgJppZPLH8yHz2FdB9z",
    "type": "spec/capture_base/1.1",
    "attributes": {
      "age": "Numeric",
      "name": "Text"
    }
  },
  "overlays": {
    "cardinality": {
      "d": "EJFjpi67XgNXHlDilb-UoyFcZcf9m4jnqRPiQnx4vUAA",
      "capture_base": "EKxVMSYCnIoPUfZHsKf8OTOhsNgJppZPLH8yHz2FdB9z",
      "type": "spec/overlays/cardinality/1.1",
      "attribute_cardinality": {
        "age": "2",
        "name": "1"
      }
    },
    "character_encoding": {
      "d": "EGcXsw3LaRTlmnCVbxLYI2xB7WMkig2qI8pW5P98W0Mz",
      "capture_base": "EKxVMSYCnIoPUfZHsKf8OTOhsNgJppZPLH8yHz2FdB9z",
      "type": "spec/overlays/character_encoding/1.1",
      "attribute_character_encoding": {
        "age": "utf-8",
        "name": "utf-8"
      }
    },
    "conditional": {
      "d": "EHF7zB6Ajrxr9zCTg8SstIZE5YHWioG1bzWjzAuBJyaC",
      "capture_base": "EKxVMSYCnIoPUfZHsKf8OTOhsNgJppZPLH8yHz2FdB9z",
      "type": "spec/overlays/conditional/1.1",
      "attribute_conditions": {
        "age": "${0} ~= nil and ${0} ~= ''"
      },
      "attribute_dependencies": {
        "age": [
          "name"
        ]
      }
    },
    "conformance": {
      "d": "ED1DI-zvv-Jm3v319ksQIFdKPCDkp4-d4gky2w2WPOk9",
      "capture_base": "EKxVMSYCnIoPUfZHsKf8OTOhsNgJppZPLH8yHz2FdB9z",
      "type": "spec/overlays/conformance/1.1",
      "attribute_conformance": {
        "age": "M",
        "name": "O"
      }
    },
    "entry": [
      {
        "d": "EJk_7snxZ24OebeEN3BFtjcZ_-nBGjMXt4YHX07yXP6x",
        "capture_base": "EKxVMSYCnIoPUfZHsKf8OTOhsNgJppZPLH8yHz2FdB9z",
        "type": "spec/overlays/entry/1.1",
        "language": "eng",
        "attribute_entries": {
          "age": {
            "a": "Option A",
            "b": "Option B"
          },
          "name": {
            "a": "Option A",
            "b": "Option B"
          }
        }
      },
      {
        "d": "EKO-fGzwp7omXv8RJ4WMSoGI7e0DKHklbXbawvau3imH",
        "capture_base": "EKxVMSYCnIoPUfZHsKf8OTOhsNgJppZPLH8yHz2FdB9z",
        "type": "spec/overlays/entry/1.1",
        "language": "pol",
        "attribute_entries": {
          "name": {
            "a": "Opcja A",
            "b": "Opcja B"
          }
        }
      }
    ],
    "entry_code": {
      "d": "EHXbVeI2onaOhmu2D-F63PXRPx2V0iHw71La_OCCzH2j",
      "capture_base": "EKxVMSYCnIoPUfZHsKf8OTOhsNgJppZPLH8yHz2FdB9z",
      "type": "spec/overlays/entry_code/1.1",
      "attribute_entry_codes": {
        "age": [
          "a",
          "b"
        ],
        "name": [
          "a",
          "b"
        ]
      }
    },
    "information": [
      {
        "d": "EHliDSsOvIRV0Wfm7-O8Gyo514BvoCg_QD14fVQCHIha",
        "capture_base": "EKxVMSYCnIoPUfZHsKf8OTOhsNgJppZPLH8yHz2FdB9z",
        "type": "spec/overlays/information/1.1",
        "language": "eng",
        "attribute_information": {
          "age": "age information",
          "name": "name information"
        }
      }
    ],
    "label": [
      {
        "d": "EIaG22w9wI1Hz5KnDkbVfnBXRdUUeLiH1pK8fr33RnBf",
        "capture_base": "EKxVMSYCnIoPUfZHsKf8OTOhsNgJppZPLH8yHz2FdB9z",
        "type": "spec/overlays/label/1.1",
        "language": "eng",
        "attribute_categories": [],
        "attribute_labels": {
          "age": "Age",
          "name": "Name"
        },
        "category_labels": {}
      }
    ],
    "meta": [
      {
        "d": "EJsJpRSMmQcLCBysWWBGQIdBdXEBEem7i0ZJ8ShxvQ5l",
        "capture_base": "EKxVMSYCnIoPUfZHsKf8OTOhsNgJppZPLH8yHz2FdB9z",
        "type": "spec/overlays/meta/1.1",
        "language": "eng",
        "description": "Test case OCA",
        "name": "Test"
      }
    ],
    "unit": {
      "d": "EMXt63BvnAvX-ESmFLjNpEMElog-DDOd8xl4iL3QEZMA",
      "capture_base": "EKxVMSYCnIoPUfZHsKf8OTOhsNgJppZPLH8yHz2FdB9z",
      "type": "spec/overlays/unit/1.1",
      "attribute_unit": {
        "age": "kg",
        "name": "kg"
      }
    }
  }
}"#
    };

    assert_eq!(serialized_bundle, expected);
}
