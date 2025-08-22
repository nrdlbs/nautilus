use std::collections::BTreeMap;
use prost_types::Value;
use sui_sdk_types::StructTag;
use sui_sdk_types::TypeTag;
use crate::parsers::common::*;
use crate::parsers::types::*;
use std::str::FromStr;

pub fn map_pool_data(value: &Box<Value>) -> Result<CetusPoolData, anyhow::Error> {
    // Extract struct fields from prost Value
    let struct_value = match &value.kind {
        Some(prost_types::value::Kind::StructValue(s)) => s,
        _ => return Err(anyhow::anyhow!("Expected StructValue, got {:?}", value.kind)),
    };

    let fields = &struct_value.fields;

    let pool_data = CetusPoolData {
        coin_a: extract_string_or_number_from_fields(fields, "coin_a")?,
        coin_b: extract_string_or_number_from_fields(fields, "coin_b")?,
        current_sqrt_price: extract_string_or_number_from_fields(fields, "current_sqrt_price")?,
        current_tick_index: TickIndex { 
            bits: extract_nested_string_from_fields(fields, "current_tick_index", "bits")?,
        },
        fee_growth_global_a: extract_string_or_number_from_fields(fields, "fee_growth_global_a")?,
        fee_growth_global_b: extract_string_or_number_from_fields(fields, "fee_growth_global_b")?,
        fee_protocol_coin_a: extract_string_or_number_from_fields(fields, "fee_protocol_coin_a")?,
        fee_protocol_coin_b: extract_string_or_number_from_fields(fields, "fee_protocol_coin_b")?,
        fee_rate: extract_string_or_number_from_fields(fields, "fee_rate")?,
        id: extract_nested_id_from_fields(fields, "id")?,
        index: extract_string_or_number_from_fields(fields, "index")?,
        is_pause: extract_bool_from_fields(fields, "is_pause")?,
        liquidity: extract_string_or_number_from_fields(fields, "liquidity")?,
        position_manager: parse_position_manager_from_fields(fields)?,
        rewarder_manager: parse_rewarder_manager_from_fields(fields)?,
        tick_manager: parse_tick_manager_from_fields(fields)?,
        tick_spacing: extract_number_from_fields(fields, "tick_spacing")?,
        url: extract_string_from_fields(fields, "url")?,
    };

    println!("pool_data: {:?}", pool_data.rewarder_manager.rewarders.iter().map(|rewarder| TypeTag::Struct(Box::new(StructTag::from_str(&format!("0x{}", rewarder.reward_coin.name)).unwrap()))).collect::<Vec<_>>());
    
    Ok(pool_data)
}

fn parse_position_manager_from_fields(fields: &BTreeMap<String, Value>) -> Result<PositionManager, anyhow::Error> {
    let pm_value = fields.get("position_manager")
        .ok_or_else(|| anyhow::anyhow!("Field 'position_manager' not found"))?;
    
    let pm_struct = match &pm_value.kind {
        Some(prost_types::value::Kind::StructValue(s)) => s,
        _ => return Err(anyhow::anyhow!("Expected StructValue for position_manager")),
    };
    
    // Get the actual fields (might be wrapped in "fields")
    let pm_fields = if let Some(fields_value) = pm_struct.fields.get("fields") {
        match &fields_value.kind {
            Some(prost_types::value::Kind::StructValue(s)) => &s.fields,
            _ => return Err(anyhow::anyhow!("Expected StructValue for 'fields' in position_manager")),
        }
    } else {
        &pm_struct.fields
    };
    
    let positions_value = pm_fields.get("positions")
        .ok_or_else(|| anyhow::anyhow!("Field 'positions' not found in position_manager"))?;
    
    let positions_struct = match &positions_value.kind {
        Some(prost_types::value::Kind::StructValue(s)) => s,
        _ => return Err(anyhow::anyhow!("Expected StructValue for positions")),
    };
    
    // Get positions fields (might be wrapped in "fields")
    let positions_fields = if let Some(fields_value) = positions_struct.fields.get("fields") {
        match &fields_value.kind {
            Some(prost_types::value::Kind::StructValue(s)) => &s.fields,
            _ => return Err(anyhow::anyhow!("Expected StructValue for 'fields' in positions")),
        }
    } else {
        &positions_struct.fields
    };
    
    Ok(PositionManager {
        position_index: extract_string_or_number_from_fields(pm_fields, "position_index")?,
        positions: PositionsList {
            head: extract_string_from_fields(positions_fields, "head")?,
            id: extract_nested_id_from_fields(positions_fields, "id")?,
            size: extract_string_or_number_from_fields(positions_fields, "size")?,
            tail: extract_string_from_fields(positions_fields, "tail")?,
        },
        tick_spacing: extract_number_from_fields(pm_fields, "tick_spacing")?,
    })
}

