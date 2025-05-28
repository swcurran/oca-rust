use isolang::Language;
use oca_bundle::{
    state::{
        attribute::{Attribute, AttributeType},
        encoding::Encoding,
        entries::EntriesElement,
        entry_codes::EntryCodes as EntryCodesValue,
        oca::{
            overlay::Overlay, OCABox
        },
    },
    Encode as EncodeBundle, HashFunctionCode, SerializationFormats,
};

use cascade::cascade;
use maplit::hashmap;
use serde_value::Value;

#[test]
fn create_oca() {
    let mut oca = cascade! {
        OCABox::new();
        // ..add_meta(Language::Eng, "name".to_string(), "Test".to_string());
        // ..add_meta(Language::Eng, "description".to_string(), "Test case OCA".to_string());
    };

    let mut attribute = cascade! {
        Attribute::new("name".to_string());
        ..set_attribute_type(oca_ast::ast::NestedAttrType::Value(AttributeType::Text));
        // ..set_encoding(Encoding::Utf8);
        // ..set_cardinality("1".to_string());
        // ..set_conformance("O".to_string());
        // ..set_label(isolang::Language::Eng, "Name".to_string());
        // ..set_entry_codes(EntryCodesValue::Array(vec!["a".to_string(), "b".to_string()]));
        // ..set_entry(isolang::Language::Pol, EntriesElement::Object(hashmap! {
        //     "a".to_string() => "Opcja A".to_string(),
        //     "b".to_string() => "Opcja B".to_string(),
        // }));
        // ..set_entry(isolang::Language::Eng, EntriesElement::Object(hashmap! {
        //     "a".to_string() => "Option A".to_string(),
        //     "b".to_string() => "Option B".to_string(),
        // }));
        // ..set_unit("kg".to_string());
    };
//    attribute.set_format("^[a-zA-Z]*$".to_string());

    oca.add_attribute(attribute);

    let mut attribute_2 = cascade! {
        Attribute::new("age".to_string());
        ..set_attribute_type(oca_ast::ast::NestedAttrType::Value(AttributeType::Numeric));
        // ..set_encoding(Encoding::Utf8);
        // ..set_cardinality("2".to_string());
        // ..set_conformance("M".to_string());
        // ..set_label(isolang::Language::Eng, "Age".to_string());
        // ..set_entry_codes(EntryCodesValue::Array(vec!["a".to_string(), "b".to_string()]));
        // ..set_entry(isolang::Language::Eng, EntriesElement::Object(hashmap! {
        //     "a".to_string() => "Option A".to_string(),
        //     "b".to_string() => "Option B".to_string(),
        // }));
        // ..set_unit("kg".to_string());
    };
 //   attribute_2.set_format("^[a-zA-Z]*$".to_string());

    oca.add_attribute(attribute_2);

    let oca_bundle = oca.generate_bundle();
    assert_eq!(oca_bundle.said, oca.generate_bundle().said);

    let code = HashFunctionCode::Blake3_256;
    let format = SerializationFormats::JSON;

    let oca_bundle_encoded = oca_bundle.encode(&code, &format).unwrap();
    let oca_bundle_version = String::from_utf8(oca_bundle_encoded[6..23].to_vec()).unwrap();
    let oca_bundle_json = String::from_utf8(oca_bundle_encoded).unwrap();

    // assert_eq!(oca_bundle_version, "OCAS20JSON0009c7_");

    assert_eq!(oca_bundle.capture_base.attributes.len(), 2);

    assert_eq!(oca_bundle.overlays.len(), 10);

    let sample = {
        r#"{
  "v": "OCAS20JSON0009c7_",
  "digest": "EJGdam92dtQt4xuycxsofie56L_IbMkOYjbSXR6ofM1M",
  "capture_base": {
    "digest": "ELWv7hk0Ek2XDTFaMEdYeOXwxs97YROpQ3lfBsY6lTLa",
    "type": "capture_base/2.0.0",
    "attributes": {
      "age": "Numeric",
      "name": "Text"
    }
  },
  "overlays": {
    "cardinality": {
      "digest": "EJzb8RLuTQGdFNqMaCb6A8T_xLnzUljStx04MGtNKUOy",
      "capture_base": "ELWv7hk0Ek2XDTFaMEdYeOXwxs97YROpQ3lfBsY6lTLa",
      "type": "overlay/cardinality/2.0.0",
      "attribute_cardinality": {
        "age": "2",
        "name": "1"
      }
    },
    "character_encoding": {
      "digest": "EAlGlIc5TOOwxl0WAq8mvw1RUTpU-azydYwxMWOJ0Xfp",
      "capture_base": "ELWv7hk0Ek2XDTFaMEdYeOXwxs97YROpQ3lfBsY6lTLa",
      "type": "overlay/character_encoding/2.0.0",
      "attribute_character_encoding": {
        "age": "utf-8",
        "name": "utf-8"
      }
    },
    "conformance": {
      "digest": "EFY4yw6VlaWkfsTnoecPLzcTQXoeMGIHJiRrsylU4zAe",
      "capture_base": "ELWv7hk0Ek2XDTFaMEdYeOXwxs97YROpQ3lfBsY6lTLa",
      "type": "overlay/conformance/2.0.0",
      "attribute_conformance": {
        "age": "M",
        "name": "O"
      }
    },
    "entry": [
      {
        "digest": "EGoCpJb3yribWgUoE9GDlrF0W1j90LKFyoHPv-gf-tjy",
        "capture_base": "ELWv7hk0Ek2XDTFaMEdYeOXwxs97YROpQ3lfBsY6lTLa",
        "type": "overlay/entry/2.0.0",
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
        "digest": "EMmNN95FiuEtEihd2RP95DulmxnACwaE2095HJM1nSnP",
        "capture_base": "ELWv7hk0Ek2XDTFaMEdYeOXwxs97YROpQ3lfBsY6lTLa",
        "type": "overlay/entry/2.0.0",
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
      "digest": "EBzneOmABK1DZoNMM9IY7aVaEFafUJ0wPQTwLGxdwzwB",
      "capture_base": "ELWv7hk0Ek2XDTFaMEdYeOXwxs97YROpQ3lfBsY6lTLa",
      "type": "overlay/entry_code/2.0.0",
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
      "digest": "EDxPeL07zBRW2TS3Dr_BUlNwrVR9RvMtRWqLJfTX4F9z",
      "capture_base": "ELWv7hk0Ek2XDTFaMEdYeOXwxs97YROpQ3lfBsY6lTLa",
      "type": "overlay/format/2.0.0",
      "attribute_formats": {
        "age": "^[a-zA-Z]*$",
        "name": "^[a-zA-Z]*$"
      }
    },
    "label": [
      {
        "digest": "EOBdWMsW1rd4gVhDEXnEzQhVTJKVsY_xmsR6l8p4b0o_",
        "capture_base": "ELWv7hk0Ek2XDTFaMEdYeOXwxs97YROpQ3lfBsY6lTLa",
        "type": "overlay/label/2.0.0",
        "language": "eng",
        "attribute_labels": {
          "age": "Age",
          "name": "Name"
        }
      }
    ],
    "meta": [
      {
        "digest": "EKfqq_oWk9IPS_e4fgAdybFlwFcpnoL_08HGM6V4B0cc",
        "capture_base": "ELWv7hk0Ek2XDTFaMEdYeOXwxs97YROpQ3lfBsY6lTLa",
        "type": "overlay/meta/2.0.0",
        "language": "eng",
        "description": "Test case OCA",
        "name": "Test"
      }
    ],
    "unit": {
      "digest": "EHiR16flGctWaLh4dWn3wLyJEm5kb7eUJ24mb6kXgvK6",
      "capture_base": "ELWv7hk0Ek2XDTFaMEdYeOXwxs97YROpQ3lfBsY6lTLa",
      "type": "overlay/unit/2.0.0",
      "attribute_unit": {
        "age": "kg",
        "name": "kg"
      }
    }
  }
}"#
    };

    let serialized_bundle: Value = serde_json::from_str(&oca_bundle_json).unwrap();
    let expected: Value = serde_json::from_str(sample).unwrap();
    assert_eq!(serialized_bundle, expected);
}
