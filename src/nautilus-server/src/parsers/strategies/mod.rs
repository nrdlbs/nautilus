use prost_types::Value;
use crate::parsers::types::*;

pub mod auto_rebalance;

pub fn try_match(json: Box<Value>) -> Result<Strategy, anyhow::Error> {
    // Try AutoRebalance strategy first
    if let Ok(auto_rebalance_data) = auto_rebalance::map_strategy_data(&json) {
        return Ok(Strategy::AutoRebalance(auto_rebalance_data));
    }
    
    // Add more strategy types here in the future
    // if let Ok(other_strategy_data) = other_strategy::map_strategy_data(&json) {
    //     return Ok(Strategy::OtherStrategy(other_strategy_data));
    // }

    // If no strategy matches, return error
    Err(anyhow::anyhow!("Unknown strategy type - could not match any known strategy patterns"))
}