fn parse_rewarder_manager_from_fields(fields: &BTreeMap<String, Value>) -> Result<RewarderManager, anyhow::Error> {
    let rm_value = fields.get("rewarder_manager")
        .ok_or_else(|| anyhow::anyhow!("Field 'rewarder_manager' not found"))?;
    
    let rm_struct = match &rm_value.kind {
        Some(prost_types::value::Kind::StructValue(s)) => s,
        _ => return Err(anyhow::anyhow!("Expected StructValue for rewarder_manager")),
    };
    
    // Get the actual fields (might be wrapped in "fields")
    let rm_fields = if let Some(fields_value) = rm_struct.fields.get("fields") {
        match &fields_value.kind {
            Some(prost_types::value::Kind::StructValue(s)) => &s.fields,
            _ => return Err(anyhow::anyhow!("Expected StructValue for 'fields' in rewarder_manager")),
        }
    } else {
        &rm_struct.fields
    };
    
    let rewarders_value = rm_fields.get("rewarders")
        .ok_or_else(|| anyhow::anyhow!("Field 'rewarders' not found in rewarder_manager"))?;
    
    let rewarders_list = match &rewarders_value.kind {
        Some(prost_types::value::Kind::ListValue(list)) => &list.values,
        _ => return Err(anyhow::anyhow!("Expected ListValue for rewarders")),
    };
    
    let mut rewarders = Vec::new();
    for rewarder_value in rewarders_list {
        let rewarder_struct = match &rewarder_value.kind {
            Some(prost_types::value::Kind::StructValue(s)) => s,
            _ => continue,
        };
        
        // Get rewarder fields (might be wrapped in "fields")
        let rewarder_fields = if let Some(fields_value) = rewarder_struct.fields.get("fields") {
            match &fields_value.kind {
                Some(prost_types::value::Kind::StructValue(s)) => &s.fields,
                _ => continue,
            }
        } else {
            &rewarder_struct.fields
        };
        
        let reward_coin_value = rewarder_fields.get("reward_coin")
            .ok_or_else(|| anyhow::anyhow!("Field 'reward_coin' not found in rewarder"))?;
        
        let reward_coin_struct = match &reward_coin_value.kind {
            Some(prost_types::value::Kind::StructValue(s)) => s,
            _ => return Err(anyhow::anyhow!("Expected StructValue for reward_coin")),
        };
        
        // Get reward_coin fields (might be wrapped in "fields")
        let reward_coin_fields = if let Some(fields_value) = reward_coin_struct.fields.get("fields") {
            match &fields_value.kind {
                Some(prost_types::value::Kind::StructValue(s)) => &s.fields,
                _ => return Err(anyhow::anyhow!("Expected StructValue for 'fields' in reward_coin")),
            }
        } else {
            &reward_coin_struct.fields
        };
        
        rewarders.push(Rewarder {
            emissions_per_second: extract_string_or_number_from_fields(rewarder_fields, "emissions_per_second")?,
            growth_global: extract_string_or_number_from_fields(rewarder_fields, "growth_global")?,
            reward_coin: RewardCoin {
                name: extract_string_from_fields(reward_coin_fields, "name")?,
            },
        });
    }
    
    Ok(RewarderManager {
        last_updated_time: extract_string_or_number_from_fields(rm_fields, "last_updated_time")?,
        points_growth_global: extract_string_or_number_from_fields(rm_fields, "points_growth_global")?,
        points_released: extract_string_or_number_from_fields(rm_fields, "points_released")?,
        rewarders,
    })
}

