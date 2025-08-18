use crate::math::tick_math;
use crate::math::tick_math::*;
use crate::parsers::common::*;
use crate::parsers::types::*;
use prost_types::Value;

pub fn map_strategy_data(value: &Box<Value>) -> Result<AutoRebalanceStrategy, anyhow::Error> {
    // Extract struct fields from prost Value
    let struct_value = match &value.kind {
        Some(prost_types::value::Kind::StructValue(s)) => s,
        _ => {
            return Err(anyhow::anyhow!(
                "Expected StructValue, got {:?}",
                value.kind
            ))
        }
    };

    let fields = &struct_value.fields;

    let strategy_data = AutoRebalanceStrategy {
        id: extract_nested_id_from_fields(fields, "id")?,
        version: extract_number_from_fields(fields, "version")?,
        owner: extract_string_from_fields(fields, "owner")?,
        position_registry_id: extract_number_from_fields(fields, "position_registry_id")?,
        description: extract_string_from_fields(fields, "description")?,
        lower_sqrt_price_change_threshold_bps: extract_number_from_fields(
            fields,
            "lower_sqrt_price_change_threshold_bps",
        )?,
        upper_sqrt_price_change_threshold_bps: extract_number_from_fields(
            fields,
            "upper_sqrt_price_change_threshold_bps",
        )?,
        lower_sqrt_price_change_threshold_direction: extract_bool_from_fields(
            fields,
            "lower_sqrt_price_change_threshold_direction",
        )?,
        upper_sqrt_price_change_threshold_direction: extract_bool_from_fields(
            fields,
            "upper_sqrt_price_change_threshold_direction",
        )?,
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

// public fun get_new_tick_range(
//     strategy: &AutoRebalanceStrategy,
//     current_sqrt_price: u128,
//     tick_spacing: u32,
//     tick_lower_index: I32,
//     tick_upper_index: I32,
// ): (I32, I32) {
//     let sqrt_price_lower = get_sqrt_price_at_tick(tick_lower_index);
//     let sqrt_price_upper = get_sqrt_price_at_tick(tick_upper_index);
//     // price = price * (1 +- price_change_threshold_bps / 10000)
//     // sqrt_price = sqrt_price * sqrt(1 +- price_change_threshold_bps / 10000)
//     let lower_sqrt_price_change =
//         sqrt_price_lower * (strategy.lower_sqrt_price_change_threshold_bps as u128) / 10000u128;
//     let upper_sqrt_price_change =
//         sqrt_price_upper * (strategy.upper_sqrt_price_change_threshold_bps as u128) / 10000u128;
//     let mut min_acceptable_sqrt_price = if (strategy.lower_sqrt_price_change_threshold_direction) {
//         sqrt_price_lower + lower_sqrt_price_change
//     } else {
//         sqrt_price_lower - lower_sqrt_price_change
//     };
//     let mut max_acceptable_sqrt_price = if (strategy.upper_sqrt_price_change_threshold_direction) {
//         sqrt_price_upper - upper_sqrt_price_change
//     } else {
//         sqrt_price_upper + upper_sqrt_price_change
//     };
//     if (min_acceptable_sqrt_price < MIN_SQRT_PRICE_X64) {
//         min_acceptable_sqrt_price = MIN_SQRT_PRICE_X64;
//     };
//     if (max_acceptable_sqrt_price > MAX_SQRT_PRICE_X64) {
//         max_acceptable_sqrt_price = MAX_SQRT_PRICE_X64;
//     };
//     if (
//         current_sqrt_price > min_acceptable_sqrt_price && current_sqrt_price < max_acceptable_sqrt_price
//     ) {
//         abort errors::error_invalid_sqrt_price()
//     };
//     let mut new_sqrt_price_lower =
//         current_sqrt_price - current_sqrt_price * (strategy.range_multiplier as u128) / 10000u128;
//     let mut new_sqrt_price_upper =
//         current_sqrt_price + current_sqrt_price * (strategy.range_multiplier as u128) / 10000u128;
//     if (new_sqrt_price_lower < MIN_SQRT_PRICE_X64) {
//         new_sqrt_price_lower = MIN_SQRT_PRICE_X64;
//     };
//     if (new_sqrt_price_upper > MAX_SQRT_PRICE_X64) {
//         new_sqrt_price_upper = MAX_SQRT_PRICE_X64;
//     };
//     let new_tick_lower_index = utils::round_tick_to_spacing(
//         utils::bound_tick(get_tick_at_sqrt_price(new_sqrt_price_lower)),
//         tick_spacing,
//     );
//     let new_tick_upper_index = utils::round_tick_to_spacing(
//         utils::bound_tick(get_tick_at_sqrt_price(new_sqrt_price_upper)),
//         tick_spacing,
//     );
//     (new_tick_lower_index, new_tick_upper_index)
// }

const MAX_SQRT_PRICE_X64: u128 = 79226673515401279992447579055;
const MIN_SQRT_PRICE_X64: u128 = 4295048016;

pub fn get_new_tick_range(
    current_sqrt_price: u128,
    current_tick_index: u32,
    position_lower_tick: i32,
    position_upper_tick: i32,
    lower_sqrt_price_change_threshold_bps: u64,
    upper_sqrt_price_change_threshold_bps: u64,
    lower_sqrt_price_change_threshold_direction: bool,
    upper_sqrt_price_change_threshold_direction: bool,
    range_multiplier_bps: u64,
    tick_spacing: u32,
) -> Result<(i32, i32), anyhow::Error> {
    println!("current_sqrt_price: {:?}", current_sqrt_price);
    println!("current_tick_index: {:?}", current_tick_index);
    println!("position_lower_tick: {:?}", position_lower_tick);
    println!("position_upper_tick: {:?}", position_upper_tick);
    println!(
        "lower_sqrt_price_change_threshold_bps: {:?}",
        lower_sqrt_price_change_threshold_bps
    );
    println!(
        "upper_sqrt_price_change_threshold_bps: {:?}",
        upper_sqrt_price_change_threshold_bps
    );
    println!(
        "lower_sqrt_price_change_threshold_direction: {:?}",
        lower_sqrt_price_change_threshold_direction
    );
    println!(
        "upper_sqrt_price_change_threshold_direction: {:?}",
        upper_sqrt_price_change_threshold_direction
    );
    println!("range_multiplier_bps: {:?}", range_multiplier_bps);
    println!("tick_spacing: {:?}", tick_spacing);

    let sqrt_price_lower = tick_math::get_sqrt_price_at_tick(position_lower_tick);
    let sqrt_price_upper = tick_math::get_sqrt_price_at_tick(position_upper_tick);
    let lower_sqrt_price_change =
        sqrt_price_lower * (lower_sqrt_price_change_threshold_bps as u128) / 10000u128;
    let upper_sqrt_price_change =
        sqrt_price_upper * (upper_sqrt_price_change_threshold_bps as u128) / 10000u128;
    let _min_acceptable_sqrt_price = if lower_sqrt_price_change_threshold_direction {
        sqrt_price_lower + lower_sqrt_price_change
    } else {
        sqrt_price_lower - lower_sqrt_price_change
    };
    let _max_acceptable_sqrt_price = if upper_sqrt_price_change_threshold_direction {
        sqrt_price_upper - upper_sqrt_price_change
    } else {
        sqrt_price_upper + upper_sqrt_price_change
    };
    println!(
        "_min_acceptable_sqrt_price: {:?}",
        _min_acceptable_sqrt_price
    );
    println!(
        "_max_acceptable_sqrt_price: {:?}",
        _max_acceptable_sqrt_price
    );
    if current_sqrt_price < _min_acceptable_sqrt_price
        || current_sqrt_price > _max_acceptable_sqrt_price
    {
        let mut new_sqrt_price_lower =
            current_sqrt_price - current_sqrt_price * (range_multiplier_bps as u128) / 10000u128;
        let mut new_sqrt_price_upper =
            current_sqrt_price + current_sqrt_price * (range_multiplier_bps as u128) / 10000u128;

        if new_sqrt_price_lower < MIN_SQRT_PRICE_X64 {
            new_sqrt_price_lower = MIN_SQRT_PRICE_X64;
        }
        if new_sqrt_price_upper > MAX_SQRT_PRICE_X64 {
            new_sqrt_price_upper = MAX_SQRT_PRICE_X64;
        }

        let new_tick_lower_index = tick_math::round_tick_to_spacing(
            tick_math::bound_tick(tick_math::get_tick_at_sqrt_price(new_sqrt_price_lower)),
            tick_spacing,
        );
        let new_tick_upper_index = tick_math::round_tick_to_spacing(
            tick_math::bound_tick(tick_math::get_tick_at_sqrt_price(new_sqrt_price_upper)),
            tick_spacing,
        );
        Ok((new_tick_lower_index, new_tick_upper_index))
    } else {
        return Err(anyhow::anyhow!("Invalid sqrt price"));
    }
}
