use std::collections::BTreeMap;
use prost_types::Value;
use serde_json::{Value as JsonValue, Map};

// Helper functions to extract values from prost_types::Value fields
pub fn extract_string_from_fields(fields: &BTreeMap<String, Value>, key: &str) -> Result<String, anyhow::Error> {
    let value = fields.get(key)
        .ok_or_else(|| anyhow::anyhow!("Field '{}' not found", key))?;
    
    match &value.kind {
        Some(prost_types::value::Kind::StringValue(s)) => Ok(s.clone()),
        _ => Err(anyhow::anyhow!("Expected StringValue for field '{}', got {:?}", key, value.kind)),
    }
}

pub fn extract_bool_from_fields(fields: &BTreeMap<String, Value>, key: &str) -> Result<bool, anyhow::Error> {
    let value = fields.get(key)
        .ok_or_else(|| anyhow::anyhow!("Field '{}' not found", key))?;
    
    match &value.kind {
        Some(prost_types::value::Kind::BoolValue(b)) => Ok(*b),
        _ => Err(anyhow::anyhow!("Expected BoolValue for field '{}', got {:?}", key, value.kind)),
    }
}

pub fn extract_number_from_fields(fields: &BTreeMap<String, Value>, key: &str) -> Result<u64, anyhow::Error> {
    let value = fields.get(key)
        .ok_or_else(|| anyhow::anyhow!("Field '{}' not found", key))?;
    
    match &value.kind {
        Some(prost_types::value::Kind::NumberValue(n)) => Ok(*n as u64),
        Some(prost_types::value::Kind::StringValue(s)) => {
            s.parse::<u64>().map_err(|e| anyhow::anyhow!("Failed to parse string '{}' as u64: {}", s, e))
        },
        _ => Err(anyhow::anyhow!("Expected NumberValue or StringValue for field '{}', got {:?}", key, value.kind)),
    }
}

pub fn extract_nested_string_from_fields(fields: &BTreeMap<String, Value>, parent_key: &str, child_key: &str) -> Result<String, anyhow::Error> {
    let parent_value = fields.get(parent_key)
        .ok_or_else(|| anyhow::anyhow!("Field '{}' not found", parent_key))?;
    
    let parent_struct = match &parent_value.kind {
        Some(prost_types::value::Kind::StructValue(s)) => s,
        _ => return Err(anyhow::anyhow!("Expected StructValue for field '{}', got {:?}", parent_key, parent_value.kind)),
    };
    
    // Check if there's a "fields" wrapper
    if let Some(fields_value) = parent_struct.fields.get("fields") {
        let fields_struct = match &fields_value.kind {
            Some(prost_types::value::Kind::StructValue(s)) => s,
            _ => return Err(anyhow::anyhow!("Expected StructValue for 'fields' in '{}', got {:?}", parent_key, fields_value.kind)),
        };
        extract_string_or_number_from_fields(&fields_struct.fields, child_key)
    } else {
        extract_string_or_number_from_fields(&parent_struct.fields, child_key)
    }
}

pub fn extract_string_or_number_from_fields(fields: &BTreeMap<String, Value>, key: &str) -> Result<String, anyhow::Error> {
    let value = fields.get(key)
        .ok_or_else(|| anyhow::anyhow!("Field '{}' not found", key))?;
    
    match &value.kind {
        Some(prost_types::value::Kind::StringValue(s)) => Ok(s.clone()),
        Some(prost_types::value::Kind::NumberValue(n)) => Ok(n.to_string()),
        _ => Err(anyhow::anyhow!("Expected StringValue or NumberValue for field '{}', got {:?}", key, value.kind)),
    }
}

pub fn extract_nested_id_from_fields(fields: &BTreeMap<String, Value>, key: &str) -> Result<String, anyhow::Error> {
    let value = fields.get(key)
        .ok_or_else(|| anyhow::anyhow!("Field '{}' not found", key))?;
    
    match &value.kind {
        Some(prost_types::value::Kind::StringValue(s)) => Ok(s.clone()),
        Some(prost_types::value::Kind::StructValue(s)) => {
            // Try to extract "id" field from nested structure
            if let Some(id_value) = s.fields.get("id") {
                match &id_value.kind {
                    Some(prost_types::value::Kind::StringValue(id_str)) => Ok(id_str.clone()),
                    _ => Err(anyhow::anyhow!("Expected StringValue for nested id in field '{}'", key)),
                }
            } else {
                Err(anyhow::anyhow!("No 'id' field found in nested structure for field '{}'", key))
            }
        },
        _ => Err(anyhow::anyhow!("Expected StringValue or StructValue for field '{}', got {:?}", key, value.kind)),
    }
}

// Helper functions for serde_json::Value
pub fn extract_string_from_json_fields(fields: &Map<String, JsonValue>, key: &str) -> Result<String, anyhow::Error> {
    let value = fields.get(key)
        .ok_or_else(|| anyhow::anyhow!("Field '{}' not found", key))?;
    
    match value {
        JsonValue::String(s) => Ok(s.clone()),
        _ => Err(anyhow::anyhow!("Expected String for field '{}', got {:?}", key, value)),
    }
}

pub fn extract_number_from_json_fields(fields: &Map<String, JsonValue>, key: &str) -> Result<u64, anyhow::Error> {
    let value = fields.get(key)
        .ok_or_else(|| anyhow::anyhow!("Field '{}' not found", key))?;
    
    match value {
        JsonValue::Number(n) => Ok(n.as_u64().unwrap_or(0)),
        JsonValue::String(s) => {
            s.parse::<u64>().map_err(|e| anyhow::anyhow!("Failed to parse string '{}' as u64: {}", s, e))
        },
        _ => Err(anyhow::anyhow!("Expected Number or String for field '{}', got {:?}", key, value)),
    }
}

pub fn extract_nested_string_from_json_fields(fields: &Map<String, JsonValue>, parent_key: &str, child_key: &str) -> Result<String, anyhow::Error> {
    let parent_value = fields.get(parent_key)
        .ok_or_else(|| anyhow::anyhow!("Field '{}' not found", parent_key))?;
    
    let parent_obj = match parent_value {
        JsonValue::Object(obj) => obj,
        _ => return Err(anyhow::anyhow!("Expected Object for field '{}', got {:?}", parent_key, parent_value)),
    };
    
    // Handle both string and number values
    let value = parent_obj.get(child_key)
        .ok_or_else(|| anyhow::anyhow!("Field '{}' not found in '{}'", child_key, parent_key))?;
    
    match value {
        JsonValue::String(s) => Ok(s.clone()),
        JsonValue::Number(n) => Ok(n.to_string()),
        _ => Err(anyhow::anyhow!("Expected String or Number for field '{}' in '{}', got {:?}", child_key, parent_key, value)),
    }
}

pub fn extract_nested_id_from_json_fields(fields: &Map<String, JsonValue>, key: &str) -> Result<String, anyhow::Error> {
    let value = fields.get(key)
        .ok_or_else(|| anyhow::anyhow!("Field '{}' not found", key))?;
    
    match value {
        JsonValue::String(s) => Ok(s.clone()),
        JsonValue::Object(obj) => {
            if let Some(JsonValue::String(id_str)) = obj.get("id") {
                Ok(id_str.clone())
            } else {
                Err(anyhow::anyhow!("No 'id' field found in nested structure for field '{}'", key))
            }
        },
        _ => Err(anyhow::anyhow!("Expected String or Object for field '{}', got {:?}", key, value)),
    }
}