fn parse_tick_manager_from_fields(fields: &BTreeMap<String, Value>) -> Result<TickManager, anyhow::Error> {
    let tm_value = fields.get("tick_manager")
        .ok_or_else(|| anyhow::anyhow!("Field 'tick_manager' not found"))?;
    
    let tm_struct = match &tm_value.kind {
        Some(prost_types::value::Kind::StructValue(s)) => s,
        _ => return Err(anyhow::anyhow!("Expected StructValue for tick_manager")),
    };
    
    // Get the actual fields (might be wrapped in "fields")
    let tm_fields = if let Some(fields_value) = tm_struct.fields.get("fields") {
        match &fields_value.kind {
            Some(prost_types::value::Kind::StructValue(s)) => &s.fields,
            _ => return Err(anyhow::anyhow!("Expected StructValue for 'fields' in tick_manager")),
        }
    } else {
        &tm_struct.fields
    };
    
    let ticks_value = tm_fields.get("ticks")
        .ok_or_else(|| anyhow::anyhow!("Field 'ticks' not found in tick_manager"))?;
    
    let ticks_struct = match &ticks_value.kind {
        Some(prost_types::value::Kind::StructValue(s)) => s,
        _ => return Err(anyhow::anyhow!("Expected StructValue for ticks")),
    };
    
    // Get ticks fields (might be wrapped in "fields")
    let ticks_fields = if let Some(fields_value) = ticks_struct.fields.get("fields") {
        match &fields_value.kind {
            Some(prost_types::value::Kind::StructValue(s)) => &s.fields,
            _ => return Err(anyhow::anyhow!("Expected StructValue for 'fields' in ticks")),
        }
    } else {
        &ticks_struct.fields
    };
    
    let head_value = ticks_fields.get("head")
        .ok_or_else(|| anyhow::anyhow!("Field 'head' not found in ticks"))?;
    
    let head_list = match &head_value.kind {
        Some(prost_types::value::Kind::ListValue(list)) => &list.values,
        _ => return Err(anyhow::anyhow!("Expected ListValue for head")),
    };
    
    let mut head_ticks = Vec::new();
    for tick_value in head_list {
        let tick_struct = match &tick_value.kind {
            Some(prost_types::value::Kind::StructValue(s)) => s,
            _ => continue,
        };
        
        // Get tick fields (might be wrapped in "fields")
        let tick_fields = if let Some(fields_value) = tick_struct.fields.get("fields") {
            match &fields_value.kind {
                Some(prost_types::value::Kind::StructValue(s)) => &s.fields,
                _ => continue,
            }
        } else {
            &tick_struct.fields
        };
        
        head_ticks.push(TickOption {
            is_none: extract_bool_from_fields(tick_fields, "is_none")?,
            v: extract_string_or_number_from_fields(tick_fields, "v")?,
        });
    }
    
    let tail_value = ticks_fields.get("tail")
        .ok_or_else(|| anyhow::anyhow!("Field 'tail' not found in ticks"))?;
    
    let tail_struct = match &tail_value.kind {
        Some(prost_types::value::Kind::StructValue(s)) => s,
        _ => return Err(anyhow::anyhow!("Expected StructValue for tail")),
    };
    
    // Get tail fields (might be wrapped in "fields")
    let tail_fields = if let Some(fields_value) = tail_struct.fields.get("fields") {
        match &fields_value.kind {
            Some(prost_types::value::Kind::StructValue(s)) => &s.fields,
            _ => return Err(anyhow::anyhow!("Expected StructValue for 'fields' in tail")),
        }
    } else {
        &tail_struct.fields
    };
    
    let random_value = ticks_fields.get("random")
        .ok_or_else(|| anyhow::anyhow!("Field 'random' not found in ticks"))?;
    
    let random_struct = match &random_value.kind {
        Some(prost_types::value::Kind::StructValue(s)) => s,
        _ => return Err(anyhow::anyhow!("Expected StructValue for random")),
    };
    
    // Get random fields (might be wrapped in "fields")
    let random_fields = if let Some(fields_value) = random_struct.fields.get("fields") {
        match &fields_value.kind {
            Some(prost_types::value::Kind::StructValue(s)) => &s.fields,
            _ => return Err(anyhow::anyhow!("Expected StructValue for 'fields' in random")),
        }
    } else {
        &random_struct.fields
    };
    
    Ok(TickManager {
        tick_spacing: extract_number_from_fields(tm_fields, "tick_spacing")?,
        ticks: TicksData {
            head: head_ticks,
            id: extract_nested_id_from_fields(ticks_fields, "id")?,
            level: extract_string_or_number_from_fields(ticks_fields, "level")?,
            list_p: extract_string_or_number_from_fields(ticks_fields, "list_p")?,
            max_level: extract_string_or_number_from_fields(ticks_fields, "max_level")?,
            random: RandomSeed {
                seed: extract_string_or_number_from_fields(random_fields, "seed")?,
            },
            size: extract_string_or_number_from_fields(ticks_fields, "size")?,
            tail: TickOption {
                is_none: extract_bool_from_fields(tail_fields, "is_none")?,
                v: extract_string_or_number_from_fields(tail_fields, "v")?,
            },
        },
    })
}