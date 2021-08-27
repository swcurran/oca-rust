use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::{Result, Value};

#[derive(Serialize, Deserialize, Debug)]
pub struct SchemaBaseBranch {
    schema_base: SchemaBase,
}

#[derive(Serialize, Deserialize, Debug)]
struct SchemaBase {
    name: String,
    #[serde(rename = "type")]
    type_name: String,
    #[serde(rename = "@context")]
    context: String,
    description: String,
    classification: String,
    issued_by: String,
    attributes: HashMap<String, Value>,
}

pub fn parse() -> Result<SchemaBaseBranch> {
    let data = r#"
 {"schema_base":{"@context":"https://odca.tech/v1","name":"Presentation Status","type":"spec/schema_base/1.0","description":"","classification":"","issued_by":"","attributes":{"presentation_urn":"Text","verified":"Boolean"},"pii_attributes":[]},"overlays":[{"@context":"https://odca.tech/overlays/v1","type":"spec/overlay/label/1.0","issued_by":"","role":"","purpose":"","schema_base":"hl:4YjW5R27kqiCDX5Tq6d3kNXmbK7g9skt6jjYW5iVZbL1","language":"en_US","attr_labels":{"presentation_urn":"Presentation URN","verified":"Verified"},"attr_categories":["_cat-1_"],"cat_labels":{"_cat-1_":""},"cat_attributes":{"_cat-1_":["presentation_urn","verified"]}},{"@context":"https://odca.tech/overlays/v1","type":"spec/overlay/character_encoding/1.0","issued_by":"","role":"","purpose":"","schema_base":"hl:4YjW5R27kqiCDX5Tq6d3kNXmbK7g9skt6jjYW5iVZbL1","default_character_encoding":"utf-8","attr_character_encoding":{"presentation_urn":"utf-8","verified":"utf-8"}}]}
        "#;

    let v: SchemaBaseBranch = serde_json::from_str(data)?;

    Ok(v)
}
