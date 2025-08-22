use serde_json::Value;
use crate::parsers::types::*;
use std::collections::HashMap;

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

pub fn map_position_balances_data(json: &Box<Value>) -> Result<BalancesBag, anyhow::Error> {
	// Navigate to the balances field in the nested structure with multiple fallbacks
	let balance_bag = json
		.get("balance_bag")
		.and_then(|f| f.get("balances"))
		.and_then(|bf| bf.get("contents"))
		.or_else(|| {
			json.get("fields")
				.and_then(|f| f.get("balances"))
				.and_then(|bf| bf.get("contents"))
		})
		.or_else(|| {
			json.get("value")
				.and_then(|v| v.get("fields"))
				.and_then(|f| f.get("balance_bag"))
				.and_then(|bb| bb.get("fields"))
				.and_then(|b| b.get("balances"))
				.and_then(|bf| bf.get("fields"))
				.and_then(|bff| bff.get("contents"))
		})
		.ok_or_else(|| anyhow::anyhow!("Could not find balances contents path"))?;

    println!("balance_bag: {:?}", balance_bag);

	let mut balances = HashMap::new();
	
	// Parse the contents array which contains balance entries
	if let Value::Array(contents) = balance_bag {
		for content in contents {
			// Each content item should have key (coin_type) and value (amount)
			if let Value::Object(content_obj) = content {
				// Try multiple shapes for coin_type path
				let coin_type_opt = content_obj
					.get("fields").and_then(|f| f.get("key")).and_then(|k| k.get("fields")).and_then(|kf| kf.get("name")).and_then(|n| n.as_str())
					.or_else(|| content_obj.get("fields").and_then(|f| f.get("key")).and_then(|k| k.get("name")).and_then(|n| n.as_str()))
					.or_else(|| content_obj.get("key").and_then(|k| k.get("fields")).and_then(|kf| kf.get("name")).and_then(|n| n.as_str()))
					.or_else(|| content_obj.get("key").and_then(|k| k.get("name")).and_then(|n| n.as_str()));

				let coin_type = coin_type_opt.ok_or_else(|| anyhow::anyhow!("Could not extract coin_type name"))?;
				
				// Try multiple shapes for amount path
				let amount_value_opt = content_obj
					.get("fields").and_then(|f| f.get("value"))
					.or_else(|| content_obj.get("value"));

				let amount_value = amount_value_opt.ok_or_else(|| anyhow::anyhow!("Could not find value field for amount"))?;
				
				let amount: u64 = if let Some(u) = amount_value.as_u64() {
					u
				} else if let Some(s) = amount_value.as_str() {
					 s.parse::<u64>().map_err(|_| anyhow::anyhow!("Invalid amount string: {}", s))?
				} else {
					return Err(anyhow::anyhow!("Amount is neither a number nor a string"));
				};
				
				balances.insert(coin_type.to_string(), amount);
			}
		}
	}

	Ok(BalancesBag { balances })
}