use prost_types::Value;
use crate::parsers::types::*;

pub mod cetus;

pub fn try_match(json: Box<Value>) -> Result<Pool, anyhow::Error> {
    // Try Cetus pool first
    if let Ok(cetus_pool_data) = cetus::map_pool_data(&json) {
        return Ok(Pool::Cetus(cetus_pool_data));
    }
    
    // Add more DEX types here in the future
    // if let Ok(other_dex_pool_data) = other_dex::map_pool_data(&json) {
    //     return Ok(Pool::OtherDex(other_dex_pool_data));
    // }

    // If no pool type matches, return error
    Err(anyhow::anyhow!("Unknown pool type - could not match any known DEX pool patterns"))
}