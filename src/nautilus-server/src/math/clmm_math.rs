use bigdecimal::BigDecimal;
use num_bigint::BigInt;
use crate::math::tick_math;
use num_traits::ToPrimitive;
use crate::aggregator;
use sui_sdk_types::TypeTag;
use std::io::{Error, ErrorKind};

pub struct CalculateAmountsByLiquidityRequest {
    pub tick_lower_index: i32,
    pub tick_upper_index: i32,
    pub sqrt_price_x64: u128,
    pub liquidity: u128,
}

pub struct CalculateAddLiquidityOnlyCoinARequest {
    pub tick_lower_index: i32,
    pub tick_upper_index: i32,
    pub sqrt_price_x64: u128,
    pub coin_a_amount: u64,
    pub coin_a_type: TypeTag,
    pub coin_b_type: TypeTag,
    pub max_remain_rate: u64, // scale 1,000,000,000
}

pub struct CalculateAmountBWithOnlyCoinARequest {
    pub tick_lower_index: i32,
    pub tick_upper_index: i32,
    pub sqrt_price_x64: u128,
    pub coin_a_amount: u64, 
}

pub struct EstimateLiquidityAndAmountBFromAmountARequest {
    pub tick_lower_index: i32,
    pub tick_upper_index: i32,
    pub sqrt_price_x64: u128,
    pub coin_a_amount: u64,
}

impl CalculateAddLiquidityOnlyCoinARequest {
    pub fn new(tick_lower_index: i32, tick_upper_index: i32, sqrt_price_x64: u128, coin_a_amount: u64, coin_a_type: TypeTag, coin_b_type: TypeTag, max_remain_rate: u64) -> Self {
        Self { tick_lower_index, tick_upper_index, sqrt_price_x64, coin_a_amount, coin_a_type, coin_b_type, max_remain_rate }
    }
}

impl CalculateAmountsByLiquidityRequest {
    pub fn new(tick_lower_index: i32, tick_upper_index: i32, sqrt_price_x64: u128, liquidity: u128) -> Self {
        Self { tick_lower_index, tick_upper_index, sqrt_price_x64, liquidity }
    }
}

pub fn calculate_amounts_by_liquidity(request: CalculateAmountsByLiquidityRequest) -> Result<(u64, u64), Error> {
    let sqrt_price_lower = tick_math::get_sqrt_price_at_tick(request.tick_lower_index);
    let sqrt_price_upper = tick_math::get_sqrt_price_at_tick(request.tick_upper_index);

    if request.sqrt_price_x64 < sqrt_price_lower {
        let amount_a = BigInt::from(request.liquidity) * (BigInt::from(sqrt_price_upper) - BigInt::from(sqrt_price_lower)) * BigInt::from(2u128.pow(64)) / (BigInt::from(sqrt_price_lower) * BigInt::from(sqrt_price_upper));
        if amount_a.to_u64().is_none() {
            return Err(Error::new(ErrorKind::Other, "Amount A is too large"));
        }
        return Ok((amount_a.to_u64().unwrap(), 0));
    }

    if request.sqrt_price_x64 > sqrt_price_upper {
        let amount_b = BigInt::from(request.liquidity) * (BigInt::from(sqrt_price_upper) - BigInt::from(sqrt_price_lower)) / BigInt::from(2u128.pow(64));
        if amount_b.to_u64().is_none() {
            return Err(Error::new(ErrorKind::Other, "Amount B is too large"));
        }
        return Ok((0, amount_b.to_u64().unwrap()));
    }

    let amount_a = BigInt::from(request.liquidity) * (BigInt::from(sqrt_price_upper) - BigInt::from(request.sqrt_price_x64)) * BigInt::from(2u128.pow(64)) / (BigInt::from(sqrt_price_upper) * BigInt::from(request.sqrt_price_x64));
    let amount_b = BigInt::from(request.liquidity) * (BigInt::from(request.sqrt_price_x64) - BigInt::from(sqrt_price_lower)) / BigInt::from(2u128.pow(64));
    if amount_a.to_u64().is_none() {
        return Err(Error::new(ErrorKind::Other, "Amount A is too large"));
    }
    if amount_b.to_u64().is_none() {
        return Err(Error::new(ErrorKind::Other, "Amount B is too large"));
    }
    return Ok((amount_a.to_u64().unwrap(), amount_b.to_u64().unwrap()));
}

pub fn get_amount_ratio(tick_lower: i32, tick_upper: i32, sqrt_price_x64: u128) -> (f64, f64) {
    let liquidity = est_liquidity_from_amount_a(EstimateLiquidityAndAmountBFromAmountARequest {
        tick_lower_index: tick_lower,
        tick_upper_index: tick_upper,
        sqrt_price_x64: sqrt_price_x64,
        coin_a_amount: 100000000,
    }).unwrap();
    let amounts = calculate_amounts_by_liquidity(CalculateAmountsByLiquidityRequest {
        tick_lower_index: tick_lower,
        tick_upper_index: tick_upper,
        sqrt_price_x64: sqrt_price_x64,
        liquidity: liquidity,
    }).unwrap();

    let curr_price = tick_math::sqrt_price_x64_to_price(sqrt_price_x64, 0, 0);
    let transform_amount_b = BigDecimal::from(amounts.0) * BigDecimal::from(curr_price.clone());
    let total_amount = transform_amount_b.clone() + BigDecimal::from(amounts.1);
    let ratio_a = transform_amount_b.clone() / total_amount.clone();
    let ratio_b = BigDecimal::from(amounts.1) / total_amount.clone();

    println!("liquidity: {}", liquidity);
    println!("amounts: {:?}", amounts);
    println!("curr_price: {}", curr_price);
    println!("transform_amount_b: {}", transform_amount_b);
    println!("total_amount: {}", total_amount);
    println!("ratio_a: {}", ratio_a);
    println!("ratio_b: {}", ratio_b);

    return (ratio_a.to_f64().unwrap(), ratio_b.to_f64().unwrap());
}

