use serde_json::Value;
use crate::parsers::common::*;
use crate::parsers::types::*;

pub fn map_position_data(value: &Box<Value>) -> Result<CetusPosition, anyhow::Error> {
    // Extract position fields from JSON value
    let position_value = match value.get("position") {
        Some(Value::Object(s)) => s,
        _ => return Err(anyhow::anyhow!("Expected Object for 'position', got {:?}", value.get("position"))),
    };

    let fields = position_value;

    println!("fields: {:?}", fields);

    let position_data = CetusPosition {
        id: extract_nested_id_from_json_fields(fields, "id")?,
        pool: extract_string_from_json_fields(fields, "pool")?,
        index: extract_number_from_json_fields(fields, "index")?,
        coin_type_a: TypeName {
            name: extract_nested_string_from_json_fields(fields, "coin_type_a", "name")?,
        },
        coin_type_b: TypeName {
            name: extract_nested_string_from_json_fields(fields, "coin_type_b", "name")?,
        },
        name: extract_string_from_json_fields(fields, "name")?,
        description: extract_string_from_json_fields(fields, "description")?,
        url: extract_string_from_json_fields(fields, "url")?,
        tick_lower_index: I32Wrapper {
            bits: extract_nested_string_from_json_fields(fields, "tick_lower_index", "bits")?,
        },
        tick_upper_index: I32Wrapper {
            bits: extract_nested_string_from_json_fields(fields, "tick_upper_index", "bits")?,
        },
        liquidity: extract_number_from_json_fields(fields, "liquidity")? as u128,
    };
    
    Ok(position_data)
}