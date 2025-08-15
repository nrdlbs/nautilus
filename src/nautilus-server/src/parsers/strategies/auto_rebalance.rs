use prost_types::Value;
use crate::parsers::common::*;
use crate::parsers::types::*;

pub fn map_strategy_data(value: &Box<Value>) -> Result<AutoRebalanceStrategy, anyhow::Error> {
    // Extract struct fields from prost Value
    let struct_value = match &value.kind {
        Some(prost_types::value::Kind::StructValue(s)) => s,
        _ => return Err(anyhow::anyhow!("Expected StructValue, got {:?}", value.kind)),
    };

    let fields = &struct_value.fields;

    let strategy_data = AutoRebalanceStrategy {
        id: extract_nested_id_from_fields(fields, "id")?,
        version: extract_number_from_fields(fields, "version")?,
        owner: extract_string_from_fields(fields, "owner")?,
        position_registry_id: extract_number_from_fields(fields, "position_registry_id")?,
        description: extract_string_from_fields(fields, "description")?,
        lower_sqrt_price_change_threshold_bps: extract_number_from_fields(fields, "lower_sqrt_price_change_threshold_bps")?,
        upper_sqrt_price_change_threshold_bps: extract_number_from_fields(fields, "upper_sqrt_price_change_threshold_bps")?,
        lower_sqrt_price_change_threshold_direction: extract_bool_from_fields(fields, "lower_sqrt_price_change_threshold_direction")?,
        upper_sqrt_price_change_threshold_direction: extract_bool_from_fields(fields, "upper_sqrt_price_change_threshold_direction")?,
        rebalance_cooldown_secs: extract_number_from_fields(fields, "rebalance_cooldown_secs")?,
        range_multiplier: extract_number_from_fields(fields, "range_multiplier")?,
        rebalance_max_tick: I32Wrapper {
            bits: extract_nested_string_from_fields(fields, "rebalance_max_tick", "bits")?,
        },
        rebalance_min_tick: I32Wrapper {
            bits: extract_nested_string_from_fields(fields, "rebalance_min_tick", "bits")?,
        },
        rebalance_paused: extract_bool_from_fields(fields, "rebalance_paused")?,
        lp_slippage_tolerance_bps: extract_number_from_fields(fields, "lp_slippage_tolerance_bps")?,
        last_rebalance_timestamp: extract_number_from_fields(fields, "last_rebalance_timestamp")?,
    };
    
    Ok(strategy_data)
}


