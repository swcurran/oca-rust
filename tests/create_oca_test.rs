use maplit::hashmap;
use oca_rust::state::{
    attribute::{Attribute, AttributeType, Entry},
    encoding::Encoding,
    oca::OCA,
    validator::Validator,
};

use oca_rust::controller::load_oca;

#[test]
fn create_oca() {
    let mut oca = OCA::new(Encoding::Utf8)
        .add_name(hashmap! {
            "en_EN".to_string() => "Driving Licence".to_string(),
            "pl_PL".to_string() => "Prawo Jazdy".to_string(),
        })
        .add_description(hashmap! {
            "en_EN".to_string() => "OCA representing driving licence".to_string(),
            "pl_PL".to_string() => "OCA reprezentująca prawo jazdy".to_string(),
        });

    let first_name_attr =
        Attribute::new("first_name".to_string(), AttributeType::Text).add_label(hashmap! {
            "en_EN".to_string() => "First name: ".to_string(),
            "pl_PL".to_string() => "Imię: ".to_string(),
        });

    let last_name_attr = Attribute::new(String::from("last_name"), AttributeType::Text)
        .set_pii()
        .add_label(hashmap! {
            "en_EN".to_string() => "Last name: ".to_string(),
            "pl_PL".to_string() => "Nazwisko: ".to_string(),
        });

    let gender_attr = Attribute::new(String::from("gender"), AttributeType::Text)
        .add_label(hashmap! {
            "en_EN".to_string() => "Gender: ".to_string(),
            "pl_PL".to_string() => "Płeć: ".to_string(),
        })
        .add_entries(vec![
            Entry::new(
                "male".to_string(),
                hashmap! {
                    "en_EN".to_string() => "Male".to_string(),
                    "pl_PL".to_string() => "Mężczyzna".to_string(),
                },
            ),
            Entry::new(
                "female".to_string(),
                hashmap! {
                    "en_EN".to_string() => "Female".to_string(),
                    "pl_PL".to_string() => "Kobieta".to_string(),
                },
            ),
        ]);

    oca = oca
        .add_attribute(first_name_attr)
        .add_attribute(last_name_attr)
        .add_attribute(gender_attr)
        .finalize();
    assert_eq!(oca.capture_base.attributes.len(), 3);
    assert_eq!(oca.capture_base.pii.len(), 1);

    let validator =
        Validator::new().enforce_translations(vec!["en_EN".to_string(), "pl_PL".to_string()]);
    let validation_result = validator.validate(&oca);
    assert!(validation_result.is_ok());

    let oca_json = serde_json::to_string_pretty(&serde_json::to_value(&oca).unwrap()).unwrap();
    let mut loaded_oca = load_oca(&mut oca_json.as_bytes()).unwrap();

    let birth_date_attr = Attribute::new(String::from("birth_date"), AttributeType::Date)
        .set_pii()
        .add_label(hashmap! {
            "en_EN".to_string() => "Birth date: ".to_string(),
            "pl_PL".to_string() => "Data urodzenia: ".to_string(),
        })
        .add_format("DD/MM/YYYY".to_string());

    loaded_oca = loaded_oca.add_attribute(birth_date_attr).finalize();

    assert_eq!(loaded_oca.capture_base.attributes.len(), 4);
    assert_eq!(loaded_oca.capture_base.pii.len(), 2);
}