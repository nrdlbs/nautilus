use serde_json::Value;
use crate::parsers::types::*;

pub mod cetus;

pub fn try_match(json: Box<Value>) -> Result<Position, anyhow::Error> {
    // Try Cetus position first
    if let Ok(cetus_position_data) = cetus::map_position_data(&json) {
        return Ok(Position::Cetus(cetus_position_data));
    }
    
    // Add more DEX position types here in the future
    // if let Ok(other_dex_position_data) = other_dex::map_position_data(&json) {
    //     return Ok(Position::OtherDex(other_dex_position_data));
    // }

    // If no position type matches, return error
    Err(anyhow::anyhow!("Unknown position type - could not match any known DEX position patterns"))
}