pub fn est_liquidity_from_amount_a(request: EstimateLiquidityAndAmountBFromAmountARequest) -> Result<u128, Error> {
    let sqrt_price_upper = tick_math::get_sqrt_price_at_tick(request.tick_upper_index);

    if request.sqrt_price_x64 > sqrt_price_upper {
        return Err(Error::new(ErrorKind::Other, "Can not estimate liquidity and amount B from amount A"));
    }

    let num = BigInt::from(request.coin_a_amount) * BigInt::from(sqrt_price_upper) * BigInt::from(request.sqrt_price_x64);
    let denom = BigInt::from(sqrt_price_upper) - BigInt::from(request.sqrt_price_x64);
    if denom.eq(&BigInt::from(0)) || num.eq(&BigInt::from(0)) {
        return Ok(0);
    }

    let liquidity = num / denom / BigInt::from(1u128 << 64);
    return Ok(liquidity.to_u128().unwrap());
}

pub async fn calculate_add_liquidity_only_coin_a_liquidity(request: CalculateAddLiquidityOnlyCoinARequest) -> (u128, Option<aggregator::cetus::RouteData>) {
    let cetus_aggregator = aggregator::cetus::CetusAggregator::new();
    let mark_price = cetus_aggregator.get_mark_price(request.coin_a_type.to_string(), request.coin_b_type.to_string(), request.coin_a_amount).await.unwrap();
    let (amount_a_ratio, amount_b_ratio) = get_amount_ratio(request.tick_lower_index, request.tick_upper_index, request.sqrt_price_x64);
    let mut best_swap_amount = request.coin_a_amount * (amount_a_ratio * 1000000000f64) as u64 / 1000000000; // floor
    let max_loop = 200;
    let mut low = 0;
    let mut high = request.coin_a_amount;
    let mut liquidity = 0;
    for i in 0..max_loop {
        best_swap_amount = if i == 0 { best_swap_amount } else { (low + high) / 2 };
        let receive_amount = (best_swap_amount as u128 * mark_price / 1000000000u128) as u64;
        let rem_a = request.coin_a_amount - best_swap_amount;
        let rem_b = receive_amount;
        let est_liquidity = est_liquidity_from_amount_a(EstimateLiquidityAndAmountBFromAmountARequest {
            tick_lower_index: request.tick_lower_index,
            tick_upper_index: request.tick_upper_index,
            sqrt_price_x64: request.sqrt_price_x64,
            coin_a_amount: rem_a,
        }).unwrap();
        let amounts = calculate_amounts_by_liquidity(CalculateAmountsByLiquidityRequest {
            tick_lower_index: request.tick_lower_index,
            tick_upper_index: request.tick_upper_index,
            sqrt_price_x64: request.sqrt_price_x64,
            liquidity: est_liquidity,
        }).unwrap();
        println!("liquidity: {}", est_liquidity);
        println!("amounts: {:?}", amounts);
        println!("rem_a: {}", rem_a);
        println!("rem_b: {}", rem_b);
        println!("best_swap_amount: {}", best_swap_amount);
        if rem_a >= amounts.0 && rem_b >= amounts.1 {
            let not_add_amount_a = rem_a - amounts.0;
            let not_add_amount_b = rem_b - amounts.1;
            let max_remain_amount_a = rem_a * request.max_remain_rate / 1000000000;
            let max_remain_amount_b = rem_b * request.max_remain_rate / 1000000000;
            println!("not_add_amount_a: {}", not_add_amount_a);
            println!("not_add_amount_b: {}", not_add_amount_b);
            println!("max_remain_amount_a: {}", max_remain_amount_a);
            println!("max_remain_amount_b: {}", max_remain_amount_b);
            if not_add_amount_a > max_remain_amount_a || not_add_amount_b > max_remain_amount_b {
                high = best_swap_amount - 1;
            } else {
                liquidity = est_liquidity;
                break;
            }
        } else {
            low = best_swap_amount + 1;
        }

        if low > high {
            break;
        }
    };

    let route = cetus_aggregator.swap_by_amount_in(aggregator::cetus::SwapByAmountInRequest {
        from: request.coin_a_type.to_string(),
        to: request.coin_b_type.to_string(),
        amount_in: best_swap_amount,
    }).await.unwrap();

    return (liquidity as u128, route.data);
}

mod test {
    use super::*;

    #[test]
    fn test_calculate_amounts_by_liquidity() {
        let request = CalculateAmountsByLiquidityRequest::new(51240, 59760, 313942532289648482348, 1000000000000000000);
        let (amount_a, amount_b) = calculate_amounts_by_liquidity(request).unwrap();
        println!("amount_a: {}", amount_a);
        println!("amount_b: {}", amount_b);
        assert_eq!(amount_a, 1000000000000000000);
        assert_eq!(amount_b, 0);
    }
}